//! Le menu de la chambre de Sakura (adv_mainmenu) et ce qu'il lance : le
//! menu des mini-jeux (cas 3), le mode challenge et son menu (cas 4), puis
//! pour chaque partie la musique, le choix du personnage
//! (AD_MiniGameStProc), le tutoriel (sauf en challenge), le jeu et le noir
//! (cas 5 et 6), avant de revenir au menu.

use std::collections::HashMap;

use crate::data::{self, Uni};
use crate::engine::{BUTTONS, Button};
use crate::sys::Sys;
use crate::font::{Atlas, Font, Glyph, Message, MESSAGES};
use crate::gfx::{Blend, Gfx, GREY};
use crate::map;
use crate::mcs;
use crate::result;
use crate::title;
use crate::mini::GAMES;
use crate::minigame::{self, Session, SEL, SEL_BG, tutorial};
use crate::original::Iso;
use crate::pointer::{self, Zone};
use crate::render::Renderer;
use crate::dialog::{self, Windows};
use crate::{bat, card, catch, mogura, run};

/// Où en est l'enchaînement.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    /// La chambre apparaît (état 0) : depuis le noir (`fade`), ou en
    /// revenant du menu des mini-jeux.
    RoomIn { fade: bool },
    /// La chambre (état 1) : une entrée choisie part (sous-états 10 et
    /// 0x14) ; « retour au titre » pose sa question (0x1e), qui ramène au
    /// titre (0x28).
    Room,
    RoomGo,
    RoomAsk,
    RoomLeave,
    /// Les options, sur la chambre (état 8) : le jeu les mène.
    Options,
    /// Du menu des mini-jeux à la chambre (état 3, sous-état 0x14).
    ToRoom,
    /// Un menu apparaît (sa musique, le noir s'efface).
    MenuIn,
    Menu,
    /// L'explication du mode challenge apparaît, reste, s'efface.
    ExplainIn,
    Explain,
    ExplainOut,
    /// Le passage au menu du challenge (ou retour au menu des mini-jeux).
    Switch,
    /// Le noir venu du menu.
    Enter,
    /// Le noir avant le scénario bonus (état 4, sous-état 0x28).
    BonusOut,
    /// Une partie de mini-jeu.
    Session,
    /// Au challenge : un message attend ○ (état 8 du cas 4).
    Inform,
    /// Au challenge : quitter ? (états 0x14 et 0x1e du cas 4).
    Ask,
    /// Les fins vues (état 7) : ○ ou ✕ ramènent à la chambre en 10 images.
    Endings,
    EndingsOut,
}

/// Ce qui est dessiné (AD_GPTranModeSet2).
#[derive(Clone, Copy, PartialEq)]
enum View {
    Nothing,
    Room,
    Menu,
    /// L'explication du challenge, à cette opacité (0 à 1).
    Explain(f32),
    Session,
    Endings,
}

/// Une phrase du jeu : ses glyphes placés, et sa taille (colonnes en
/// octets, lignes) qui fait celle de sa fenêtre.
struct Phrase {
    glyphs: Vec<Glyph>,
    size: (i32, i32),
}

/// Les entrées des menus des mini-jeux : un mini-jeu, l'explication du
/// challenge, le scénario bonus (il se débloque dans l'histoire), le
/// challenge, le retour.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Entry {
    Game(i32),
    Explain,
    Bonus,
    Challenge,
    Back,
}

/// Ce que demande la chambre au jeu.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Leave {
    NewGame,
    Bonus,
    Load,
    Album,
    Options,
    Endings,
    Title,
}

/// Ce que la chambre sait de la sauvegarde : les mini-jeux déjà joués dans
/// l'histoire, le challenge ouvert, le scénario bonus ouvert.
#[derive(Clone, Copy, Default)]
pub struct Unlocks {
    pub played: [bool; 5],
    pub challenge: bool,
    pub bonus: bool,
}

/// Les fins vues (ending_check_flag_set : six drapeaux spéciaux), et
/// l'image de leurs vignettes (ending_check_bg_set, « end_gp »), chargée.
#[derive(Clone, Default)]
pub struct Endings {
    pub seen: [bool; 6],
    pub sheet: Option<String>,
}

pub struct MainMenu {
    stage: Stage,
    pub endings: Endings,
    /// L'entrée choisie de la chambre (0x1da755e, 0 à 7 sauf 2), le
    /// personnage qui tient compagnie et sa bulle (AD_MainmenuCharaSet), les
    /// lignes de ses bulles (chara_txt_set : personnage, bulle, ligne),
    /// ce qui est débloqué, et une demande au jeu.
    room: usize,
    chara: (usize, usize),
    bubbles: Vec<Vec<Glyph>>,
    pub unlocks: Unlocks,
    leave: Option<Leave>,
    view: View,
    session: Option<Session>,
    /// Le menu du challenge (sinon celui des mini-jeux).
    challenge: bool,
    /// Les points de chaque mini-jeu du challenge en cours (-1 : pas encore
    /// joué ; 0x1c27ecd).
    scores: [i32; 5],
    /// L'entrée choisie du menu, et le curseur (Cursor_Move) : départ,
    /// arrivée, image et durée du glissement.
    entry: usize,
    cursor: ((f32, f32), (f32, f32), i32, i32),
    /// Une entrée survolée par la souris, choisie à la prochaine image.
    hovered: Option<usize>,
    /// Les fenêtres du challenge, leurs phrases.
    windows: Windows,
    texts: HashMap<Message, Phrase>,
    /// Les cinq jeux du challenge en cours sont faits (0x1c27ed4).
    finished: bool,
    /// Le challenge a déjà été réussi une fois (gardé dans la sauvegarde) :
    /// son message ne revient plus. Et s'il vient de l'être.
    cleared: bool,
    newly_cleared: bool,
    /// Le message à ouvrir au retour au menu.
    notice: Option<Message>,
}

