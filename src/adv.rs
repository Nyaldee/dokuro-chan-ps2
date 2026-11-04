//! L'histoire (adv_main) : le script remplit des lots, joués un à un —
//! la scène se prépare (AD_AdvBthAfterProc), la page attend la fin des
//! transitions, son texte paraît lettre à lettre avec la voix, puis attend
//! ○ ; un choix ouvre ses réponses. Voir docs/moteur.md.

use std::sync::Arc;

use crate::album::{self, Album};
use crate::backlog::{self, Backlog};
use crate::cglib::Library;
use crate::bread::{BRead, Place};
use crate::data::Uni;
use crate::dialog::{Action, Windows};
use crate::font::{self, Atlas, Font, Glyph, Message};
use crate::engine::Button;
use crate::sys::Sys;
use crate::gfx::{Gfx, GREY};
use crate::map::{self, Map};
use crate::result;
use crate::minigame::{self, Session};
use crate::options::Settings;
use crate::readmode::{self, ReadMode};
use crate::savedata;
use crate::original::Iso;
use crate::pointer::{self, Pick, Zone};
use crate::scene::{Images, Pass, Scene};
use crate::stcm2::{self, Script};
use crate::story::{Batch, Entry, Flag, Kind, World};

/// Les fenêtres de l'histoire.
const MAINWIN: &str = "mainwin";
const SPEAKER: &str = "speaker";
/// Les réponses d'un choix (« select%d ») et leur curseur.
const SELECT: [&str; 10] = ["select0", "select1", "select2", "select3", "select4", "select5", "select6", "select7", "select8", "select9"];
const SELECT_CURSOR: &str = "select";
/// Les bandeaux de la carte : le commentaire du lieu, « 移動先選択 ».
const MAP_COMMENT: &str = "map";
const MAP_TITLE: &str = "map2";
/// Le menu du jeu.
const GAME_MENU: &str = "menu";
/// L'image du signe d'attente.
const ICON: &str = "win2";
/// Ce que précède chaque page (AD_ModeChangeProc2).
const PREFIX: &[u8] = b"#Speed[5]#Effect[0]#Scale[1]";
/// Le titre d'une partie avant le premier save_name : « なし »
/// (AD_GameInit, `0x365020`).
const NO_NAME: &[u8] = b"\x82\xc8\x82\xb5";
/// Les questions de l'exécutable : « 好感度確認を終了します よろしいですか？ »
/// à la fin du résultat de la journée ; « ドクロちゃんに好感度を割り振ります
/// よろしいですか？ » et ainsi pour chaque fille, à la répartition des points.
const RESULT_ASK: usize = 0x3653C0;
/// Le retour à une page de l'historique : sa question, et ce qui marque le
/// retour dans l'historique (« 巻き戻し »).
const REWIND_ASK: usize = 0x365220;
const REWIND_MARK: usize = 0x365268;
const SHARE_ASK: [usize; 5] = [0x365400, 0x365450, 0x3654A0, 0x3654F0, 0x365540];

/// La répartition des points (AD_Distribute*) : la case du curseur
/// (colonne, rangée ; la case (1, 0), celle de Sakura, ne se choisit pas),
/// et les points donnés (l'écran s'assombrit alors : cRam01cf64b2).
struct Share {
    cell: (i32, i32),
    done: bool,
}

/// La fenêtre du menu de jeu (centre, taille).
const MENU_WIN: [i32; 4] = [0x140, 0xDC, 0xB4, 0xAA];

/// La hauteur d'une entrée du menu de jeu (0x9d + 0x1e × k).
fn menu_y(k: usize) -> i32 {
    0x9D + 0x1E * k as i32
}

/// La fille d'une case (0 à 4 : la case de Sakura est sautée).
fn share_girl((col, row): (i32, i32)) -> i32 {
    let k = col + row * 3;
    if k > 0 { k - 1 } else { k }
}

/// Où en est l'histoire (cRam01da7310).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum State {
    /// Passer au lot suivant, ou relancer le script (1).
    Next,
    /// Le lot se prépare : la scène change (2).
    Change,
    /// Les transitions du lot (3).
    Wait,
    /// Le texte paraît (4).
    Print,
    /// La page attend ○ (6).
    Read,
    /// L'historique, ouvert depuis une page qui attend (AD_AdvSubProc,
    /// sous-état 1) ; la question du retour à une page, puis le noir.
    Log,
    LogAsk,
    LogRewind,
    /// Le menu du jeu (AD_AdvSubProc, sous-état 4), la question du retour à
    /// la chambre, puis le noir avant la chambre.
    Menu,
    MenuAsk,
    ToRoom,
    /// Les options, ouvertes depuis le menu du jeu (sous-état 5) : le jeu
    /// les mène, puis rouvre le menu.
    Options,
    /// La sauvegarde et le chargement (sous-états 7 et 8) : le noir (10
    /// images), puis l'écran de la carte mémoire, que mène le jeu ; puis
    /// les fenêtres reviennent (sous-état 6, 15 images) avant la page.
    CardIn(bool),
    Card,
    Back,
    /// Un choix (7).
    Choose,
    /// La carte (9) : le noir, puis la carte ; un lieu choisi, son effet
    /// (0x1e images), puis le noir.
    MapIn,
    Map,
    MapChosen,
    MapOut,
    /// Le résultat de la journée (0x0c) : le noir, l'écran, ○ et la
    /// question « 好感度確認を終了します よろしいですか？ », le noir.
    ResultIn,
    Result,
    ResultAsk,
    ResultOut,
    /// La répartition des points (0x0d) : le noir, l'écran, le choix d'une
    /// fille, sa question, le noir.
    DistributeIn,
    Distribute,
    DistributeAsk,
    DistributeOut,
    /// Un mini-jeu (0x0e) : le noir, puis la partie.
    MinigameIn,
    Minigame,
    /// Gagné : l'image bonus attend ○ (0x0f, 0x1e) ; puis le décor
    /// revient (0x28, 0x32).
    Bonus,
    MinigameOut,
    /// La fin du jeu (0x16) : le noir, 0x14 images ; l'image bonus de
    /// l'album, si elle vient d'être gagnée, attend ○ ; puis le retour.
    EndIn,
    EndBonus,
    EndOut,
    /// Une vidéo (état 8) : le noir ; la vidéo, confiée au jeu, qui
    /// rappelle `movie_done` ; le noir s'efface en 10 images.
    MovieIn,
    Away,
    MovieOut,
    /// L'histoire est finie.
    End,
}

/// Ce que l'histoire demande au jeu.
#[derive(Clone, Debug, PartialEq)]
pub enum Request {
    /// Le retour à la chambre de Sakura (le menu du jeu).
    Room,
    /// Les options (le menu du jeu).
    Options,
    /// L'écran de la carte mémoire pour sauvegarder, ou charger.
    Save,
    Load,
    Movie(Vec<u8>),
    /// La fin du jeu.
    GameEnd,
}

pub struct Adv {
    pub world: World,
    iso: Arc<Iso>,
    scripts: Option<Uni>,
    images: Images,
    pub scene: Scene,
    windows: Windows,
    state: State,
    /// Le lot joué, le compte de son attente et sa durée (sRam01da7566,
    /// sRam01da7568), le compte à rebours (sRam01da756a).
    current: usize,
    count: i32,
    total: i32,
    timer: i32,
    /// ○ a été appuyé pendant les transitions : elles vont plus vite.
    fast: bool,
    /// La sorte du lot joué et celle du précédent.
    kind: Kind,
    /// Le nom affiché dans la fenêtre du locuteur.
    speaker: Option<Vec<u8>>,
    /// Le signe d'attente (AD_AdvEnterTran) : son angle (−1 : caché).
    icon: i32,
    /// Ce qu'attend le joueur (la page, le choix, la carte : cRam01da7310),
    /// où reviennent l'historique, le menu du jeu, les fenêtres cachées.
    waiting: State,
    /// Les fenêtres cachées par ✕ (sous-état 2) ; celles qu'ont cachées le
    /// menu, l'historique ou ✕, à rouvrir, et le curseur du choix caché.
    hidden: bool,
    concealed: Vec<&'static str>,
    cursor_concealed: bool,
    /// Le menu du jeu : l'entrée choisie, la précédente, l'avancée du
    /// curseur de l'une à l'autre (sur 6 images), son apparition (sur 15).
    menu: (usize, usize, i32, i32),
    /// La question du retour à la chambre, et ses réponses.
    ask: (Vec<Glyph>, Vec<Glyph>, Vec<Glyph>),
    backlog: Backlog,
    /// Le choix : réponses (texte), la choisie, la plus longue (en octets).
    choices: Vec<Vec<u8>>,
    choice: usize,
    choice_len: i32,
    /// Une demande au jeu, à prendre.
    request: Option<Request>,
    /// La partie de mini-jeu en cours, le mini-jeu tout juste noté joué.
    session: Option<Session>,
    played: Option<i32>,
    /// La carte, quand elle est ouverte ; le résultat de la journée,
    /// quand il est affiché (le niveau et la jauge de chaque fille).
    map: Option<Map>,
    result: Option<[(i32, i32); 5]>,
    share: Option<Share>,
    /// Le point d'entrée joué et les drapeaux de l'histoire à son départ
    /// (AD_AdvSaveEachProc), pour une partie sauvegardée.
    at_entry: (Entry, Vec<Option<Flag>>),
    /// Ce que le joueur a déjà lu (gardé d'une partie à l'autre), la place
    /// de la page dans le script, la page est-elle déjà lue (cRam01da73a0),
    /// le nom du point d'entrée tel que le garde le jeu (0x1da732a). Le jeu
    /// copie ce qui a été lu avec ses données système à chaque attente (une
    /// page, un choix, la carte : AD_AdvSaveProc) : c'est cette copie qui
    /// est gardée.
    pub read: BRead,
    pub read_kept: BRead,
    /// La partie vient de la chambre (le scénario bonus : 0x1da7594 à 1) :
    /// sa fin y ramène, au lieu du titre.
    pub from_room: bool,
    /// Les images de l'album déjà vues, et leur copie (comme `read_kept`).
    pub album: Album,
    pub album_kept: Album,
    place: Place,
    page_read: bool,
    entry_name: [u8; 0x20],
    /// L'avance rapide et la lecture automatique.
    readmode: ReadMode,
}