impl MainMenu {
    /// Les phrases du challenge, dans l'exécutable, et leurs glyphes dans
    /// l'atlas.
    /// Les lignes des bulles de la chambre : `chara_txt_set(personnage,
    /// bulle, ligne, texte)` du script 4.
    pub fn new(font: Option<&Font>, atlas: &mut Atlas, cleared: bool, chara_texts: &[(i32, i32, i32, Vec<u8>)]) -> MainMenu {
        let texts: HashMap<Message, Phrase> = font
            .map(|f| MESSAGES.iter().map(|&m| (m, Phrase { glyphs: f.layout(f.message(m)), size: Font::measure(f.message(m)) })).collect())
            .unwrap_or_default();
        if let Some(f) = font {
            // Chaque glyphe dans la palette du texte (0) et dans celle de
            // l'ombre (8).
            let wanted: Vec<(u16, u8)> = texts.values().flat_map(|p| &p.glyphs).flat_map(|g| [(g.code, 0), (g.code, 8)]).collect();
            atlas.add(f, &wanted);
        }
        // AD_MainmenuCharaTxtWinMake : « #Scale[0.9] » puis les trois lignes,
        // en palette 6.
        let bubbles: Vec<Vec<Glyph>> = (0..10)
            .map(|n| {
                let Some(f) = font else { return Vec::new() };
                let mut text = b"#Scale[0.9]".to_vec();
                for line in 0..3 {
                    if let Some((_, _, _, t)) = chara_texts.iter().find(|c| (c.0 * 2 + c.1) as usize == n && c.2 == line) {
                        text.extend_from_slice(t);
                    }
                    text.extend_from_slice(b"#n");
                }
                let glyphs = f.layout(&text);
                let wanted: Vec<(u16, u8)> = glyphs.iter().map(|g| (g.code, 6)).collect();
                atlas.add(f, &wanted);
                glyphs
            })
            .collect();
        let mut menu = MainMenu {
            stage: Stage::RoomIn { fade: true },
            endings: Endings::default(),
            room: 0,
            chara: (0, 0),
            bubbles,
            unlocks: Unlocks::default(),
            leave: None,
            view: View::Nothing,
            session: None,
            challenge: false,
            scores: [-1; 5],
            entry: 0,
            cursor: ((0.0, 0.0), (0.0, 0.0), 1, 1),
            hovered: None,
            windows: Windows::default(),
            texts,
            finished: false,
            cleared,
            newly_cleared: false,
            notice: None,
        };
        menu.place_room_cursor();
        menu
    }

    /// Vrai une fois, quand le challenge vient d'être réussi pour la
    /// première fois (à garder dans la sauvegarde).
    pub fn take_cleared(&mut self) -> bool {
        std::mem::take(&mut self.newly_cleared)
    }

    fn glyphs(&self, m: Message) -> Vec<Glyph> {
        self.texts.get(&m).map(|p| p.glyphs.clone()).unwrap_or_default()
    }

    /// Les entrées du menu en cours.
    fn entries(&self) -> Vec<Entry> {
        let mut e: Vec<Entry> = (0..5).map(Entry::Game).collect();
        if self.challenge {
            e.extend([Entry::Explain, Entry::Bonus, Entry::Back]);
        } else {
            e.extend([Entry::Challenge, Entry::Back]);
        }
        e
    }

    /// Un mini-jeu ou le challenge du menu des mini-jeux se lance-t-il ?
    /// (AD_MiniGamePlayCheck, AD_MinigameChallengeOpenCheck.)
    fn open(&self, e: Entry) -> bool {
        match e {
            Entry::Game(k) => self.unlocks.played[k as usize],
            Entry::Challenge => self.unlocks.challenge,
            Entry::Bonus => self.unlocks.bonus,
            _ => true,
        }
    }

    /// Le curseur de la chambre (x = 0x15e) sur son entrée, l'entrée 2
    /// n'existant pas.
    fn room_cursor(&self) -> (f32, f32) {
        let k = self.room as i32;
        (0x15E as f32, (0x7E + 0x28 * if k < 2 { k } else { k - 1 }) as f32)
    }

    fn place_room_cursor(&mut self) {
        let at = self.room_cursor();
        self.cursor = (at, at, 1, 1);
    }

    /// Cursor_Move : le curseur glisse en 5 images depuis où il en est.
    fn glide(&mut self, to: (f32, f32)) {
        let now = self.cursor_at();
        self.cursor = (now, to, 0, 5);
    }

    fn cursor_at(&self) -> (f32, f32) {
        let (from, to, t, dur) = self.cursor;
        let k = t as f32 / dur as f32;
        (from.0 + (to.0 - from.0) * k, from.1 + (to.1 - from.1) * k)
    }

    /// Le personnage de la chambre et sa bulle, tirés d'un nombre
    /// (AD_MainmenuCharaSet : n % 5, n & 1).
    pub fn set_chara(&mut self, n: i32) {
        let n = n.unsigned_abs() as usize;
        self.chara = (n % 5, n & 1);
    }

    /// La demande de la chambre au jeu, à prendre.
    pub fn take_leave(&mut self) -> Option<Leave> {
        self.leave.take()
    }

    /// De retour d'un écran lancé par la chambre (sans noir : celui-ci
    /// s'efface).
    pub fn back_to_room(&mut self) {
        self.stage = Stage::RoomIn { fade: true };
    }

    /// Les options refermées (état 0, sous-état 0x14) : la bulle revient,
    /// le curseur reparaît sur la première entrée.
    pub fn back_from_options(&mut self) {
        self.room = 0;
        self.place_room_cursor();
        self.stage = Stage::RoomIn { fade: false };
    }

    /// Le curseur posé sur une entrée, sans glisser.
    fn place_cursor(&mut self, k: usize) {
        self.entry = k;
        let at = (0x5F as f32, self.entry_top(k) + 16.0);
        self.cursor = (at, at, 1, 1);
    }

    /// Une image (1/60 s) avec les boutons tenus, un clic de souris et la
    /// souris si elle a bougé (points de l'écran). Au menu, l'entrée sous la
    /// souris est choisie ; un clic lance celle qu'il touche. Au choix du
    /// personnage, un clic sur un autre portrait le choisit. Sinon un clic
    /// vaut ○.
    pub fn step(&mut self, sys: &mut Sys, mut buttons: [bool; BUTTONS], click: Option<(f32, f32)>) {
        // La souris sur les entrées : la survolée devient la choisie, la
        // cliquée part.
        let clicked = self.mouse(sys);
        match (self.stage, clicked) {
            (Stage::Room, Some(k)) => self.room = k,
            (Stage::Menu, Some(k)) => self.entry = k,
            _ => {}
        }
        // Pendant la course, Espace saute (↑) et Ctrl glisse (↓) ; ailleurs,
        // ces deux touches ne font rien.
        if self.session.as_ref().is_some_and(|s| s.running(sys)) {
            buttons[Button::Up as usize] |= buttons[Button::Jump as usize];
            buttons[Button::Down as usize] |= buttons[Button::Slide as usize];
        }
        // Un clic vaut ○ sur une entrée, et là où le jeu attend n'importe
        // quelle touche ; pas à côté des entrées, ni pendant une question
        // (elle a sa souris). Au choix du personnage, un clic sur un autre
        // portrait le choisit.
        let entries = matches!(self.stage, Stage::Room | Stage::Menu);
        let question = matches!(self.stage, Stage::Ask | Stage::RoomAsk);
        if click.is_some()
            && !question
            && (!entries || clicked.is_some())
            && !self.session.as_mut().is_some_and(|s| s.pick(sys))
        {
            buttons[Button::Circle as usize] = true;
        }
        sys.input.update(buttons);
        let countdown = |sys: &mut Sys| {
            if sys.lock < 0 {
                return true;
            }
            sys.lock -= 1;
            false
        };
        if self.cursor.2 < self.cursor.3 {
            self.cursor.2 += 1;
        }
        match self.stage {
            Stage::RoomIn { fade } => {
                // adv_mainmenu, état 0 : la bulle, le curseur sur la première
                // entrée ; depuis le noir, la musique et le noir qui
                // s'efface.
                self.room = 0;
                if fade {
                    sys.play_music(MENU_MUSIC);
                    sys.blackout.end(&mut sys.effects, 10);
                    sys.lock = 10;
                    self.place_room_cursor();
                } else {
                    sys.lock = 0xF;
                    let to = self.room_cursor();
                    self.glide(to);
                }
                self.view = View::Room;
                self.stage = Stage::Room;
            }
            Stage::Room => self.room(sys),
            Stage::RoomGo => {
                if countdown(sys) {
                    self.room_go(sys);
                }
            }
            Stage::RoomAsk => match self.windows.answer(sys) {
                Some(0) => {
                    sys.blackout.start(&mut sys.effects, 10, 0x7F);
                    sys.dim.end(&mut sys.effects, 15);
                    sys.lock = 10;
                    self.stage = Stage::RoomLeave;
                }
                Some(_) => {
                    sys.dim.end(&mut sys.effects, 15);
                    self.stage = Stage::Room;
                }
                None => {}
            },
            Stage::RoomLeave => {
                if countdown(sys) {
                    self.leave = Some(Leave::Title);
                }
            }
            Stage::Options => {}
            // AD_AdvPartStartProc(1) : le scénario bonus ; le challenge en
            // cours est laissé.
            Stage::BonusOut => {
                if countdown(sys) {
                    self.challenge = false;
                    self.finished = false;
                    self.scores = [-1; 5];
                    self.leave = Some(Leave::Bonus);
                }
            }
            Stage::Endings => {
                let ok = sys.menu_key(0x1F) || click.is_some();
                if ok || sys.menu_key(0x1E) {
                    sys.menu_sound(if ok { 2 } else { 0 });
                    sys.lock = 10;
                    self.stage = Stage::EndingsOut;
                }
            }
            Stage::EndingsOut => {
                if countdown(sys) {
                    self.stage = Stage::RoomIn { fade: false };
                }
            }
            Stage::ToRoom => {
                if countdown(sys) {
                    self.stage = Stage::RoomIn { fade: false };
                }
            }
            Stage::MenuIn => {
                sys.play_music(MENU_MUSIC);
                self.view = View::Menu;
                sys.blackout.end(&mut sys.effects, 10);
                sys.lock = 10;
                self.stage = Stage::Menu;
                if let Some(m) = self.notice.take() {
                    // InformationWinMake : le message en 10 images, au-dessus
                    // de l'assombrissement.
                    let (glyphs, size) = self.texts.get(&m).map_or((Vec::new(), (0, 1)), |p| (p.glyphs.clone(), p.size));
                    self.windows.inform(glyphs, size, 10, 0xE6);
                    self.stage = Stage::Inform;
                }
            }
            Stage::Menu => self.menu(sys),
            Stage::Inform => {
                if sys.menu_key(0x1F) {
                    sys.dim.end(&mut sys.effects, 15);
                    self.windows.close_information(5);
                    sys.lock = 0x14;
                    self.stage = Stage::Menu;
                }
            }
            Stage::Ask => match self.windows.answer(sys) {
                // はい : le challenge est abandonné.
                Some(0) => {
                    sys.dim.end(&mut sys.effects, 15);
                    self.leave_challenge(sys);
                }
                Some(_) => {
                    sys.dim.end(&mut sys.effects, 15);
                    self.stage = Stage::Menu;
                }
                None => {}
            },
            Stage::ExplainIn => {
                self.view = View::Explain((10 - sys.lock) as f32 / 10.0);
                if sys.lock < 1 {
                    self.stage = Stage::Explain;
                } else {
                    sys.lock -= 1;
                }
            }
            Stage::Explain => {
                if sys.menu_key(0x1F) {
                    sys.lock = 10;
                    self.stage = Stage::ExplainOut;
                }
            }
            Stage::ExplainOut => {
                self.view = View::Explain(sys.lock as f32 / 10.0);
                if sys.lock < 1 {
                    self.view = View::Menu;
                    if !self.challenge {
                        self.challenge = true;
                        let first = self.scores.iter().position(|&s| s < 0).unwrap_or(5);
                        self.place_cursor(first);
                    }
                    self.stage = Stage::Menu;
                } else {
                    sys.lock -= 1;
                }
            }
            Stage::Switch => {
                if countdown(sys) {
                    sys.blackout.end(&mut sys.effects, 10);
                    self.stage = Stage::Menu;
                }
            }
            Stage::Enter => {
                if countdown(sys) {
                    let mode = if self.challenge { minigame::Mode::Challenge } else { minigame::Mode::Menu };
                    self.session = Some(Session::start(mode, sys));
                    self.view = View::Session;
                    self.stage = Stage::Session;
                }
            }
            Stage::Session => {
                if self.session.as_mut().is_some_and(|s| s.step(sys)) {
                    if self.challenge {
                        self.score(sys);
                    }
                    self.session = None;
                    self.stage = Stage::MenuIn;
                }
            }
        }
        self.windows.update();
    }