impl Adv {
    pub fn new(iso: Arc<Iso>) -> Adv {
        Adv {
            world: World::default(),
            scripts: Uni::open(&iso, "UNION/SCRIPT.UNI"),
            images: Images::new(iso.clone()),
            iso,
            scene: Scene::default(),
            windows: Windows::default(),
            state: State::End,
            current: 0,
            count: 0,
            total: 0,
            timer: 0,
            fast: false,
            kind: Kind::Silent,
            speaker: None,
            icon: -1,
            waiting: State::Read,
            hidden: false,
            concealed: Vec::new(),
            cursor_concealed: false,
            menu: (0, 0, 6, 0),
            ask: Default::default(),
            backlog: Backlog::default(),
            choices: Vec::new(),
            choice: 0,
            choice_len: 0,
            request: None,
            session: None,
            played: None,
            map: None,
            result: None,
            share: None,
            at_entry: Default::default(),
            read: BRead::default(),
            read_kept: BRead::default(),
            from_room: false,
            album: Album::default(),
            album_kept: Album::default(),
            place: Place::default(),
            page_read: false,
            entry_name: [0; 0x20],
            readmode: ReadMode::default(),
        }
    }

    /// Exécute un point d'entrée d'un script (son état ne survit pas au
    /// passage, comme dans le jeu qui recharge le fichier).
    fn run(&mut self, entry: &Entry) -> bool {
        let Some(mut script) = self.scripts.as_ref().and_then(|u| u.read(&self.iso, entry.file as u32)).and_then(Script::parse) else { return false };
        let Some(at) = script.entry(&entry.name) else { return false };
        stcm2::run(&mut script, at, &mut self.world);
        true
    }

    /// AD_StScrirptProc (au démarrage du jeu, puis à chaque partie neuve) :
    /// « init » du script 0, puis les scripts de départ qu'il inscrit
    /// (personnages, décors, vidéos, drapeaux, bulles de la chambre). Les
    /// drapeaux spéciaux (les données système) gardent leur valeur.
    pub fn boot(&mut self, seed: i32) {
        let kept: Vec<(Vec<u8>, i16)> = self.world.flags.special.iter().flatten().map(|f| (f.name.clone(), f.value)).collect();
        self.world = World { random: seed, ..World::default() };
        self.run(&Entry { file: 0, name: b"init".to_vec() });
        for e in self.world.init_scripts.clone() {
            self.run(&e);
        }
        self.world.batches.clear();
        for (name, value) in kept {
            self.world.flags.set(&name, value, true);
        }
        self.album.register(&self.world);
    }

    /// Les fins vues : les six drapeaux spéciaux d'ending_check_flag_set.
    pub fn endings_seen(&self) -> [bool; 6] {
        std::array::from_fn(|k| self.world.ending_flags.iter().rev().find(|e| e.0 == k as i32).is_some_and(|e| self.world.flags.get(&e.1, true) > 0))
    }

    /// AD_EndcheckGpSet : l'image des vignettes des fins (ending_check_bg_set,
    /// un décor), chargée.
    pub fn ending_sheet(&mut self, gfx: &mut Gfx) -> Option<String> {
        let name = self.world.ending_bg.clone();
        self.images.bg(gfx, &self.world, &name)
    }

    /// AD_CgLibMain, état 0 : l'écran de l'album, ses images chargées ;
    /// l'image bonus de la troisième sorte, si elle vient d'être gagnée, est
    /// notée vue (et gardée tout de suite).
    pub fn open_album(&mut self, gfx: &mut Gfx, sys: &mut Sys) -> Library {
        let mut sheets: Vec<(Vec<u8>, String)> = Vec::new();
        for e in &self.world.cg {
            if !sheets.iter().any(|s| s.0 == e.sheet)
                && let Some(key) = self.images.image(gfx, &self.world, &e.sheet)
            {
                sheets.push((e.sheet.clone(), key));
            }
        }
        let bonus = self.album.complete_bonus(&self.world).and_then(|id| {
            let name = self.world.bgs.get(id as usize)?.0.clone();
            let key = self.images.bg(gfx, &self.world, &name)?;
            self.album.record(id, false);
            self.album_kept = self.album.clone();
            Some(key)
        });
        Library::new(&self.world, &self.album, |n| sheets.iter().find(|s| s.0 == n).map(|s| s.1.clone()), bonus, sys)
    }

    /// Une image de l'écran de l'album ; vrai quand il est refermé.
    pub fn step_album(&mut self, lib: &mut Library, sys: &mut Sys, font: &Font, atlas: &mut Atlas, gfx: &mut Gfx) -> bool {
        let (images, world) = (&mut self.images, &self.world);
        lib.step(sys, &self.album, font, atlas, gfx, |gfx, id| {
            let name = world.bgs.get(id as usize)?.0.clone();
            images.bg(gfx, world, &name)
        })
    }

    /// AD_MVPlay : l'album note la vidéo vue. Dans l'histoire, elle sera
    /// gardée à la prochaine attente ; ailleurs (le titre, l'album), tout de
    /// suite.
    pub fn record_movie(&mut self, name: &[u8], keep: bool) {
        if let Some(id) = album::movie_id(&self.world, name) {
            self.album.record(id, true);
            if keep {
                self.album_kept = self.album.clone();
            }
        }
    }

    /// AD_EndcheckGpgLibCgRecord : toutes les fins vues, l'album note leur
    /// image (gardée tout de suite : le jeu la copie à sa prochaine
    /// sauvegarde système).
    pub fn record_ending_sheet(&mut self) {
        if let Some(id) = album::bg_id(&self.world, &self.world.ending_bg) {
            self.album.record(id, false);
            self.album_kept = self.album.clone();
        }
    }

    /// AD_BackLogSubSaveProc, à l'attente d'une page ou d'un choix : la
    /// partie à cet instant devient le point de retour de la dernière page
    /// de l'historique (AD_RewindSave).
    fn keep_page(&mut self, sys: &Sys, waiting: State) {
        if self.backlog.pending() {
            self.waiting = waiting;
            let game = self.snapshot(sys);
            self.backlog.keep(game);
        }
    }

    /// AD_AdvSaveProc (les données système) : ce qui a été lu et vu est
    /// copié pour être gardé.
    fn keep(&mut self) {
        self.read_kept = self.read.clone();
        self.album_kept = self.album.clone();
    }

    /// Une partie neuve (AD_AdvPartStartProc) : l'histoire commence au
    /// script que désigne « init » (start_stcm, ou bonus_stcm pour le
    /// scénario bonus).
    pub fn new_game(&mut self, seed: i32, settings: &Settings, bonus: bool) {
        self.boot(seed);
        self.world.next = if bonus { self.world.bonus.clone() } else { self.world.start.clone() };
        self.from_room = bonus;
        self.world.save_name = NO_NAME.to_vec();
        self.start_windows(settings);
        self.readmode.stop(settings, -1);
        self.state = State::Next;
        self.current = 0;
    }

    /// AD_AdvMainWinMake : la fenêtre de texte et celle du locuteur,
    /// fermées, aux opacités des options.
    fn start_windows(&mut self, settings: &Settings) {
        self.windows = Windows::default();
        self.windows.create(MAINWIN, 0, 0x46, 2, Some(8), (0x19, 0x14), Vec::new());
        let [frame, back] = settings.window.map(i32::from);
        self.windows.act(MAINWIN, &[Action { dur: 0, kind: 2, x: 0x140, y: 0x16D, w: 0x25D, h: 0x78, frame, back, ..Action::READY }]);
        self.speaker = None;
    }

    /// Les tests : l'histoire reprend à un point d'entrée d'un script.
    pub fn jump(&mut self, file: i32, name: &[u8]) {
        self.world.next = Entry { file, name: name.to_vec() };
    }

    /// La partie de mini-jeu en cours (le jeu y fait ses clics).
    pub fn session(&mut self) -> Option<&mut Session> {
        self.session.as_mut()
    }

    /// Le mini-jeu tout juste joué dans l'histoire (AD_MiniGamePlayRecord).
    pub fn take_played(&mut self) -> Option<i32> {
        self.played.take()
    }

    pub fn take_request(&mut self) -> Option<Request> {
        self.request.take()
    }

    /// La vidéo est finie (état 8, sous-état 0x14) : le noir s'efface en 10
    /// images, la musique en cours repart du début (AD_BGMRestart).
    pub fn movie_done(&mut self, sys: &mut Sys) {
        if self.state == State::Away {
            sys.blackout.end(&mut sys.effects, 10);
            sys.restart_music(true);
            self.timer = 10;
            self.state = State::MovieOut;
        }
    }

    pub fn finished(&self) -> bool {
        self.state == State::End
    }

    fn batch(&self) -> Option<&Batch> {
        self.world.batches.get(self.current)
    }