    /// Le résultat compte dans le challenge (état 5 du cas 4) ; le curseur
    /// va au premier jeu à faire. Les cinq faits : moins de 100 points, un
    /// message d'échec ; sinon, la première fois, un message de réussite ;
    /// l'un et l'autre sur l'écran assombri (pour l'échec, le jeu se trompe
    /// d'arguments et n'assombrit rien : bug corrigé, voir le README).
    fn score(&mut self, sys: &mut Sys) {
        let kind = sys.kind as usize;
        self.scores[kind] = if sys.won { sys.points } else { 0 };
        let Some(next) = self.scores.iter().position(|&s| s < 0) else {
            let total: i32 = self.scores.iter().sum();
            if total < 100 {
                self.notice = Some(Message::ChallengeFailed);
                sys.dim.start(&mut sys.effects, 10, 0x50);
            } else if !self.cleared {
                self.notice = Some(Message::ChallengeCleared);
                sys.dim.start(&mut sys.effects, 10, 0x50);
                self.cleared = true;
                self.newly_cleared = true;
            }
            self.finished = true;
            self.place_cursor(5);
            return;
        };
        self.place_cursor(next);
    }

    /// 戻る ou × au challenge : l'écran s'assombrit, la question s'ouvre
    /// (abandonner, ou rentrer une fois les cinq jeux faits).
    fn ask_leave(&mut self, sys: &mut Sys) {
        let (question, width) = if self.finished { (Message::BackToRoom, 0x13E) } else { (Message::Retire, 0x172) };
        let (question, yes, no) = (self.glyphs(question), self.glyphs(Message::Yes), self.glyphs(Message::No));
        self.windows.ask(question, yes, no, (width, 0x44));
        sys.dim.start(&mut sys.effects, 10, 0x50);
        sys.lock = 0xC;
        self.stage = Stage::Ask;
    }

    /// Les menus (adv_mainmenu, cas 3 et 4) : la croix déplace le curseur
    /// (au challenge, les jeux déjà faits sont sautés), ○ lance l'entrée,
    /// × revient du challenge.
    fn menu(&mut self, sys: &mut Sys) {
        let entries = self.entries();
        if sys.menu_key(0x1F) {
            match entries[self.entry] {
                Entry::Game(kind) if self.open(Entry::Game(kind)) && !(self.challenge && self.scores[kind as usize] >= 0) => {
                    sys.menu_sound(2);
                    sys.kind = kind;
                    sys.blackout.start(&mut sys.effects, 10, 0x7F);
                    sys.lock = 10;
                    self.stage = Stage::Enter;
                }
                Entry::Challenge | Entry::Explain if self.open(entries[self.entry]) => {
                    sys.menu_sound(2);
                    sys.lock = 10;
                    self.stage = Stage::ExplainIn;
                }
                // Le scénario bonus, ouvert (AD_AdvBonusScenarioPlayCheck) :
                // le noir en 10 images.
                Entry::Bonus if self.open(Entry::Bonus) => {
                    sys.menu_sound(2);
                    sys.blackout.start(&mut sys.effects, 10, 0x7F);
                    sys.lock = 10;
                    self.stage = Stage::BonusOut;
                }
                Entry::Back if self.challenge => {
                    sys.menu_sound(2);
                    self.ask_leave(sys);
                }
                Entry::Back => {
                    sys.menu_sound(2);
                    sys.lock = 10;
                    self.stage = Stage::ToRoom;
                }
                _ => {}
            }
            return;
        }
        if sys.menu_key(0x1E) {
            sys.menu_sound(0);
            if self.challenge {
                self.ask_leave(sys);
            } else {
                sys.lock = 10;
                self.stage = Stage::ToRoom;
            }
            return;
        }
        let before = self.entry;
        let n = entries.len();
        let playable = |k: usize| !(self.challenge && matches!(entries[k], Entry::Game(g) if self.scores[g as usize] >= 0));
        if let Some(k) = self.hovered.take() {
            if playable(k) {
                self.entry = k;
            }
        } else {
            let step = if sys.input.direct(0, 0x14, 5, true) {
                n - 1
            } else if sys.input.direct(1, 0x14, 5, true) {
                1
            } else {
                0
            };
            if step != 0 {
                let mut k = (self.entry + step) % n;
                while !playable(k) {
                    k = (k + step) % n;
                }
                self.entry = k;
            }
        }
        if self.entry != before {
            sys.menu_sound(1);
            sys.lock = 5;
            let to = (0x5F as f32, self.entry_top(self.entry) + 16.0);
            self.glide(to);
        }
    }

    /// La chambre (adv_mainmenu, état 1, sous-état 0) : ↑ ↓ choisissent
    /// (l'entrée 2 est sautée), ○ part.
    fn room(&mut self, sys: &mut Sys) {
        if sys.menu_key(0x1F) {
            sys.menu_sound(2);
            // Sous-état 10 : le noir pour ce qui quitte la chambre, la
            // question pour le retour au titre.
            match self.room {
                0..=3 => sys.blackout.start(&mut sys.effects, 10, 0x7F),
                7 => {
                    let (q, yes, no) = (self.glyphs(Message::BackToTitle), self.glyphs(Message::Yes), self.glyphs(Message::No));
                    self.windows.ask(q, yes, no, (0x148, 0x24));
                    sys.dim.start(&mut sys.effects, 10, 0x50);
                }
                _ => {}
            }
            sys.lock = 0xC;
            self.stage = Stage::RoomGo;
            return;
        }
        let before = self.room;
        if let Some(k) = self.hovered.take() {
            self.room = k;
        } else if sys.input.direct(0, 0x14, 5, true) {
            self.room = if self.room == 3 { 1 } else { (self.room + 7) % 8 };
        } else if sys.input.direct(1, 0x14, 5, true) {
            self.room = if self.room == 1 { 3 } else { (self.room + 1) % 8 };
        }
        if self.room != before {
            sys.lock = 5;
            sys.menu_sound(1);
            let to = self.room_cursor();
            self.glide(to);
        }
    }

    /// Sous-état 0x14 : l'entrée choisie part ; le personnage de la chambre
    /// change pour la prochaine fois (AD_MainmenuCharaSet).
    fn room_go(&mut self, sys: &mut Sys) {
        if self.room <= 3 {
            // Le jeu tire ce nombre du compte de la musique ; le hasard en
            // tient lieu.
            self.set_chara(sys.rng.next());
        }
        self.stage = Stage::Room;
        match self.room {
            0 => self.leave = Some(Leave::NewGame),
            1 => self.leave = Some(Leave::Load),
            3 => self.leave = Some(Leave::Album),
            4 => {
                // État 3 : le menu des mini-jeux, le curseur sur le premier.
                self.challenge = false;
                self.entry = 0;
                let to = (0x5F as f32, self.entry_top(0) + 16.0);
                self.glide(to);
                self.view = View::Menu;
                self.stage = Stage::Menu;
            }
            5 => {
                // Le curseur et la bulle s'en vont ; la chambre reste dessous.
                self.leave = Some(Leave::Options);
                self.stage = Stage::Options;
            }
            6 => {
                // État 7 : la chambre reste dessous ; le jeu note l'image des
                // fins dans l'album si elles sont toutes vues
                // (AD_EndcheckGpgLibCgRecord).
                self.leave = Some(Leave::Endings);
                self.view = View::Endings;
                self.stage = Stage::Endings;
            }
            _ => self.stage = Stage::RoomAsk,
        }
    }


    /// La chambre (AD_MainmenuTran, cas 0 et 1) : le fond, la bulle
    /// « メインメニュー » (AD_MainmenuModeTran(0)), le personnage et sa bulle
    /// (AD_MainmenuCharaTran), les entrées (AD_MainmenuMainTran), le
    /// curseur ; pendant les options, ni le texte de la bulle ni le
    /// curseur.
    fn draw_room(&self, gfx: &mut Gfx, atlas: Option<&Atlas>) {
        gfx.tran(ROOM, [0.0, 0.0, 512.0, 448.0], [0.0, 0.0, 640.0, 448.0], 0, 0x7F, Blend::Normal, GREY);
        gfx.tran_center(Some(BUBBLES), 1.0, [0.0, 124.0, 400.0, 94.0], [320.0, 50.0, 400.0, 94.0], 0xDE, 0x7F, GREY, 0.0, None);
        let (chara, bubble) = self.chara;
        let [u, v, w, h] = PORTRAITS[chara];
        gfx.tran(CHARA, [u, v, w, h], [(0xBF - (w as i32 >> 1)) as f32, 0x198 as f32 - h, w, h], 0xDD, 0x7F, Blend::Normal, GREY);
        gfx.tran(BUBBLES, [0.0, 0.0, 219.0, 121.0], [69.0, 105.0, 219.0, 121.0], 0xDD, 0x7F, Blend::Normal, GREY);
        let options = self.stage == Stage::Options;
        if let (Some(atlas), Some(glyphs), false) = (atlas, self.bubbles.get(chara * 2 + bubble), options) {
            // La fenêtre « charatxt » (centre 0xbc, 0x9e ; 0xcd × 0x5a), son
            // texte à 3, 5 du coin.
            atlas.draw(gfx, glyphs, (0xBC - (0xCD >> 1) + 3, 0x9E - (0x5A >> 1) + 5), 0xDE, 6, None, None);
        }
        for (k, place) in room_places() {
            let scale = if k == self.room { 1.05 } else { 1.0 };
            gfx.tran_center(Some(PARTS), scale, ROOM_ENTRIES[k], place, 0xDC, 0x7F, GREY, 0.0, None);
        }
        if !options {
            self.draw_cursor(gfx);
        }
    }