    /// Une image (adv_main).
    pub fn step(&mut self, sys: &mut Sys, gfx: &mut Gfx, font: &Font, atlas: &mut Atlas) {
        match self.state {
            State::Next => self.next(),
            State::Change => self.change(sys, gfx, font, atlas),
            State::Wait => {
                // Les transitions ; ○ les accélère, l'avance rapide aussi :
                // vitesse de l'avance + 5 images par image.
                if self.count < self.total {
                    self.count += 1;
                    if self.fast || self.check(sys) == readmode::SKIP {
                        let n = sys.settings.skip_speed(false) + 5;
                        for _ in 0..n {
                            self.scene.step(&mut sys.effects, &mut sys.rumble);
                        }
                        self.count += n;
                    } else if sys.input.key(3) {
                        self.fast = true;
                    }
                } else {
                    self.show_text(sys, gfx, font, atlas);
                }
            }
            State::Print => {
                // L'avance rapide montre le texte d'un coup.
                if self.check(sys) == readmode::SKIP {
                    self.windows.skip_text(MAINWIN);
                    self.timer = 0;
                }
                if self.timer > 0 {
                    self.timer -= 1;
                    if sys.input.key(3) && self.timer > 2 && self.readmode.mode == readmode::NONE {
                        self.windows.skip_text(MAINWIN);
                        self.timer = 2;
                    }
                } else {
                    self.after_text(sys, gfx, font, atlas);
                }
            }
            State::Read => {
                if !self.sub_keys(sys, font, atlas, gfx) {
                    self.icon = (self.icon + 1) % 0x2D;
                    // ○ (que l'attente de l'avance ou de la lecture
                    // automatique retient) ; l'avance, son attente passée ;
                    // la lecture automatique, L2, ou son attente passée et la
                    // voix finie.
                    let check = self.check(sys);
                    let mut go = sys.menu_key(0x1F) || sys.lock <= 0 && (sys.click.is_some() || sys.input.tapped(Button::WheelDown));
                    if check == readmode::SKIP {
                        go |= sys.lock < 1;
                    } else if check == readmode::AUTO {
                        let voice = self.batch().is_some_and(|b| b.voice.is_some());
                        go |= sys.input.key(6) || sys.lock < 1 && (!voice || !sys.voice_playing());
                    }
                    if go {
                        sys.stop_voice();
                        self.icon = -1;
                        self.state = State::Next;
                    }
                }
            }
            State::Log => self.log(sys, font, atlas, gfx),
            State::LogAsk => match self.windows.answer(sys) {
                // はい : le plan 5 s'éclaircit, le noir tombe en 0xf images.
                Some(0) => {
                    sys.dim.end(&mut sys.effects, 0xF);
                    sys.blackout.start(&mut sys.effects, 0xF, 0x7F);
                    self.backlog.close();
                    self.timer = 0xF;
                    self.state = State::LogRewind;
                }
                Some(_) => {
                    sys.dim.end(&mut sys.effects, 0xF);
                    sys.lock = 0xF;
                    self.state = State::Log;
                }
                None => {}
            },
            State::LogRewind => {
                if self.countdown() {
                    self.rewind(sys, gfx, font, atlas);
                }
            }
            State::Menu => self.menu(sys),
            // La souris y choisit aussi la réponse survolée (elle était
            // ignorée ici : la question bloquait qui jouait à la souris).
            State::MenuAsk => match self.windows.answer(sys) {
                // はい : la chambre, depuis le noir.
                Some(0) => {
                    sys.dim.end(&mut sys.effects, 0xF);
                    sys.blackout.start(&mut sys.effects, 10, 0x7F);
                    self.timer = 10;
                    self.state = State::ToRoom;
                }
                Some(_) => {
                    sys.dim.end(&mut sys.effects, 0xF);
                    self.state = State::Menu;
                }
                None => {}
            },
            State::ToRoom => {
                if self.countdown() {
                    self.request = Some(Request::Room);
                    self.state = State::End;
                }
            }
            State::Choose => self.choose(sys, gfx, font, atlas),
            State::Options | State::Card => {}
            State::CardIn(save) => {
                if self.countdown() {
                    // L'écran de la carte mémoire a son propre noir ; les
                    // bruitages en boucle se taisent (AD_LoopSEPause).
                    sys.blackout.end(&mut sys.effects, 1);
                    sys.pause_loops();
                    self.request = Some(if save { Request::Save } else { Request::Load });
                    self.state = State::Card;
                }
            }
            State::Back => {
                if self.countdown() {
                    self.back_to_waiting();
                }
            }
            State::MinigameIn => {
                self.count += 1;
                if self.countdown() {
                    self.state = State::Minigame;
                }
            }
            State::MapIn => {
                if self.countdown() {
                    self.wait_map();
                }
            }
            State::Map => self.map(sys, gfx, font, atlas),
            State::MapChosen => {
                if self.countdown() {
                    sys.blackout.start(&mut sys.effects, 10, 0x7F);
                    self.timer = 10;
                    self.state = State::MapOut;
                }
            }
            State::MapOut => {
                if self.countdown() {
                    // AD_MapEnd ; le noir reste jusqu'à la page suivante.
                    for name in [MAP_COMMENT, MAP_TITLE] {
                        self.windows.act(name, &[Action { dur: 5, kind: 3, ..Action::READY }]);
                    }
                    self.map = None;
                    self.state = State::Next;
                }
            }
            State::ResultIn => {
                if self.countdown() {
                    // AD_ResultStart : les niveaux d'après les drapeaux.
                    self.result = Some(result::levels(&self.world));
                    sys.blackout.end(&mut sys.effects, 10);
                    self.state = State::Result;
                }
            }
            State::Result => {
                // AD_ResultProc : ○ (ou un clic) pose la question.
                if sys.menu_key(0x1F) || sys.click.is_some() {
                    self.ask(RESULT_ASK, (0xFA, 0x37), font, atlas, gfx);
                    self.state = State::ResultAsk;
                }
            }
            State::ResultAsk => match self.windows.answer(sys) {
                // はい : le noir en 10 images, 0xc images d'attente
                // (AD_CheckWinProc).
                Some(0) => {
                    sys.blackout.start(&mut sys.effects, 10, 0x7F);
                    self.timer = 0xC;
                    self.state = State::ResultOut;
                }
                Some(_) => self.state = State::Result,
                None => {}
            },
            State::ResultOut => {
                // AD_ResultEnd ; le noir reste jusqu'à la page suivante.
                if self.countdown() {
                    self.result = None;
                    self.state = State::Next;
                }
            }
            State::DistributeIn => {
                self.count += 1;
                if self.countdown() {
                    // État 0x0d, sous-état 0 : la partie est copiée, le noir
                    // s'efface.
                    self.keep();
                    sys.blackout.end(&mut sys.effects, 10);
                    self.state = State::Distribute;
                }
            }
            State::Distribute => self.distribute(sys, gfx, font, atlas),
            State::EndIn => {
                if self.countdown() {
                    // Sous-état 5 : la fenêtre de texte et les personnages
                    // s'en vont.
                    self.windows = Windows::default();
                    self.speaker = None;
                    self.scene.clear_charas();
                    self.timer = 0;
                    self.end_bonus(sys, gfx);
                }
            }
            State::EndBonus => {
                if sys.menu_key(0x1F) || sys.click.is_some() {
                    sys.lock = 0x14;
                    self.timer = 0x14;
                    self.end_bonus(sys, gfx);
                }
            }
            State::EndOut => {
                // Sous-état 0x28, puis la sauvegarde système (le portage écrit
                // la sienne) et le retour.
                if self.countdown() {
                    self.keep();
                    self.request = Some(Request::GameEnd);
                    self.state = State::End;
                }
            }
            State::DistributeAsk => match self.windows.answer(sys) {
                // はい : le noir en 0x14 images, puis AD_DistributeEnd.
                Some(0) => {
                    sys.blackout.start(&mut sys.effects, 0x14, 0x7F);
                    self.timer = 0x14;
                    self.state = State::DistributeOut;
                }
                Some(_) => self.state = State::Distribute,
                None => {}
            },
            State::DistributeOut => {
                if self.countdown() {
                    // AD_DistributeEnd : les points vont au drapeau de la
                    // fille ; l'écran s'assombrit sous le noir.
                    if let Some(s) = &mut self.share {
                        let girl = share_girl(s.cell);
                        if let Some((_, flag, _)) = self.world.results.iter().rev().find(|r| r.0 == girl).cloned() {
                            self.world.flags.add(&flag, self.world.distribute as i16, false);
                        }
                        s.done = true;
                    }
                    self.state = State::Next;
                }
            }
            State::Minigame => self.minigame(sys, gfx),
            State::Bonus => {
                if sys.input.key(0x1F) || sys.click.is_some() {
                    sys.stop_music();
                    sys.blackout.start(&mut sys.effects, 10, 0x7F);
                    self.timer = 10;
                    self.state = State::MinigameOut;
                }
            }
            State::MinigameOut => {
                if self.countdown() {
                    // AD_BGPrintChangeBk : le décor d'avant l'image bonus.
                    if self.world.won == 1 {
                        self.scene.restore_bg();
                    }
                    sys.blackout.end(&mut sys.effects, 10);
                    self.state = State::Next;
                }
            }
            State::MovieIn => {
                // Le passage fini, l'état 8 attend que la musique mise en
                // pause (AD_ModeChangeProc4) se soit tue (Snd_Strm_BgmCheck),
                // ce qui prend plusieurs images sur PS2 : le décor du lot a
                // le temps de changer sous le noir (deux images après le
                // passage). La pause du portage est immédiate : il attend que
                // ce changement soit fait, sans quoi il aurait lieu après la
                // vidéo, sous le noir qui s'efface.
                if self.timer >= 0 {
                    self.timer -= 1;
                }
                if self.timer < 0 && !self.scene.bg_changing() {
                    self.request = self.batch().map(|b| Request::Movie(b.movie.clone()));
                    self.state = State::Away;
                }
            }
            State::MovieOut => {
                if self.countdown() {
                    self.state = State::Next;
                }
            }
            State::Away | State::End => {}
        }
        self.scene.step(&mut sys.effects, &mut sys.rumble);
        self.windows.update();
        self.backlog.update();
        // AD_ReadModeProc, hors des sous-états, des mini-jeux et du résultat
        // de la journée ; le signe (AD_ReadModeTran) hors des sous-états.
        let free = !self.sub_state();
        let active = free && !matches!(self.state, State::MinigameIn | State::Minigame | State::Bonus | State::MinigameOut | State::ResultIn | State::Result | State::ResultAsk | State::ResultOut | State::EndIn | State::EndBonus | State::EndOut | State::MovieIn | State::Away | State::MovieOut | State::End);
        if self.readmode.step(&sys.input, &mut sys.settings, active, self.state == State::Read) {
            sys.lock = 10;
        }
        if free {
            self.readmode.tick(&sys.settings);
        }
    }

    /// Un sous-état d'AD_AdvSubProc est ouvert : l'historique, le menu du
    /// jeu, les options, la carte mémoire, les fenêtres cachées.
    fn sub_state(&self) -> bool {
        self.hidden || matches!(self.state, State::Log | State::LogAsk | State::LogRewind | State::Menu | State::MenuAsk | State::ToRoom | State::Options | State::CardIn(_) | State::Card | State::Back)
    }

    /// AD_ReadModeCheck pour la page en cours.
    fn check(&self, sys: &Sys) -> i16 {
        self.readmode.check(&sys.settings, self.page_read)
    }

    /// La durée des passages d'un lot à l'autre (fondus, fenêtre de texte :
    /// adv_main, état 2) : 0xf images, 0xb − vitesse en avance rapide.
    fn mode_dur(&self, sys: &Sys) -> i32 {
        if self.check(sys) == readmode::SKIP { 0xB - sys.settings.skip_speed(false) } else { 0xF }
    }

    /// La carte s'ouvre (AD_ModeChangeProc2 et 3, cas 4) : le script de la
    /// carte pose les lieux (AD_MapScrirptProc), le curseur va au premier
    /// lieu choisissable (AD_MapStart), les bandeaux s'ouvrent en 0x1e images
    /// (AD_MapWinMake).
    fn open_map(&mut self, gfx: &mut Gfx, font: &Font, atlas: &mut Atlas) {
        let entry = self.world.map_script.clone();
        self.run(&entry);
        let map = Map::start(&self.world);
        let comment = self.place_comment(map.cursor, gfx, font, atlas);
        let mut text = b"#Scale[0.9]".to_vec();
        text.extend_from_slice(font.message(Message::MapTitle));
        let title = font::typeset(font, atlas, gfx, &text, 0.0);
        for (name, glyphs, y, w) in [(MAP_COMMENT, comment, 0x1A4, 0x1E5), (MAP_TITLE, title, 0x1E, 0xF0)] {
            self.windows.create(name, 4, 100, 2, Some(8), (0x14, 4), glyphs);
            self.windows.act(name, &[Action { dur: 0x1E, kind: 1, x: 0x140, y, w, h: 0x22, frame: 0x6E, back: 0x6E, ..Action::READY }]);
        }
        self.map = Some(map);
    }