    /// Les fins vues (AD_MainmenuEndcheckTran) : le fond de la chambre, la
    /// bulle « エンディングチェック » (AD_MainmenuModeTran(3)), le cadre des
    /// vignettes et celui des noms, la vignette de chaque fin vue (un sixième
    /// de « end_gp », réduit) et son nom, « ??? » pour les autres, et la part
    /// des fins vues en pour cent.
    fn draw_endings(&self, gfx: &mut Gfx) {
        let tran = |gfx: &mut Gfx, texture: &str, src: [f32; 4], x: f32, y: f32, prio: i32| gfx.tran(texture, src, [x, y, src[2], src[3]], prio, 0x7F, Blend::Normal, GREY);
        gfx.tran(ROOM, [0.0, 0.0, 512.0, 448.0], [0.0, 0.0, 640.0, 448.0], 0, 0x7F, Blend::Normal, GREY);
        gfx.tran_center(Some(BUBBLES), 1.0, [1.0, 420.0, 378.0, 92.0], [320.0, 50.0, 378.0, 92.0], 0xDE, 0x7F, GREY, 0.0, None);
        tran(gfx, ENDING, [14.0, 13.0, 330.0, 234.0], (0x140 - (330 >> 1)) as f32, 98.0, 0xDC);
        gfx.tran(ENDING, [11.0, 271.0, 490.0, 101.0], [20.0, 337.0, 600.0, 101.0], 0xDC, 0x7F, Blend::Normal, GREY);
        // La place de chaque fin dans « end_gp » et sur l'écran (colonne,
        // rangée), et son nom dans « menu_endifng » (0x35d5e0).
        const CELLS: [(i32, i32); 6] = [(1, 0), (2, 1), (0, 1), (0, 0), (2, 0), (1, 1)];
        const NAMES: [[f32; 4]; 6] = [[2.0, 384.0, 254.0, 29.0], [0.0, 416.0, 232.0, 29.0], [1.0, 451.0, 255.0, 29.0], [257.0, 384.0, 255.0, 29.0], [257.0, 417.0, 255.0, 28.0], [259.0, 451.0, 206.0, 29.0]];
        const UNSEEN: [f32; 4] = [431.0, 484.0, 70.0, 27.0];
        let mut count = 0;
        for k in 0..6 {
            let name = if self.endings.seen[k] {
                let (col, row) = CELLS[k];
                if let Some(sheet) = &self.endings.sheet {
                    let src = [(col * 0xAA) as f32, (row * 0xE0) as f32, 170.0, 224.0];
                    gfx.tran(sheet, src, [(col * 0x6A + 0xA0) as f32, (row * 0x70 + 0x66) as f32, 106.0, 112.0], 0xDC, 0x7F, Blend::Normal, GREY);
                }
                count += 1;
                NAMES[k]
            } else {
                UNSEEN
            };
            let (x, y) = ((k as i32 / 3 * 0x129 + 0xAA) as f32, ((k as i32 % 3) * 32 + 0x165) as f32);
            gfx.tran_center(Some(ENDING), 1.0, name, [x, y, name[2], name[3]], 0xDD, 0x7F, GREY, 0.0, None);
        }
        tran(gfx, ENDING, [371.0, 15.0, 118.0, 79.0], 500.0, 98.0, 0xDC);
        tran(gfx, ENDING, [1.0, 479.0, 90.0, 32.0], 512.0, 108.0, 0xDD);
        tran(gfx, ENDING, [381.0, 479.0, 31.0, 31.0], 580.0, 140.0, 0xDD);
        let digit = |gfx: &mut Gfx, d: i32, x: i32| tran(gfx, ENDING, [(d * 0x1B + 0x68) as f32, 480.0, 27.0, 30.0], x as f32, 141.0, 0xDD);
        let percent = count * 100 / 6;
        if percent == 100 {
            digit(gfx, 1, 0x1F8);
            digit(gfx, 0, 0x20C);
            digit(gfx, 0, 0x228);
        } else {
            if percent > 9 {
                digit(gfx, percent / 10, 0x20A);
            }
            digit(gfx, percent % 10, 0x226);
        }
    }

    /// Le curseur en auréole (« kasol »).
    fn draw_cursor(&self, gfx: &mut Gfx) {
        let (x, y) = self.cursor_at();
        gfx.tran_center(Some(CURSOR), 1.0, [7.0, 0.0, 41.0, 34.0], [x.trunc(), y.trunc(), 41.0, 34.0], 0x15, 0x7F, GREY, 0.0, None);
    }

    /// Le challenge quitté : ses points sont effacés, retour au menu des
    /// mini-jeux (le jeu, lui, revient à la chambre de Sakura).
    fn leave_challenge(&mut self, sys: &mut Sys) {
        self.challenge = false;
        self.finished = false;
        self.scores = [-1; 5];
        // Le curseur revient sur « チャレンジモード ».
        self.place_cursor(5);
        sys.blackout.start(&mut sys.effects, 10, 0x7F);
        sys.lock = 10;
        self.stage = Stage::Switch;
    }

    /// Le rectangle d'une entrée dans « parts ».
    fn entry_src(&self, entry: Entry) -> [f32; 4] {
        if !self.open(entry) {
            return LOCKED;
        }
        match entry {
            Entry::Game(kind) => GAMES[kind as usize].1,
            Entry::Challenge => [0.0, 276.0, 229.0, 32.0],
            Entry::Explain => [2.0, 352.0, 340.0, 30.0],
            Entry::Bonus => [303.0, 78.0, 199.0, 32.0],
            Entry::Back => [237.0, 276.0, 59.0, 32.0],
        }
    }

    /// Le haut d'une entrée : la liste part de y = 110 (140 au challenge),
    /// une entrée sous l'autre, 5 pixels entre deux.
    fn entry_top(&self, k: usize) -> f32 {
        let mut y = if self.challenge { 0x8C } else { 110 };
        for e in self.entries().into_iter().take(k) {
            let h = self.entry_src(e)[3] as i32;
            y += (h >> 1) + (h >> 1) + 5;
        }
        y as f32
    }

    /// Une entrée du menu : son centre (à gauche en x = 0x78) et sa taille.
    fn entry_place(&self, k: usize, entry: Entry) -> [f32; 4] {
        let [_, _, w, h] = self.entry_src(entry);
        [((w as i32 >> 1) + 0x78) as f32, self.entry_top(k) + (h as i32 >> 1) as f32, w, h]
    }

    /// La souris sur les entrées de la chambre ou du menu : l'entrée
    /// survolée devient la choisie, un clic la lance.
    fn mouse(&mut self, sys: &mut Sys) -> Option<usize> {
        let zones: Vec<Zone<usize>> = match self.stage {
            Stage::Room => room_places().into_iter().map(|(k, [x, y, w, h])| Zone::centered(x, y, w, h, k)).collect(),
            Stage::Menu => self.entries().into_iter().enumerate().map(|(k, e)| {
                let [x, y, w, h] = self.entry_place(k, e);
                Zone::centered(x, y, w, h, k)
            }).collect(),
            _ => return None,
        };
        let pick = pointer::pick(sys, &zones);
        self.hovered = pick.hovered;
        pick.clicked
    }

    /// Les menus : le fond de la chambre, la bulle « ミニゲーム » ou
    /// « チャレンジモード » (AD_MainmenuModeTran), les entrées, la choisie
    /// agrandie (AD_MainmenuGameTran), le curseur en auréole ; au challenge,
    /// les points de chaque jeu fait et leur total sur 100
    /// (AD_MainmenuChallengTran).
    fn draw_menu(&mut self, gfx: &mut Gfx) {
        let entries = self.entries();
        gfx.tran(ROOM, [0.0, 0.0, 512.0, 448.0], [0.0, 0.0, 640.0, 448.0], 0, 0x7F, Blend::Normal, GREY);
        let bubble = if self.challenge { [1.0, 320.0, 436.0, 94.0] } else { [1.0, 222.0, 350.0, 94.0] };
        gfx.tran_center(Some(BUBBLES), 1.0, bubble, [320.0, 50.0, bubble[2], bubble[3]], 0xDE, 0x7F, GREY, 0.0, None);
        for (k, &e) in entries.iter().enumerate() {
            let src = self.entry_src(e);
            let center = self.entry_place(k, e);
            let center_y = center[1];
            let scale = if k == self.entry { 1.05 } else { 1.0 };
            gfx.tran_center(Some(PARTS), scale, src, center, 0xDC, 0x7F, GREY, 0.0, None);
            if let (true, Entry::Game(kind)) = (self.challenge, e) {
                let score = self.scores[kind as usize];
                if score >= 0 {
                    for i in 0..2 {
                        if i != 0 || score > 9 {
                            let d = if i == 0 { score / 10 % 10 } else { score % 10 };
                            gfx.tran(ENDING, [(d * 0x1B + 0x68) as f32, 480.0, 27.0, 30.0], [(i * 0x1C + 0x1FE) as f32, center_y - 15.0, 27.0, 30.0], 0xDD, 0x7F, Blend::Normal, GREY);
                        }
                    }
                }
            }
        }
        if self.challenge {
            // 合計ポイント : le total, sur 100.
            let total: i32 = self.scores.iter().filter(|&&s| s >= 0).sum();
            gfx.tran_center(Some(PARTS), 1.0, [270.0, 192.0, 171.0, 33.0], [220.0, 115.0, 171.0, 33.0], 0xDC, 0x7F, GREY, 0.0, None);
            gfx.tran_center(Some(PARTS), 1.0, [454.0, 195.0, 32.0, 32.0], [439.0, 115.0, 32.0, 32.0], 0xDD, 0x7F, GREY, 0.0, None);
            for (i, d) in [1, 0, 0].into_iter().enumerate() {
                gfx.tran(NUMBERS, [(d * 0x1B + 0xB8) as f32, 141.0, 27.0, 30.0], [((i as i32 + 4) * 0x1C + 0x15E) as f32, 100.0, 27.0, 30.0], 0xDD, 0x7F, Blend::Normal, GREY);
            }
            for i in 0..3 {
                if (i != 0 || total > 99) && (i != 1 || total > 9) {
                    let d = [total / 100 % 10, total / 10 % 10, total % 10][i as usize];
                    gfx.tran(ENDING, [(d * 0x1B + 0x68) as f32, 480.0, 27.0, 30.0], [(i * 0x1C + 0x14F) as f32, 100.0, 27.0, 30.0], 0xDD, 0x7F, Blend::Normal, GREY);
                }
            }
        }
        self.draw_cursor(gfx);
    }

    /// L'image (AD_MainmenuTran et AD_GPTran).
    pub fn draw(&mut self, gfx: &mut Gfx, sys: &Sys, atlas: Option<&Atlas>) {
        match self.view {
            View::Room => self.draw_room(gfx, atlas),
            View::Menu => self.draw_menu(gfx),
            View::Endings => self.draw_endings(gfx),
            View::Explain(k) => {
                // AD_MainmenuChallengExplanationTran, sur le menu tant qu'il
                // apparaît ou s'efface.
                if k < 1.0 {
                    self.draw_menu(gfx);
                }
                gfx.tran(EXPLAIN, [0.0, 0.0, 512.0, 448.0], [0.0, 0.0, 640.0, 448.0], 0xE0, (k * 130.0) as i32, Blend::Normal, GREY);
            }
            _ => {}
        }
        if let Some(s) = &self.session {
            s.draw(gfx, sys);
        }
        self.windows.draw(gfx, atlas);
    }
}