    /// État 0x16, sous-états 0x0a à 0x1e : l'image bonus de l'album qu'ouvre
    /// cette fin (AD_CgBonusGetJudg), si elle n'a pas encore été vue,
    /// remplace le décor et sort du noir en 0x14 images ; ○ (après 0x14
    /// images) passe. Sinon, le jeu revient (au titre après une partie
    /// commencée par はじめから ou chargée).
    fn end_bonus(&mut self, sys: &mut Sys, gfx: &mut Gfx) {
        let bonus = self.album.bonus(&self.world);
        if let Some(id) = bonus
            && let Some(name) = self.world.bgs.get(id as usize).map(|b| b.0.clone())
            && let Some(key) = self.images.bg(gfx, &self.world, &name)
        {
            self.scene.show_bg(&name, key);
            self.album.record(id, false);
            sys.blackout.end(&mut sys.effects, 0x14);
            sys.lock = 0x14;
            self.state = State::EndBonus;
        } else {
            self.state = State::EndOut;
        }
    }

    /// AD_CheckWinMake : une question de l'exécutable, はい ou いいえ
    /// (いいえ choisi d'abord).
    fn ask(&mut self, addr: usize, size: (i32, i32), font: &Font, atlas: &mut Atlas, gfx: &mut Gfx) {
        let text = |t: &[u8], atlas: &mut Atlas, gfx: &mut Gfx| font::typeset(font, atlas, gfx, t, 0.0);
        let question = text(font.string(addr), atlas, gfx);
        let (yes, no) = (text(font.message(Message::Yes), atlas, gfx), text(font.message(Message::No), atlas, gfx));
        self.windows.ask(question, yes, no, size);
    }

    /// État 0x0d (AD_DistributeProc) : les flèches vont de case en case
    /// (en boucle ; la case de Sakura est sautée dans le sens du
    /// mouvement), la souris aussi ; ○ (ou un clic sur la case) choisit la
    /// fille : sa case clignote (AD_DistributeEnd2) et sa question s'ouvre.
    fn distribute(&mut self, sys: &mut Sys, gfx: &mut Gfx, font: &Font, atlas: &mut Atlas) {
        let Some(s) = &mut self.share else { return };
        // Les cases des filles (pas celle de Sakura).
        let zones: Vec<_> = (0..6).filter(|&c| c != 1).map(|c| Zone::new(result::cell_rect(c), (c % 3, c / 3))).collect();
        let Pick { hovered: pointed, clicked } = pointer::pick(sys, &zones);
        if sys.menu_key(0x1F) || clicked.is_some() {
            if let Some(c) = clicked {
                s.cell = c;
            }
            sys.menu_sound(2);
            result::flash(&mut sys.effects, s.cell);
            let girl = share_girl(s.cell) as usize;
            self.ask(SHARE_ASK[girl.min(4)], (0xD2, 0x51), font, atlas, gfx);
            self.state = State::DistributeAsk;
            return;
        }
        let before = s.cell;
        let (mut col, mut row) = before;
        if sys.input.direct(0, 0x14, 7, true) {
            row -= 1;
        } else if sys.input.direct(1, 0x14, 7, true) {
            row += 1;
        }
        if sys.input.direct(2, 0x14, 7, true) {
            col -= 1;
        } else if sys.input.direct(3, 0x14, 7, true) {
            col += 1;
        }
        if let Some(c) = pointed {
            (col, row) = c;
        }
        if (col, row) != before {
            col = col.rem_euclid(3);
            row = row.rem_euclid(2);
            if (col, row) == (1, 0) {
                match before {
                    (0, _) => col = 2,
                    (2, _) => col = 0,
                    _ => row = 1,
                }
            }
        }
        s.cell = (col, row);
    }

    /// État 9 : la carte attend (AD_AdvSaveProc copie la partie).
    fn wait_map(&mut self) {
        self.keep();
        self.state = State::Map;
    }

    /// Le commentaire d'un lieu, à l'échelle 0,9.
    fn place_comment(&self, k: usize, gfx: &mut Gfx, font: &Font, atlas: &mut Atlas) -> Vec<Glyph> {
        let mut text = b"#Scale[0.9]".to_vec();
        if let Some(p) = self.world.places.get(k) {
            text.extend_from_slice(&p.comment);
        }
        font::typeset(font, atlas, gfx, &text, 0.0)
    }

    /// État 9 : la carte ; le lieu choisi donne le script suivant et la
    /// valeur de « switch » (AD_MapProc) ; sinon □, ✕ et L1
    /// (AD_AdvSubProc).
    fn map(&mut self, sys: &mut Sys, gfx: &mut Gfx, font: &Font, atlas: &mut Atlas) {
        if self.hidden {
            self.sub_keys(sys, font, atlas, gfx);
            return;
        }
        let Some(map) = &mut self.map else { return };
        let before = map.cursor;
        let chosen = map.step(sys, &self.world);
        let cursor = map.cursor;
        if cursor != before {
            let glyphs = self.place_comment(cursor, gfx, font, atlas);
            self.windows.set_text(MAP_COMMENT, glyphs);
        }
        if let Some(k) = chosen
            && let Some(p) = self.world.places.get(k).cloned()
        {
            self.world.next = p.entry;
            self.world.flags.set(b"switch", p.data as i16, false);
            sys.menu_sound(2);
            map::chosen(k, &mut sys.effects);
            self.scene.clear_charas();
            self.timer = 0x1E;
            self.state = State::MapChosen;
        } else {
            self.sub_keys(sys, font, atlas, gfx);
        }
    }

    /// AD_AdvSubProc, sous-état 0 : depuis une page, un choix ou la carte
    /// qui attend, L1 (la molette vers le haut) ouvre l'historique, ✕ (clic
    /// droit) cache les fenêtres, □ (Échap) ouvre le menu du jeu ; fenêtres
    /// cachées (sous-état 2), ✕ (ou ○, un clic) les rend. Vrai si une de ces
    /// touches a servi. Ces touches attendent la fin du compte à rebours
    /// (l'attente de l'avance rapide, de la lecture automatique, d'un
    /// choix) ; ouvrir un sous-état efface le signe de l'avance.
    fn sub_keys(&mut self, sys: &mut Sys, font: &Font, atlas: &mut Atlas, gfx: &mut Gfx) -> bool {
        sys.lock_asked = true;
        if sys.lock > 0 {
            return self.hidden;
        }
        if self.hidden && (sys.input.key(2) || sys.input.key(3) || sys.click.is_some()) {
            self.reveal(|_| true, &sys.settings);
            self.hidden = false;
            return true;
        }
        if (sys.input.key(4) || sys.input.tapped(Button::WheelUp)) && !self.backlog.is_empty() {
            // L'historique, sur l'écran assombri (plan 4).
            if self.state != State::Log {
                self.waiting = self.state;
            }
            sys.menu_sound(0);
            sys.stop_voice();
            self.scene.darken(4, &mut sys.effects, 0xF, 0x40);
            self.conceal(&self.all_windows(), true);
            self.hidden = false;
            self.backlog.open();
            self.icon = -1;
            self.state = State::Log;
        } else if !self.hidden && sys.input.key(2) && self.windows.is_open(MAINWIN) {
            sys.menu_sound(0);
            self.conceal(&self.all_windows(), true);
            self.hidden = true;
        } else if sys.input.key(1) {
            self.waiting = self.state;
            self.open_menu(sys, font, atlas, gfx);
        } else {
            return self.hidden;
        }
        self.readmode.cut();
        true
    }

    /// Les fenêtres d'une page, d'un choix, de la carte.
    fn all_windows(&self) -> Vec<&'static str> {
        let mut names = vec![MAINWIN, SPEAKER];
        names.extend_from_slice(&SELECT[..self.choices.len()]);
        names.extend_from_slice(&[MAP_COMMENT, MAP_TITLE]);
        names
    }

    /// AD_AdvMainWinClose, AD_AdvSelectWinClose, AD_SelCsrHid,
    /// AD_MapWinHid : les fenêtres ouvertes parmi celles-ci se cachent (en
    /// 10 images, la carte en 15), le curseur du choix aussi.
    fn conceal(&mut self, names: &[&'static str], cursor: bool) {
        for &name in names {
            if self.windows.is_open(name) && !self.concealed.contains(&name) {
                let dur = if matches!(name, MAP_COMMENT | MAP_TITLE) { 0xF } else { 10 };
                self.windows.act(name, &[Action { dur, kind: 2, ..Action::READY }]);
                self.concealed.push(name);
            }
        }
        if cursor && !self.choices.is_empty() && !self.cursor_concealed {
            self.windows.cursor_hide(SELECT_CURSOR, 0, 0xF);
            self.cursor_concealed = true;
        }
    }

    /// Les fenêtres cachées (celles que retient `keep`) reviennent : la
    /// fenêtre de texte (AD_AdvMainWinOpen) avec son texte d'un coup, le
    /// curseur du choix (AD_SelCsrShow).
    fn reveal(&mut self, keep: impl Fn(&str) -> bool, settings: &Settings) {
        let names: Vec<&'static str> = self.concealed.iter().copied().filter(|n| keep(n)).collect();
        self.concealed.retain(|n| !keep(n));
        for name in names {
            match name {
                MAINWIN => self.open_mainwin(10, settings),
                MAP_COMMENT | MAP_TITLE => self.windows.act(name, &[Action { dur: 0xF, kind: 1, ..Action::READY }]),
                _ => {
                    let [frame, back] = settings.window.map(i32::from);
                    let alpha = if name == SPEAKER { Action { frame, back, ..Action::READY } } else { Action::READY };
                    self.windows.act(name, &[Action { dur: 10, kind: 1, ..alpha }]);
                }
            }
        }
        if self.cursor_concealed && keep(SELECT_CURSOR) {
            self.windows.cursor_show(SELECT_CURSOR, 0x7F, 0xF);
            self.cursor_concealed = false;
        }
    }

    /// AD_AdvMainWinOpen : la fenêtre de texte s'ouvre en `dur` images, aux
    /// opacités des options, son texte d'un coup.
    fn open_mainwin(&mut self, dur: i32, settings: &Settings) {
        self.windows.instant_text(MAINWIN);
        let [frame, back] = settings.window.map(i32::from);
        self.windows.act(MAINWIN, &[Action { dur, kind: 1, frame, back, ..Action::READY }]);
    }

    /// Le retour à ce qu'attendait le joueur.
    fn back_to_waiting(&mut self) {
        self.state = self.waiting;
        if self.waiting == State::Read {
            self.icon = 0;
        }
    }

    /// AD_AdvSubProc, sous-état 4, et AD_AdvMenuOpen : le plan 4
    /// s'assombrit (0x40 en 0xf images), la page et les réponses
    /// reviennent si elles étaient cachées, le curseur du choix et la carte
    /// se cachent ; la fenêtre « menu » s'ouvre en 10 images, le curseur
    /// paraît en 15 sur la première entrée.
    pub fn open_menu(&mut self, sys: &mut Sys, font: &Font, atlas: &mut Atlas, gfx: &mut Gfx) {
        sys.menu_sound(0);
        sys.stop_voice();
        self.scene.darken(4, &mut sys.effects, 0xF, 0x40);
        self.reveal(|n| !matches!(n, MAP_COMMENT | MAP_TITLE | SELECT_CURSOR), &sys.settings);
        self.hidden = false;
        self.conceal(&[MAP_COMMENT, MAP_TITLE], true);
        let glyphs = font::typeset(font, atlas, gfx, font.message(Message::GameMenu), 0.0);
        self.windows.create(GAME_MENU, 1, 0xDC, 2, None, (0xC, 0xC), glyphs);
        let [x, y, w, h] = MENU_WIN;
        self.windows.act(GAME_MENU, &[Action { dur: 10, kind: 1, x, y, w, h, frame: 0x80, back: 0x80, ..Action::READY }]);
        let ask = |m: Message, atlas: &mut Atlas, gfx: &mut Gfx| font::typeset(font, atlas, gfx, font.message(m), 0.0);
        self.ask = (ask(Message::BackToRoomAsk, atlas, gfx), ask(Message::Yes, atlas, gfx), ask(Message::No, atlas, gfx));
        self.menu = (0, 0, 6, 0);
        self.icon = -1;
        self.state = State::Menu;
    }

    /// AD_AdvMenuProc : ↑ ↓ (en boucle sur cinq) choisissent, ○ valide ; □,
    /// ✕ ou Échap referment. Les entrées : sauvegarder, charger (l'écran de
    /// la carte mémoire), les options, la chambre (après une question),
    /// retour.
    fn menu(&mut self, sys: &mut Sys) {
        let (choice, before, t, appear) = &mut self.menu;
        *appear = (*appear + 1).min(0xF);
        *t = (*t + 1).min(6);
        // Les entrées, sur la largeur de la fenêtre.
        let [x, _, w, _] = MENU_WIN;
        let zones: [Zone<usize>; 5] = std::array::from_fn(|k| Zone::new([(x - w / 2) as f32, (menu_y(k) - 0xF) as f32, w as f32, 30.0], k));
        let Pick { hovered, clicked } = pointer::pick(sys, &zones);
        if let Some(k) = hovered
            && k != *choice
        {
            (*before, *choice, *t) = (*choice, k, 0);
            sys.menu_sound(1);
        }
        if sys.input.direct(0, 0x14, 6, true) {
            (*before, *choice, *t) = (*choice, (*choice + 4) % 5, 0);
            sys.menu_sound(1);
        } else if sys.input.direct(1, 0x14, 6, true) {
            (*before, *choice, *t) = (*choice, (*choice + 1) % 5, 0);
            sys.menu_sound(1);
        } else if sys.input.key(0x21) || clicked.is_some() {
            match *choice {
                0 | 1 => {
                    // セーブ, ロード (sous-états 7 et 8) : le menu se ferme, le
                    // noir tombe en 10 images, les fenêtres se cachent.
                    let save = *choice == 0;
                    sys.menu_sound(2);
                    self.windows.act(GAME_MENU, &[Action { dur: 5, kind: 2, ..Action::READY }]);
                    sys.blackout.start(&mut sys.effects, 10, 0x7F);
                    self.conceal(&self.all_windows(), true);
                    self.timer = 10;
                    self.state = State::CardIn(save);
                }
                2 => {
                    // オプション (sous-état 5) : le menu se ferme, l'écran
                    // s'assombrit presque au noir (plan 4 à 0x7f), la fenêtre
                    // de texte se cache, la voix s'arrête ; le jeu ouvre les
                    // options.
                    sys.menu_sound(2);
                    self.windows.act(GAME_MENU, &[Action { dur: 5, kind: 2, ..Action::READY }]);
                    self.scene.darken(4, &mut sys.effects, 0xF, 0x7F);
                    self.conceal(&self.all_windows(), true);
                    sys.stop_voice();
                    sys.pause_loops();
                    self.request = Some(Request::Options);
                    self.state = State::Options;
                }
                3 => {
                    // 桜くんの部屋 : « 桜くんの部屋へ戻ります よろしいですか？ ».
                    sys.menu_sound(2);
                    let (q, yes, no) = self.ask.clone();
                    self.windows.ask(q, yes, no, (0xFA, 0x37));
                    sys.dim.start(&mut sys.effects, 0xF, 0x40);
                    self.state = State::MenuAsk;
                }
                4 => self.close_menu(sys),
                _ => {}
            }
        } else if sys.input.key(1) || sys.input.key(2) {
            self.close_menu(sys);
        }
    }

    /// AD_AdvSubProc, sous-état 6 : le menu se ferme, l'écran s'éclaircit,
    /// ce qui était caché revient.
    fn close_menu(&mut self, sys: &mut Sys) {
        sys.menu_sound(0);
        self.windows.act(GAME_MENU, &[Action { dur: 5, kind: 2, ..Action::READY }]);
        self.scene.lighten(4, &mut sys.effects, 0xF);
        self.reveal(|_| true, &sys.settings);
        self.back_to_waiting();
    }

    /// L'écran de la carte mémoire refermé sans chargement (sous-état 6) :
    /// l'écran s'éclaircit, les fenêtres reviennent, le jeu attend de
    /// nouveau après 15 images.
    pub fn back_from_card(&mut self, sys: &mut Sys) {
        sys.resume_loops();
        self.scene.lighten(4, &mut sys.effects, 0xF);
        self.reveal(|_| true, &sys.settings);
        self.timer = 0xF;
        self.state = State::Back;
    }

    /// AD_AdvSaveProc : la page qui attend, figée.
    pub fn snapshot(&self, sys: &Sys) -> savedata::Game {
        savedata::Game {
            title: self.world.save_name.clone(),
            time: [0; 5],
            entry: self.at_entry.0.clone(),
            flags_at_entry: self.at_entry.1.clone(),
            batch: self.current,
            flags: self.world.flags.normal.clone(),
            music: sys.music(),
            loops: sys.loops(),
            speaker: self.speaker.clone(),
            map_script: self.world.map_script.clone(),
            scene: self.scene.snapshot(),
            state: match self.waiting {
                State::Choose => savedata::Waiting::Choose,
                State::Map => savedata::Waiting::Map,
                _ => savedata::Waiting::Read,
            },
        }
    }

    /// Une partie chargée reprend (AD_AdvLoadProc puis le sous-état 9 :
    /// AD_AdvLoadAfterProc) : les drapeaux du départ du point d'entrée, qui
    /// est rejoué ; le lot de la page, les drapeaux de la page, la scène ; le
    /// texte d'un coup, le nom, la page seule dans l'historique ; la musique ;
    /// puis le choix (AD_AdvSelectWinMake) ou la carte (AD_MapWinMake). Le
    /// jeu, lui, a mis le noir, qui s'efface ensuite.
    /// keep_log : le retour à une page de l'historique le garde.
    pub fn load(&mut self, g: &savedata::Game, sys: &mut Sys, gfx: &mut Gfx, font: &Font, atlas: &mut Atlas, keep_log: bool) {
        self.start_windows(&sys.settings);
        if !keep_log {
            // AD_AdvPartStartProc(4) : une partie chargée finit au titre.
            self.from_room = false;
            self.backlog = Backlog::default();
        }
        self.hidden = false;
        self.concealed.clear();
        self.cursor_concealed = false;
        self.choices.clear();
        self.map = None;
        self.world.flags.normal = g.flags_at_entry.clone();
        self.world.map_script = g.map_script.clone();
        self.world.save_name = g.title.clone();
        self.world.batches.clear();
        self.world.menu = Default::default();
        self.at_entry = (g.entry.clone(), g.flags_at_entry.clone());
        self.readmode.stop(&sys.settings, -1);
        self.start_place(&g.entry);
        if !self.run(&g.entry) || self.world.batches.is_empty() {
            self.state = State::End;
            return;
        }
        self.current = g.batch.min(self.world.batches.len() - 1);
        // La place de la page, lot après lot, et si elle est déjà lue.
        for k in 0..=self.current {
            let route = self.world.batches[k].route;
            self.advance_place(route);
        }
        if g.state != savedata::Waiting::Map {
            self.page_read = self.page_is_read();
        }
        self.world.flags.normal = g.flags.clone();
        self.scene.restore(&g.scene, &self.world, &mut self.images, gfx, &mut sys.effects);
        let Some(b) = self.batch().cloned() else { return };
        self.kind = b.kind;
        self.speaker = None;
        if g.state != savedata::Waiting::Map {
            let text = self.page_text(&b);
            let glyphs = font::typeset(font, atlas, gfx, &text, 0.0);
            self.windows.set_text(MAINWIN, glyphs);
            if let Some(name) = &g.speaker {
                self.open_speaker(name, 8, sys, gfx, font, atlas);
            }
            self.open_mainwin(10, &sys.settings);
            self.log_page(&b, &text, 0x96, gfx, font, atlas);
        }
        sys.stop_music();
        if let Some(m) = g.music {
            sys.play_music(m);
        }
        // Les bruitages et les boucles en cours se taisent ; les boucles de
        // la partie reprise repartent (AD_AdvLoadProc).
        sys.stop_sounds();
        sys.restore_loops(g.loops);
        self.waiting = match g.state {
            savedata::Waiting::Read => State::Read,
            savedata::Waiting::Choose => {
                self.open_choices(sys, gfx, font, atlas);
                State::Choose
            }
            savedata::Waiting::Map => {
                // AD_ModeChangeLoadProc : la carte d'emblée, sans arrivée.
                self.open_map(gfx, font, atlas);
                State::Map
            }
        };
        self.timer = 0xF;
        self.state = State::Back;
    }

    /// Le texte d'une page, et ce qui le précède (AD_ModeChangeProc2).
    fn page_text(&self, b: &Batch) -> Vec<u8> {
        let mut text = PREFIX.to_vec();
        text.extend_from_slice(&b.text);
        text
    }

    /// AD_AdvSpeakerChange : la fenêtre du nom, à sa longueur, s'ouvre en
    /// `dur` images.
    fn open_speaker(&mut self, name: &[u8], dur: i32, sys: &Sys, gfx: &mut Gfx, font: &Font, atlas: &mut Atlas) {
        let mut shown = name.to_vec();
        shown.push(b' ');
        let n = shown.len() as i32;
        let glyphs = font::typeset(font, atlas, gfx, &shown, 0.0);
        self.windows.create(SPEAKER, 2, 0x4B, 2, Some(8), (0x14, 6), glyphs);
        let w = (f64::from(n) * 12.5 + 40.0 - 12.5 + 6.0) as i32;
        let [frame, back] = sys.settings.window.map(i32::from);
        self.windows.act(SPEAKER, &[Action { dur, kind: 1, x: n * 6 + 0x32, y: 300, w, h: 0x28, frame, back, ..Action::READY }]);
        self.speaker = Some(name.to_vec());
    }

    /// AD_BackLogSet, AD_BackLogSizeSet : la page dans l'historique, son nom
    /// en première ligne, avec sa voix.
    fn log_page(&mut self, b: &Batch, text: &[u8], height: i32, gfx: &mut Gfx, font: &Font, atlas: &mut Atlas) {
        let mut logged = b.speaker.clone().map(|mut s| {
            s.extend_from_slice(b"#n");
            s
        }).unwrap_or_default();
        logged.extend_from_slice(text);
        let glyphs = font::typeset(font, atlas, gfx, &logged, 0.0);
        self.backlog.push(backlog::Entry { glyphs, width: 0x253, height, voice: b.voice.map(|v| v.0), style: 0, rewind: None }, true);
    }

    /// AD_BackLogModeProc. Tant que la voix d'une page joue, ○, L1 ou ✕
    /// l'arrêtent, et rien d'autre ne répond (l'icône de la voix bat). Sinon
    /// : ↑ ou L1 remontent, ↓ ou R1 redescendent ; ○ fait réentendre la voix
    /// de la page ; ✕ ou L1 referment ; □ ouvre le menu du jeu ; △, sur une
    /// page plus ancienne que la dernière, propose d'y revenir. À la souris,
    /// comme dans un visual novel sur PC : la molette fait défiler, et vers
    /// le bas depuis la page la plus récente elle referme.
    fn log(&mut self, sys: &mut Sys, font: &Font, atlas: &mut Atlas, gfx: &mut Gfx) {
        if sys.voice_playing() {
            if sys.input.key(3) || sys.input.key(4) || sys.input.key(2) {
                sys.stop_voice();
            }
            self.backlog.icons(2, None);
            return;
        }
        if !self.backlog.busy() {
            let wheel_close = sys.input.tapped(Button::WheelDown) && self.backlog.at_newest();
            if sys.input.key(2) || sys.input.key(4) || wheel_close {
                sys.menu_sound(0);
                self.backlog.close();
                self.scene.lighten(4, &mut sys.effects, 0xF);
                self.reveal(|_| true, &sys.settings);
                self.back_to_waiting();
                return;
            }
            if sys.input.key(1) {
                // Sous-état 4 : le menu du jeu, l'historique refermé.
                self.backlog.close();
                self.open_menu(sys, font, atlas, gfx);
                return;
            }
            if sys.input.key(3) {
                if let Some(id) = self.backlog.voice() {
                    sys.voice(id);
                }
            } else if sys.input.direct(0, 0x14, 4, true) || sys.input.tapped(Button::WheelUp) {
                self.backlog.scroll(true, 4);
            } else if sys.input.direct(1, 0x14, 4, true) || sys.input.key(5) || sys.input.tapped(Button::WheelDown) {
                self.backlog.scroll(false, 4);
            }
        }
        let busy = self.backlog.busy();
        let mic = i32::from(self.backlog.voice().is_some() && !busy);
        let back = self.backlog.rewind().is_some() && !busy;
        self.backlog.icons(mic, Some(i32::from(back)));
        // AD_RewindLoad : la question « この場面へ巻き戻します よろしいですか？ »
        // sur l'écran assombri (plan 5, 0x40 en 0xf images).
        if back && sys.input.key(0) && sys.lock < 1 {
            sys.dim.start(&mut sys.effects, 0xF, 0x40);
            self.ask(REWIND_ASK, (0xFA, 0x37), font, atlas, gfx);
            self.state = State::LogAsk;
        }
    }

    /// Sous-état 1, 0x28 : le retour à une page. Les bruitages et la voix
    /// s'arrêtent, « 巻き戻し » entre dans l'historique (gardé), et la partie
    /// de cette page reprend comme une partie chargée
    /// (AD_AdvPartStartProc(5)).
    fn rewind(&mut self, sys: &mut Sys, gfx: &mut Gfx, font: &Font, atlas: &mut Atlas) {
        let Some(game) = self.backlog.rewind().cloned() else {
            self.back_to_waiting();
            return;
        };
        sys.se.stop(0);
        sys.stop_voice();
        let glyphs = font::typeset(font, atlas, gfx, font.string(REWIND_MARK), 0.0);
        self.backlog.push(backlog::Entry { glyphs, width: 0x96, height: 0x32, voice: None, style: 0, rewind: None }, false);
        self.load(&game, sys, gfx, font, atlas, true);
        sys.blackout.end(&mut sys.effects, 10);
    }

    /// Le compte à rebours (sRam01da756a) : vrai quand il est passé sous 0.
    fn countdown(&mut self) -> bool {
        self.timer -= 1;
        self.timer < 0
    }

    /// État 0x0e : la partie ; puis (0x0f) son résultat va au script et,
    /// gagnée, l'image bonus du jeu et du personnage choisi
    /// (AD_MiniGameBonusCgGet) paraît sur la musique 0x14.
    fn minigame(&mut self, sys: &mut Sys, gfx: &mut Gfx) {
        let Some(session) = &mut self.session else { return };
        let done = session.step(sys);
        if session.take_played() {
            self.played = Some(sys.kind);
        }
        if !done {
            return;
        }
        self.session = None;
        self.world.selchara = sys.chara;
        self.world.won = i32::from(sys.won);
        let cg = self.world.bonus_cg.iter().rev().find(|c| c.0 == sys.kind && c.1 == sys.chara).map(|c| c.2.clone());
        if let (true, Some(cg)) = (sys.won, cg)
            && let Some(key) = self.images.bg(gfx, &self.world, &cg)
        {
            self.scene.show_bg(&cg, key);
            if let Some(id) = album::bg_id(&self.world, &cg) {
                self.album.record(id, false);
            }
            sys.play_music(0x14);
            sys.blackout.end(&mut sys.effects, 10);
            self.state = State::Bonus;
        } else {
            self.timer = 10;
            self.state = State::MinigameOut;
        }
    }

    /// État 1 : le lot suivant, ou un nouveau passage du script.
    fn next(&mut self) {
        if self.current + 1 < self.world.batches.len() {
            self.current += 1;
            self.state = State::Change;
            return;
        }
        let entry = self.world.next.clone();
        // Les tests (debug) : DOKURO_TRACE écrit chaque point d'entrée joué,
        // à reprendre avec DOKURO_STORY.
        if cfg!(debug_assertions) && std::env::var_os("DOKURO_TRACE").is_some() {
            eprintln!("{}:{}", entry.file, String::from_utf8_lossy(&entry.name));
        }
        self.at_entry = (entry.clone(), self.world.flags.normal.clone());
        self.start_place(&entry);
        self.world.batches.clear();
        self.world.menu = Default::default();
        if entry.name.is_empty() || !self.run(&entry) {
            self.state = State::End;
            return;
        }
        self.current = 0;
        self.state = State::Change;
    }

    /// AD_AdvBthFormat : la place des pages d'un passage du script — son
    /// numéro, le nombre qui suit les quatre premiers caractères du nom du
    /// point d'entrée (strtol ; le nom est copié sur le précédent, dont la
    /// fin reste derrière un nom plus court), et la valeur de « switch ».
    fn start_place(&mut self, entry: &Entry) {
        let n = entry.name.len().min(self.entry_name.len() - 1);
        self.entry_name[..n].copy_from_slice(&entry.name[..n]);
        self.entry_name[n] = 0;
        let switch = self.world.flags.get(b"switch", false);
        self.place = Place { file: entry.file, entry: strtol(&self.entry_name[4..]), switch: i32::from(switch), page: -1, route: -1, in_route: 0 };
    }

    /// AD_AdvBthBeforeProc : la page suivante, dans le passage ou dans sa
    /// route (une route commencée repart de la page suivante du passage).
    fn advance_place(&mut self, route: i16) {
        let p = &mut self.place;
        if route == -2 {
            p.route = -1;
        } else if route >= 0 {
            p.in_route = -1;
            p.page += 1;
            p.route = i32::from(route);
        }
        if p.route == -1 {
            p.page += 1;
        } else {
            p.in_route += 1;
        }
    }

    /// AD_BReadCheck pour la page : dans le passage (sorte 0) ou dans sa
    /// route (sorte 1).
    fn page_is_read(&mut self) -> bool {
        let kind = if self.place.route < 0 { 0 } else { 1 };
        self.read.check(kind, &self.place, 0)
    }

    /// État 2 : AD_AdvBthBeforeProc, AD_ModeChangeProc1 à 5, puis
    /// AD_AdvBthAfterProc.
    fn change(&mut self, sys: &mut Sys, gfx: &mut Gfx, font: &Font, atlas: &mut Atlas) {
        let Some(mut b) = self.batch().cloned() else {
            self.state = State::Next;
            return;
        };
        // AD_AdvBthBeforeProc : un changement de scène posé par le script
        // n'attend ni les apparitions ni les disparitions (+0x3a8, +0x3ac).
        if b.flash().is_some() {
            b.wait_app = 0;
            b.wait_del = 0;
        }
        let previous = std::mem::replace(&mut self.kind, b.kind);
        self.advance_place(b.route);
        // L'écran de la répartition cède la place à celui du lot
        // (AD_GPTranModeSet).
        self.share = None;
        let text_kind = matches!(b.kind, Kind::Text | Kind::Menu | Kind::MenuNoWait);
        // La durée des passages d'AD_ModeChangeProc1, avant que la page soit
        // vérifiée.
        let before = self.mode_dur(sys);
        // Le nom : la fenêtre se ferme s'il n'y en a pas (plus vite pendant
        // l'avance rapide) ou s'il change.
        if text_kind && b.speaker.as_ref() != self.speaker.as_ref() && self.speaker.is_some() {
            let dur = if b.speaker.is_some() {
                7
            } else if self.check(sys) == readmode::SKIP {
                0xC - sys.settings.skip_speed(false)
            } else {
                10
            };
            self.windows.act(SPEAKER, &[Action { dur, kind: 2, h: 0, ..Action::READY }]);
            self.speaker = None;
        }
        // AD_ModeChangeProc2 : la page est-elle déjà lue ? AD_ModeChangeProc3
        // : l'avance s'arrête, si les options le disent, à une page pas lue,
        // à un choix, au résultat de la journée (les points d'affection), à
        // une vidéo ; toujours à la fin du jeu.
        if text_kind {
            self.page_read = self.page_is_read();
        }
        // AD_ModeChangeProc5 : une page ou la carte efface le noir laissé par
        // la carte ou le résultat de la journée ; un lot sans texte (hors des
        // lots vides) ferme la fenêtre de texte.
        // L'état 2 attend ce passage s'il a eu lieu (sa durée), puis ce que
        // rend l'affichage de la sorte (AD_ModeChangeProc3).
        let after = self.mode_dur(sys);
        let mut waited = 0;
        if (text_kind || b.kind == Kind::Map) && sys.blackout.on() {
            sys.blackout.end(&mut sys.effects, after);
            waited = after;
        }
        if !text_kind && b.kind != Kind::Silent && self.windows.is_open(MAINWIN) {
            self.windows.act(MAINWIN, &[Action { dur: after, kind: 2, ..Action::READY }]);
            waited = after;
        }
        match b.kind {
            Kind::Text if !self.page_read => self.readmode.stop(&sys.settings, 1),
            Kind::Menu | Kind::MenuNoWait => self.readmode.stop(&sys.settings, 0),
            Kind::Result => self.readmode.stop(&sys.settings, 2),
            Kind::Movie => self.readmode.stop(&sys.settings, 3),
            Kind::GameEnd => self.readmode.stop(&sys.settings, -1),
            _ => {}
        }
        self.windows.set_text(MAINWIN, Vec::new());
        // AD_AdvBthAfterProc : comment ce lot succède au précédent. D'un lot
        // de texte (sortes 0 à 3) vers une autre sorte, ses transitions se
        // calent sur la fin du passage, la durée qu'attend l'état 2
        // (sRam01da756a : celle d'AD_ModeChangeProc5, relevée par ce que rend
        // AD_ModeChangeProc3) ; entre deux autres sortes, elles se font
        // aussitôt ; sinon, un changement de scène les décale de son éclair.
        let texty = |k: Kind| matches!(k, Kind::Text | Kind::Silent | Kind::Menu | Kind::MenuNoWait);
        let pass = if texty(previous) && !texty(b.kind) {
            let shown = match b.kind {
                Kind::Map => 0x3E,
                Kind::Mogura | Kind::Run | Kind::Card | Kind::Catch | Kind::Bat => minigame::ARRIVAL,
                _ => 0,
            };
            Pass::Into(waited.max(shown))
        } else if !texty(previous) && previous != b.kind {
            Pass::Switch
        } else {
            b.flash().map_or(Pass::Plain, Pass::Flash)
        };
        if self.scene.apply(&b, pass, &self.world, &mut self.images, gfx, &mut sys.effects)
            && let Some(id) = b.bg.as_ref().and_then(|g| album::bg_id(&self.world, &g.name))
        {
            self.album.record(id, false);
        }
        // La musique (AD_BGMPlay, AD_BGMStop : elle part au bout de son
        // retard) ; un lot de vidéo la note seulement (AD_BGMPlayRecord), la
        // vidéo relancera la musique en cours. Les bruitages
        // (AD_SEReservePlay), leurs retards.
        if let Some((n, delay)) = b.bgm
            && b.kind != Kind::Movie
        {
            sys.request_music(n, pass.music_delay(delay));
        }
        for &(id, delay) in &b.se {
            let delay = delay + pass.sound_delay();
            if id < 0 {
                sys.se.stop(delay);
            } else {
                sys.se.play(id, delay);
            }
        }
        // Les bruitages en boucle (AD_LoopSEPlay, AD_LoopSEStop), décalés
        // comme les autres. Le portage les lisait sans les jouer.
        for (slot, l) in b.loop_se.iter().enumerate() {
            if let Some((id, delay)) = *l {
                sys.loop_se(slot, id, delay + pass.sound_delay());
            }
        }
        // La durée du lot (AD_AdvBthBeforeProc) : le plus long de ce qu'il
        // lance.
        let camera: i32 = b.targets[0].iter().take_while(|t| t.dur >= 0).map(|t| i32::from(t.dur)).sum();
        self.total = (b.wait_app + b.wait_bg + b.wait_del).max(camera).max(b.wait);
        self.count = 0;
        self.fast = false;
        match b.kind {
            Kind::Text | Kind::Menu | Kind::MenuNoWait | Kind::Silent => self.state = State::Wait,
            // AD_ModeChangeProc1 (cas 7) : le résultat, depuis le noir ; le
            // jeu n'attend que la fermeture de la fenêtre de texte.
            Kind::Result => {
                if !sys.blackout.on() {
                    sys.blackout.start(&mut sys.effects, before, 0x7F);
                }
                self.timer = waited;
                self.state = State::ResultIn;
            }
            // AD_ModeChangeProc2 et 3 (cas 4) : la carte se prépare et arrive
            // par-dessus la scène (AD_GPTranModeSet(3) : AD_MapStEffectMake) ;
            // elle se dessine et répond à la fin de l'arrivée.
            Kind::Map => {
                self.open_map(gfx, font, atlas);
                let map = self.map.as_ref().map_or(0, |m| m.arrival(&self.world, &mut sys.effects));
                self.timer = map.max(waited);
                self.state = State::MapIn;
            }
            // AD_ModeChangeProc3 (cas 9 à 0x0d) : le fond du choix du
            // personnage arrive par-dessus la scène
            // (AD_MiniGameStartStEffectMake), puis la partie commence.
            Kind::Mogura | Kind::Run | Kind::Card | Kind::Catch | Kind::Bat => {
                sys.kind = match b.kind {
                    Kind::Mogura => 0,
                    Kind::Run => 1,
                    Kind::Card => 2,
                    Kind::Catch => 3,
                    _ => 4,
                };
                self.timer = minigame::arrival(&mut sys.effects).max(waited);
                self.session = Some(Session::start(minigame::Mode::Story, sys));
                self.count = 0;
                self.state = State::MinigameIn;
            }
            // AD_ModeChangeProc1 (cas 8) : le noir, la répartition commence
            // (AD_DistributeStart : le curseur sur la première case) ; son
            // écran se dessine 10 images plus tard (AD_GPTranModeSet(7)).
            Kind::Distribute => {
                if !sys.blackout.on() {
                    sys.blackout.start(&mut sys.effects, before, 0x7F);
                }
                self.share = Some(Share { cell: (0, 0), done: false });
                self.timer = waited.max(10);
                self.count = 0;
                self.state = State::DistributeIn;
            }
            // AD_ModeChangeProc1 (cas 0x0f) : le noir ; puis l'état 0x16.
            Kind::GameEnd => {
                if !sys.blackout.on() {
                    sys.blackout.start(&mut sys.effects, before, 0x7F);
                }
                self.timer = waited + 0x14;
                self.state = State::EndIn;
            }
            // AD_ModeChangeProc1 (cas 0x0e) : le noir ; puis la vidéo, que
            // l'album note vue (AD_MVPlay).
            Kind::Movie => {
                if !sys.blackout.on() {
                    sys.blackout.start(&mut sys.effects, before, 0x7F);
                }
                self.timer = waited.max(before);
                self.state = State::MovieIn;
            }
        }
        let _ = (font, atlas);
    }

    /// Fin de l'état 3 : le nom, le texte, la voix.
    fn show_text(&mut self, sys: &mut Sys, gfx: &mut Gfx, font: &Font, atlas: &mut Atlas) {
        let Some(b) = self.batch().cloned() else { return };
        if b.kind == Kind::Silent {
            self.state = State::Next;
            return;
        }
        let skipping = self.check(sys) == readmode::SKIP;
        if let Some(name) = &b.speaker
            && self.speaker.as_ref() != Some(name)
        {
            let dur = if skipping { (0xC - sys.settings.skip_speed(false)) >> 1 } else { 10 };
            self.open_speaker(name, dur, sys, gfx, font, atlas);
        }
        // Le texte paraît à la vitesse des options (AD_ReadTxtSpeedProc :
        // (10 − vitesse) / 15 image, fois le #Speed[5] de la page) ; plus
        // vite en avance rapide, même sur une page qu'elle ne passe pas :
        // fois (10 − vitesse de l'avance) / 10.
        let text = self.page_text(&b);
        self.log_page(&b, &text, if b.speaker.is_some() { 0x8C } else { 0x69 }, gfx, font, atlas);
        let mut speed = f32::from(10 - sys.settings.text_speed) / 15.0;
        if self.readmode.mode == readmode::SKIP {
            speed *= (10 - sys.settings.skip_speed(false)) as f32 / 10.0;
        }
        let glyphs = font::typeset(font, atlas, gfx, &text, speed);
        self.windows.set_text(MAINWIN, glyphs);
        self.timer = 0;
        if !self.windows.is_open(MAINWIN) {
            let [frame, back] = sys.settings.window.map(i32::from);
            self.windows.act(MAINWIN, &[Action { dur: 5, kind: 1, frame, back, ..Action::READY }]);
            self.timer = 5;
        }
        // État 4, sous-état 0 (AD_AllSndAtOnceProc) : la page paraît, la
        // musique et les bruitages encore en attente partent tout de suite.
        sys.sounds_at_once();
        // La voix, ou le bruitage en flux ; pas en avance rapide.
        if let Some((id, _)) = b.voice.or(b.stream)
            && !skipping
        {
            sys.voice(id);
        }
        self.state = State::Print;
    }

    /// Fin de l'état 4 : la page lue attend ○ ; un choix ouvre ses
    /// réponses.
    fn after_text(&mut self, sys: &mut Sys, gfx: &mut Gfx, font: &Font, atlas: &mut Atlas) {
        let time = self.windows.text_time(MAINWIN);
        if time > 0 {
            self.timer = time;
            return;
        }
        match self.kind {
            Kind::Menu | Kind::MenuNoWait => self.open_choices(sys, gfx, font, atlas),
            _ => {
                // L'attente de la page : en lecture automatique, (10 −
                // vitesse) × 5 + 10 images, plus (12 − vitesse) par
                // caractère ; en avance rapide, (11 − vitesse) / 2. Puis la
                // page est lue (AD_BReadWrite).
                sys.lock = match self.check(sys) {
                    readmode::AUTO => {
                        let speed = sys.settings.skip_speed(true);
                        (10 - speed) * 5 + 10 + (12 - speed) * self.windows.text_len(MAINWIN)
                    }
                    readmode::SKIP => (0xB - sys.settings.skip_speed(false)) >> 1,
                    _ => 0,
                };
                let kind = if self.place.route < 0 { 0 } else { 1 };
                self.read.write(kind, &self.place, 0);
                self.keep();
                self.keep_page(sys, State::Read);
                self.icon = 0;
                self.state = State::Read;
            }
        }
    }

    /// AD_AdvSelectWinMake et AD_SelCsrMake : les réponses l'une sous
    /// l'autre, à la largeur de la plus longue (12,5 × octets + 39,5) ; la
    /// première choisie (style 3), les autres en style 4 ; le curseur
    /// « select » paraît à leur gauche en 15 images. Les touches attendent
    /// 0x14 images.
    fn open_choices(&mut self, sys: &mut Sys, gfx: &mut Gfx, font: &Font, atlas: &mut Atlas) {
        self.choices = self.world.menu.iter().map_while(Clone::clone).collect();
        self.choice = 0;
        self.choice_len = self.choices.iter().map(|c| c.len() as i32).max().unwrap_or(0);
        let w = self.choice_w();
        for (k, name) in SELECT.into_iter().enumerate().take(self.choices.len()) {
            // Une réponse déjà choisie s'écrit en palette 2.
            let mut text = if self.read.check(2, &self.place, k as i32) { b"#Color[2]".to_vec() } else { Vec::new() };
            text.extend_from_slice(&self.choices[k]);
            let glyphs = font::typeset(font, atlas, gfx, &text, 0.0);
            let (style, alpha) = if k == 0 { (3, 100) } else { (4, 0x80) };
            self.windows.create(name, style, 0xA0, 2, None, (0x14, 7), glyphs);
            self.windows.act(name, &[Action { dur: 10, kind: 1, x: 0x140, y: self.choice_y(k), w, h: 0x29, frame: alpha, back: alpha, ..Action::READY }]);
        }
        self.windows.cursor_make(SELECT_CURSOR, (self.choice_x(), self.choice_y(0)), 0xA0, 0x7F, 0xF);
        sys.lock = 0x14;
        self.keep();
        self.keep_page(sys, State::Choose);
        self.state = State::Choose;
    }

    /// État 7 : ↑ ↓ (ou la souris) choisissent (AD_AdvSelectWinSelChange :
    /// la réponse choisie passe au style 3, l'autre au style 4, le curseur
    /// glisse en 6 images) ; ○ (ou un clic sur une réponse) répond : la
    /// réponse va dans le drapeau du choix et dans l'historique. Sans
    /// mouvement, □, ✕ et L1 (AD_AdvSubProc).
    fn choose(&mut self, sys: &mut Sys, gfx: &mut Gfx, font: &Font, atlas: &mut Atlas) {
        let n = self.choices.len();
        if n == 0 {
            self.state = State::Next;
            return;
        }
        if self.hidden {
            self.sub_keys(sys, font, atlas, gfx);
            return;
        }
        let before = self.choice;
        let zones: Vec<_> = (0..n).map(|k| Zone::centered(320.0, self.choice_y(k) as f32, self.choice_w() as f32, 41.0, k)).collect();
        let Pick { hovered, clicked } = pointer::pick(sys, &zones);
        if let Some(k) = hovered {
            self.choice = k;
        }
        if sys.input.direct(0, 0x14, 7, true) {
            self.choice = (self.choice + n - 1) % n;
        } else if sys.input.direct(1, 0x14, 7, true) {
            self.choice = (self.choice + 1) % n;
        }
        if self.choice != before {
            self.windows.act(SELECT[self.choice], &[Action { dur: 1, kind: 1, frame: 100, back: 100, style: 3, ..Action::READY }]);
            self.windows.act(SELECT[before], &[Action { dur: 1, kind: 1, frame: 0x7F, back: 0x7F, style: 4, ..Action::READY }]);
            sys.menu_sound(1);
            self.windows.cursor_move(SELECT_CURSOR, (self.choice_x(), self.choice_y(self.choice)), 0x80, 6);
            if clicked.is_none() {
                return;
            }
        }
        if sys.menu_key(0x1F) || clicked.is_some() {
            sys.stop_voice();
            let flag = self.world.menu_flag.clone();
            self.world.flags.set(&flag, self.choice as i16, false);
            self.read.write(2, &self.place, self.choice as i32);
            self.log_choice(gfx, font, atlas);
            self.windows.cursor_del(SELECT_CURSOR, 5);
            sys.menu_sound(2);
            for name in &SELECT[..n] {
                self.windows.act(name, &[Action { dur: 5, kind: 3, ..Action::READY }]);
            }
            self.choices.clear();
            self.state = State::Next;
        } else {
            self.sub_keys(sys, font, atlas, gfx);
        }
    }

    /// AD_AdvSelectTxtSet : le choix dans l'historique, « → » devant la
    /// réponse choisie, dans une fenêtre de style 3 à sa taille.
    fn log_choice(&mut self, gfx: &mut Gfx, font: &Font, atlas: &mut Atlas) {
        let mut text = Vec::new();
        for (k, c) in self.choices.iter().enumerate() {
            // « → », « 　 » (0x363ff8, 0x364000).
            text.extend_from_slice(if k == self.choice { b"\x81\xa8" } else { b"\x81\x40" });
            text.extend_from_slice(c);
            text.extend_from_slice(b"#n");
        }
        let glyphs = font::typeset(font, atlas, gfx, &text, 0.0);
        let width = (f64::from(self.choice_len + 2) * 13.75 + 15.0) as i32;
        self.backlog.push(backlog::Entry { glyphs, width, height: self.choices.len() as i32 * 0x1E + 0x14, voice: None, style: 3, rewind: None }, false);
    }

    /// La place d'une réponse : centrée sur l'écran, 0x2c + n pixels entre
    /// deux (n réponses).
    fn choice_y(&self, k: usize) -> i32 {
        let n = self.choices.len() as i32;
        n * -0x1F + 0xE8 + ((n * n) >> 1) + (n + 0x2C) * k as i32
    }

    /// La largeur des réponses : celle de la plus longue (12,5 × octets +
    /// 39,5).
    fn choice_w(&self) -> i32 {
        (f64::from(self.choice_len) * 12.5 + 40.0 - 12.5 + 12.0) as i32
    }

    /// Le curseur, à gauche de la plus longue réponse.
    fn choice_x(&self) -> i32 {
        0x11D - self.choice_len * 6
    }
    pub fn draw(&self, gfx: &mut Gfx, sys: &Sys, atlas: Option<&Atlas>) {
        // Le choix du personnage se dessine à la fin de l'arrivée de son
        // fond (AD_GPTranModeSet(0xd) : 10 images).
        if let Some(s) = &self.session
            && !(self.state == State::MinigameIn && self.count < minigame::ARRIVAL)
        {
            s.draw(gfx, sys);
            return;
        }
        // Pendant l'arrivée de la carte, la scène reste dessinée dessous ;
        // l'écran de la répartition se dessine 10 images après son arrivée.
        let share = self.share.as_ref().filter(|_| !(self.state == State::DistributeIn && self.count < 10));
        match (&self.map, &self.result, share) {
            (Some(map), _, _) if self.state != State::MapIn => map.draw(gfx, &self.world),
            (_, Some(levels), _) => result::draw(gfx, levels),
            (_, _, Some(s)) => result::draw_share(gfx, s.cell, s.done),
            _ => self.scene.draw(gfx),
        }
        self.windows.draw(gfx, atlas);
        self.backlog.draw(gfx, atlas);
        if matches!(self.state, State::Menu | State::MenuAsk) {
            // Cursor_Tran : le curseur glisse d'une entrée à l'autre (6
            // images) en x = 0xd0, à la hauteur de l'entrée ; il paraît en 15.
            let (choice, before, t, appear) = self.menu;
            let y0 = menu_y(before) as f32;
            let y1 = menu_y(choice) as f32;
            let y = y0 + (y1 - y0) * t as f32 / 6.0;
            gfx.tran_center(Some(crate::dialog::CURSOR), 1.0, [7.0, 0.0, 41.0, 34.0], [208.0, y.trunc(), 41.0, 34.0], 0xDC, appear * 0x7F / 0xF, GREY, 0.0, None);
        }
        if !self.sub_state() {
            self.readmode.draw(gfx, &sys.settings);
        }
        if self.icon >= 0 && !self.hidden {
            // AD_AdvEnterTran : le signe monte et descend, et clignote.
            let t = self.icon as f32;
            let y = 0x195 as f32 + ((t * 2.0).to_radians().sin() * 10.0).trunc();
            let alpha = ((t * 4.0).to_radians().sin() * 128.0) as i32;
            gfx.tran_center(Some(ICON), 1.0, [213.0, 1.0, 31.0, 31.0], [0x216 as f32, y, 31.0, 31.0], 0x47, alpha, GREY, 0.0, None);
        }
    }
}

/// strtol(texte, NULL, 10) : les blancs du début, un signe, des chiffres,
/// jusqu'au premier octet nul.
fn strtol(text: &[u8]) -> i32 {
    let text = &text[..text.iter().position(|&c| c == 0).unwrap_or(text.len())];
    let start = text.iter().position(|c| !c.is_ascii_whitespace()).unwrap_or(text.len());
    let (negative, digits) = match text.get(start) {
        Some(b'-') => (true, &text[start + 1..]),
        Some(b'+') => (false, &text[start + 1..]),
        _ => (false, &text[start..]),
    };
    let n = digits.iter().take_while(|c| c.is_ascii_digit()).fold(0i32, |n, &c| n.wrapping_mul(10).wrapping_add(i32::from(c - b'0')));
    if negative { n.wrapping_neg() } else { n }
}