/// Les images des menus (SYSTEM.UNI), des mini-jeux et de leurs tutoriels
/// (MINI.UNI), chargées au démarrage.
pub fn load_images(iso: &Iso, renderer: &mut Renderer, gfx: &mut Gfx) {
    let mut load = |uni: &Option<Uni>, index: u32, name: &'static str| {
        if let Some(img) = uni.as_ref().and_then(|u| u.read(iso, index)).and_then(|d| data::art2(&d)) {
            let id = renderer.texture(img.width, img.height, &img.rgba);
            if name == card::REPEATED {
                renderer.repeat(id);
            }
            gfx.add_texture(name, id, img.width, img.height);
        }
    };
    for (kind, (path, _, _)) in GAMES.iter().enumerate() {
        let uni = Uni::open(iso, path);
        for &(name, index) in textures(kind as i32) {
            load(&uni, index, name);
        }
    }
    let system = Uni::open(iso, "UNION/SYSTEM.UNI");
    load(&system, 40, SEL);
    load(&system, 41, SEL_BG);
    for (name, index) in MENU_TEXTURES {
        load(&system, index, name);
    }
    let mini = Uni::open(iso, "UNION/MINI.UNI");
    for kind in 0..5 {
        for chara in 0..5 {
            load(&mini, kind * 10 + chara, tutorial(kind as i32, chara as i32));
        }
    }
    // La planche de l'écran de la carte mémoire, une image TIM2.
    let memory = Uni::open(iso, "UNION/MEMORY.UNI");
    if let Some(img) = memory.as_ref().and_then(|u| u.read(iso, 10)).and_then(|d| data::tim2(&d)) {
        let id = renderer.texture(img.width, img.height, &img.rgba);
        gfx.add_texture(mcs::SHEET, id, img.width, img.height);
    }
}

/// Les images d'un mini-jeu, dans son paquet.
fn textures(kind: i32) -> &'static [(&'static str, u32)] {
    match kind {
        0 => &mogura::TEXTURES,
        1 => &run::TEXTURES,
        2 => &card::TEXTURES,
        3 => &catch::TEXTURES,
        4 => &bat::TEXTURES,
        _ => &[],
    }
}

/// Les images des menus (SYSTEM.UNI) : curseur, cadres des fenêtres,
/// textes, bulles, chiffres, explication du challenge, fond de la chambre.
const CURSOR: &str = "kasol";
const PARTS: &str = "parts";
const BUBBLES: &str = "menu_hukidasi";
const ENDING: &str = "menu_endifng";
const NUMBERS: &str = "menu_num";
const EXPLAIN: &str = "menu_chall";
const ROOM: &str = "mainmenu";
const CHARA: &str = "menu_chara";
/// « ??? » : un mini-jeu pas encore joué, un challenge ou un scénario bonus
/// fermé.
const LOCKED: [f32; 4] = [303.0, 278.0, 83.0, 30.0];
/// Les entrées de la chambre dans « parts » (0x35d520 ; l'entrée 2 n'y est
/// pas).
/// Les entrées de la chambre et leur place (centre et taille) : l'une sous
/// l'autre à partir de y = 126, tous les 0x28 pixels, centrées en
/// x = 0x177 + w / 2 ; l'entrée 2 n'existe pas.
fn room_places() -> Vec<(usize, [f32; 4])> {
    let mut y = 126.0;
    let mut places = Vec::new();
    for (k, src) in ROOM_ENTRIES.iter().enumerate().filter(|&(k, _)| k != 2) {
        places.push((k, [(src[2] as i32 >> 1) as f32 + 0x177 as f32, y, src[2], src[3]]));
        y += 40.0;
    }
    places
}

const ROOM_ENTRIES: [[f32; 4]; 8] = [
    [0.0, 0.0, 142.0, 31.0],
    [150.0, 0.0, 141.0, 30.0],
    [301.0, 0.0, 143.0, 31.0],
    [0.0, 40.0, 114.0, 30.0],
    [123.0, 40.0, 143.0, 31.0],
    [274.0, 39.0, 143.0, 32.0],
    [0.0, 79.0, 174.0, 33.0],
    [235.0, 79.0, 58.0, 32.0],
];
/// Les portraits de la chambre (menu_chara, 0x35d5b0).
const PORTRAITS: [[f32; 4]; 5] = [[8.0, 28.0, 180.0, 170.0], [201.0, 23.0, 190.0, 175.0], [7.0, 232.0, 146.0, 188.0], [168.0, 232.0, 133.0, 188.0], [315.0, 220.0, 127.0, 201.0]];
const MENU_TEXTURES: [(&str, u32); 25] = [(crate::cglib::BACK, 15), (crate::cglib::ICONS, 20), (mcs::BACK, 14), (title::BACK, 10), (title::LOGO, 11), (result::C0, 50), (result::C1, 51), (result::C2, 52), (result::C3, 53), (result::PARTS, 54), (result::BACK, 55), (map::PRTS1, 30), (map::PRTS2, 31), (map::BACK, 32), (CURSOR, 1), (dialog::MAINWIN, 2), ("win2", 3), (dialog::TEXTURE, 4), (PARTS, 6), (CHARA, 7), (BUBBLES, 8), (ENDING, 9), (NUMBERS, 12), (EXPLAIN, 13), (ROOM, 16)];
/// La musique du menu : le flux 3.
const MENU_MUSIC: u32 = 3;
