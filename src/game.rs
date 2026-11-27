//! Le jeu : comme l'exécutable (PTR_adv_title), une des boucles du jeu
//! tourne à chaque image — l'écran titre (adv_title), le menu de la chambre
//! (adv_mainmenu) ou l'histoire (adv_main) — avec ce qu'elles partagent : le système, les
//! dessins, la police et son atlas, la souris.

use std::sync::Arc;

use crate::adv::{Adv, Request};
use crate::album;
use crate::engine::{BUTTONS, Button};
use crate::audio::Audio;
use crate::cglib::Library;
use crate::font::{self, Atlas, Font, Pen};
use crate::gfx::{Blend, GREY, Gfx};
use crate::mainmenu::{self, Leave, MainMenu, Unlocks};
use crate::mcs::{self, Mcs, Outcome};
use crate::movie::{self, Player};
use crate::pointer;
use crate::options::{self, Options};
use crate::original::Iso;
use crate::record::Record;
use crate::savedata::Card;
use crate::render::Renderer;
use crate::sys::Sys;
use crate::title::{self, Title};

/// Le pointeur : sa planche, sa taille, sa pointe (une fois retourné), sa
/// priorité (au-dessus de tout, fondus compris).
const POINTER_SHEET: &str = "parts";
const POINTER_SIZE: [f32; 2] = [68.0, 48.0];
const POINTER_TIP: (f32, f32) = (6.0, 5.0);
const POINTER_PRIO: i32 = 0x1000;

/// La boucle en cours.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Title,
    Menu,
    Story,
}

/// Qui a lancé la vidéo en cours, et reprend après.
#[derive(Clone, Copy, PartialEq, Eq)]
enum After {
    /// Le démarrage (main, état 0x1e) : les quatre vidéos, puis le titre.
    Boot,
    Title,
    Story,
    Album,
}

/// Les vidéos du démarrage, dans l'ordre : les logos d'Idea Factory, de
/// Design Factory, « mate », puis l'ouverture.
const BOOT_MOVIES: [&[u8]; 4] = [b"if", b"df", b"mate", b"op"];

pub struct Game {
    pub sys: Sys,
    pub gfx: Gfx,
    /// La police du jeu (les textes sont mis en page en cours de jeu) et
    /// l'atlas de ses glyphes.
    font: Option<Font>,
    atlas: Atlas,
    mode: Mode,
    title: Title,
    menu: MainMenu,
    adv: Adv,
    /// Les options, par-dessus la chambre ou l'histoire.
    options: Option<Options>,
    /// L'écran de la carte mémoire, ouvert depuis la chambre (false) ou
    /// depuis l'histoire (true), et la carte du portage.
    card_screen: Option<(Mcs, bool)>,
    /// L'écran de l'album (AD_CgLibMain), ouvert depuis la chambre.
    library: Option<Library>,
    /// La vidéo en cours (MovieTheater) : elle seule tourne et se dessine.
    movie: Option<(Player, After)>,
    /// Les vidéos du démarrage déjà lancées.
    booted: usize,
    card: Card,
    /// Le pointeur de la souris à dessiner (points de l'écran), quand elle
    /// sert.
    pub pointer: Option<(f32, f32)>,
    /// Les boutons tenus à l'image précédente.
    last_buttons: [bool; BUTTONS],
    /// Ce qui est gardé d'une partie à l'autre, tel qu'écrit dans la
    /// sauvegarde, et les mini-jeux joués dans l'histoire.
    record: Record,
    played: [bool; 5],
    /// Les réglages tels qu'écrits à la sortie des options.
    settings: options::Settings,
}

impl Game {
    /// `boot` : les vidéos du démarrage passent avant le titre.
    pub fn new(iso: Arc<Iso>, audio: Audio, renderer: &mut Renderer, seed: u64, record: Record, boot: bool) -> Game {
        let mut gfx = Gfx::default();
        mainmenu::load_images(&iso, renderer, &mut gfx);
        let font = Font::load(&iso);
        let mut atlas = Atlas::default();
        let mut adv = Adv::new(iso.clone());
        adv.boot(0);
        // Les drapeaux spéciaux gardés reprennent leur valeur.
        for (name, value) in &record.special {
            adv.world.flags.set(name, *value, true);
        }
        let clear = adv.world.challenge_clear.clone();
        if record.legacy_cleared {
            adv.world.flags.set(&clear, 1, true);
        }
        let cleared = adv.world.flags.get(&clear, true) > 0;
        adv.read = record.read.clone();
        adv.read_kept = record.read.clone();
        // L'album gardé remplace celui des scripts (comme au chargement des
        // données système).
        if !record.album.cgs.is_empty() {
            // Les places qui lui manqueraient (« op », absente des
            // sauvegardes d'avant) s'ajoutent, pas encore vues.
            adv.album = record.album.clone();
            adv.album.register(&adv.world);
            adv.album_kept = adv.album.clone();
        }
        let menu = MainMenu::new(font.as_ref(), &mut atlas, cleared, &adv.world.chara_texts);
        gfx.load(font::TEXTURE, atlas.image());
        let mut sys = Sys::new(iso, audio, seed, record.settings);
        let title = Title::new(&mut sys);
        let mut game = Game {
            adv,
            sys,
            title,
            gfx,
            font,
            atlas,
            mode: Mode::Title,
            menu,
            options: None,
            card_screen: None,
            library: None,
            movie: None,
            booted: 0,
            card: record.card.clone(),
            pointer: None,
            last_buttons: [false; BUTTONS],
            played: record.played,
            settings: record.settings,
            record: Record::default(),
        };
        // Les tests (debug) : DOKURO_STORY lance l'histoire au démarrage
        // (DOKURO_STORY=<script>[:<point d'entrée>] : à cet endroit).
        if cfg!(debug_assertions)
            && let Ok(v) = std::env::var("DOKURO_STORY")
        {
            game.start_story(false);
            let (file, entry) = v.split_once(':').unwrap_or((&v, "sure1"));
            if let Ok(file) = file.parse::<i32>()
                && file > 1
            {
                game.adv.jump(file, entry.as_bytes());
            }
        } else if boot {
            game.next_boot_movie();
        }
        game
    }

    /// Une partie neuve de l'histoire, ou du scénario bonus.
    fn start_story(&mut self, bonus: bool) {
        let seed = self.sys.rng.next();
        self.adv.new_game(seed, &self.sys.settings, bonus);
        self.mode = Mode::Story;
        self.sys.blackout.end(&mut self.sys.effects, 10);
    }

    /// Ce qu'il faut garder, quand cela a changé depuis la dernière fois.
    pub fn take_record(&mut self) -> Option<Record> {
        let special: Vec<_> = self.adv.world.flags.special.iter().flatten().map(|f| (f.name.clone(), f.value)).collect();
        let r = &self.record;
        if special == r.special && self.played == r.played && self.settings == r.settings && self.adv.read_kept == r.read && self.adv.album_kept == r.album && self.card == r.card {
            return None;
        }
        let now = Record { special, played: self.played, settings: self.settings, read: self.adv.read_kept.clone(), album: self.adv.album_kept.clone(), card: self.card.clone(), legacy_cleared: false };
        self.record = now.clone();
        Some(now)
    }

    /// Une image (1/60 s) : les boutons tenus, un clic de souris, la souris
    /// si elle a bougé (points de l'écran).
    pub fn step(&mut self, mut buttons: [bool; BUTTONS], click: Option<(f32, f32)>, hover: Option<(f32, f32)>) {
        // Une touche ou un bouton appuyé reprend la main sur la souris.
        let pressed = buttons.iter().zip(self.last_buttons).any(|(&now, before)| now && !before);
        self.last_buttons = buttons;
        if pressed {
            self.sys.mouse = None;
        }
        if let Some(at) = click.or(hover) {
            self.sys.mouse = Some(at);
        }
        self.sys.click = click;
        self.sys.zones.clear();
        // AD_InputProc : la vibration de la manette ; hors de l'histoire, la
        // caméra ne tremble pas.
        if self.mode != Mode::Story {
            self.sys.rumble.add(0);
        }
        self.sys.rumble.frame(self.sys.settings.vibration == 1);
        if let Some((player, after)) = self.movie.as_mut() {
            self.sys.input.update(buttons);
            if player.step(&mut self.sys, &mut self.gfx) {
                let after = *after;
                self.movie = None;
                self.movie_done(after);
            }
            self.sys.end_frame();
            return;
        }
        // Le verrou des touches de menu baisse d'une image quand elles ont
        // été lues (0x1da756a).
        if std::mem::take(&mut self.sys.lock_asked) && self.sys.lock > -1 {
            self.sys.lock -= 1;
        }
        // Échap : le menu du jeu (□) pendant les pages de l'histoire, le
        // retour (×) partout ailleurs.
        let menu = std::mem::take(&mut buttons[Button::Menu as usize]);
        let story_page = self.mode == Mode::Story && self.adv.session().is_none() && self.options.is_none() && self.card_screen.is_none();
        buttons[if story_page { Button::Square } else { Button::Cross } as usize] |= menu;
        // A : la lecture automatique dans l'histoire, L2 ailleurs.
        if !story_page {
            buttons[Button::L2 as usize] |= std::mem::take(&mut buttons[Button::Auto as usize]);
        }
        if self.options.is_some() {
            self.step_options(buttons, click);
            self.sys.end_frame();
            return;
        }
        if self.card_screen.is_some() {
            self.sys.input.update(buttons);
            self.step_card();
            self.sys.end_frame();
            return;
        }
        // L'album, ouvert depuis la chambre ; refermé, la chambre revient
        // depuis le noir.
        if let (Some(lib), Some(font)) = (self.library.as_mut(), self.font.as_ref()) {
            self.sys.input.update(buttons);
            if self.adv.step_album(lib, &mut self.sys, font, &mut self.atlas, &mut self.gfx) {
                self.library = None;
                self.menu.back_to_room();
            } else if let Some(id) = lib.take_movie() {
                let name = album::movie_name(&self.adv.world, id).unwrap_or_default().to_vec();
                self.play_movie(&name, After::Album);
            }
            self.sys.end_frame();
            return;
        }
        match self.mode {
            Mode::Title => {
                self.sys.input.update(buttons);
                self.title.step(&mut self.sys);
                match self.title.take_leave() {
                    Some(title::Leave::Room) => {
                        self.menu.set_chara(self.title.idle());
                        self.menu.back_to_room();
                        self.mode = Mode::Menu;
                    }
                    // La vidéo d'ouverture, puis le titre revient.
                    Some(title::Leave::Opening) => self.play_movie(b"op", After::Title),
                    None => {}
                }
            }
            Mode::Menu => {
                let (world, flags) = (&self.adv.world, &self.adv.world.flags);
                self.menu.unlocks = Unlocks { played: self.played, challenge: flags.get(&world.challenge_open, true) > 0, bonus: flags.get(&world.bonus_open, true) > 0 };
                self.menu.endings.seen = self.adv.endings_seen();
                self.menu.step(&mut self.sys, buttons, click);
                if self.menu.take_cleared() {
                    // AD_MinigameChallengeClearSave.
                    let clear = self.adv.world.challenge_clear.clone();
                    self.adv.world.flags.set(&clear, 1, true);
                }
                match self.menu.take_leave() {
                    Some(Leave::NewGame) => self.start_story(false),
                    Some(Leave::Bonus) => self.start_story(true),
                    Some(Leave::Options) => self.open_options(options::Origin::Room),
                    // つづきから (adv_mainmenu, état 2) : depuis le noir.
                    Some(Leave::Load) => {
                        self.sys.blackout.end(&mut self.sys.effects, 1);
                        self.card_screen = Some((Mcs::start(mcs::Mode::Load, None), false));
                    }
                    Some(Leave::Title) => {
                        self.title = Title::new(&mut self.sys);
                        self.mode = Mode::Title;
                    }
                    // Les fins vues : leur image (chargée de son décor) et,
                    // toutes vues, l'album la note (AD_EndcheckGpgLibCgRecord).
                    Some(Leave::Endings) => {
                        self.menu.endings.sheet = self.adv.ending_sheet(&mut self.gfx);
                        if self.menu.endings.seen.iter().all(|&s| s) {
                            self.adv.record_ending_sheet();
                        }
                    }
                    // アルバム (entrée 3) : depuis le noir.
                    Some(Leave::Album) => self.library = Some(self.adv.open_album(&mut self.gfx, &mut self.sys)),
                    None => {}
                }
            }
            Mode::Story => {
                // Un mini-jeu de l'histoire : la course, les clics comme au
                // menu des mini-jeux.
                let sys = &mut self.sys;
                if let Some(s) = self.adv.session() {
                    if s.running(sys) {
                        buttons[Button::Up as usize] |= buttons[Button::Jump as usize];
                        buttons[Button::Down as usize] |= buttons[Button::Slide as usize];
                    }
                    if click.is_some() && !s.pick(sys) {
                        buttons[Button::Circle as usize] = true;
                    }
                }
                self.sys.input.update(buttons);
                self.step_story();
            }
        }
        self.sys.end_frame();
    }

    /// Une image de l'histoire ; ce qu'elle demande (mini-jeux, carte…).
    fn step_story(&mut self) {
        let Some(font) = self.font.as_ref() else { return };
        self.adv.step(&mut self.sys, &mut self.gfx, font, &mut self.atlas);
        if let Some(k) = self.adv.take_played() {
            self.played[k as usize] = true;
        }
        match self.adv.take_request() {
            // Le menu du jeu ramène à la chambre (depuis le noir) ; les
            // bruitages et les boucles se taisent.
            Some(Request::Room) => {
                self.adv.scene.clear(&mut self.sys.effects);
                self.sys.stop_music();
                self.sys.stop_sounds();
                self.sys.forget_loops();
                self.mode = Mode::Menu;
                self.menu.back_to_room();
            }
            // La fin du jeu ramène au titre (une partie commencée par
            // はじめから ou chargée : AD_AdvPartStartProc(2 ou 4), 0x1da7594 à 0),
            // ou à la chambre (le scénario bonus : 0x1da7594 à 1).
            Some(Request::GameEnd) => {
                self.adv.scene.clear(&mut self.sys.effects);
                self.sys.stop_music();
                self.sys.stop_sounds();
                self.sys.forget_loops();
                if self.adv.from_room {
                    self.mode = Mode::Menu;
                    self.menu.back_to_room();
                } else {
                    self.title = Title::new(&mut self.sys);
                    self.mode = Mode::Title;
                }
            }
            None if self.adv.finished() => {
                self.adv.scene.clear(&mut self.sys.effects);
                self.sys.stop_music();
                self.sys.stop_sounds();
                self.sys.forget_loops();
                self.mode = Mode::Menu;
                self.menu.back_to_room();
            }
            Some(Request::Options) => self.open_options(options::Origin::Story),
            // Les données système vont avec la sauvegarde : l'avance choisie
            // par R1 y est gardée.
            Some(Request::Save) => {
                self.settings.skip_mode = self.sys.settings.skip_mode;
                self.card_screen = Some((Mcs::start(mcs::Mode::Save, Some(self.adv.snapshot(&self.sys))), true));
            }
            Some(Request::Load) => self.card_screen = Some((Mcs::start(mcs::Mode::Load, None), true)),
            Some(Request::Movie(name)) => self.play_movie(&name, After::Story),
            None => {}
        }
    }

    /// AD_MVPlay : l'album note la vidéo vue, la voix se tait
    /// (AD_VoiceCancel), la musique aussi (le jeu attend qu'elle se soit
    /// tue), la vidéo passe. Sans son fichier, on reprend aussitôt.
    fn play_movie(&mut self, name: &[u8], after: After) {
        self.adv.record_movie(name, after != After::Story);
        self.sys.stop_voice();
        self.sys.pause_music();
        let player = movie::file(&self.adv.world, name).and_then(|path| Player::open(self.sys.iso(), &path));
        match player {
            Some(p) => self.movie = Some((p, after)),
            None => self.movie_done(after),
        }
    }

    /// Le démarrage (main, état 0x1e) : la vidéo suivante, chacune notée par
    /// l'album (« op » s'y débloque) ; après la dernière, le titre paraît.
    fn next_boot_movie(&mut self) {
        match BOOT_MOVIES.get(self.booted) {
            Some(name) => {
                self.booted += 1;
                self.play_movie(name, After::Boot);
            }
            None => self.title = Title::new(&mut self.sys),
        }
    }

    /// La vidéo finie, celui qui l'a lancée reprend.
    fn movie_done(&mut self, after: After) {
        match after {
            After::Boot => self.next_boot_movie(),
            After::Title => self.title.back(&mut self.sys),
            After::Story => self.adv.movie_done(&mut self.sys),
            After::Album => {
                if let Some(lib) = self.library.as_mut() {
                    lib.movie_done(&mut self.sys);
                }
            }
        }
    }

    /// Une image de l'écran de la carte mémoire ; une fois fermé, une partie
    /// chargée reprend (AD_AdvPartStartProc(4) : depuis le noir, le jeu
    /// remis à zéro), sinon la chambre revient depuis le noir, ou la page
    /// de l'histoire.
    fn step_card(&mut self) {
        let (Some((screen, from_story)), Some(font)) = (self.card_screen.as_mut(), self.font.as_ref()) else { return };
        let mut pen = Pen { font, atlas: &mut self.atlas, gfx: &mut self.gfx };
        screen.step(&mut self.sys, &mut self.card, &mut pen);
        let Some(outcome) = screen.outcome() else { return };
        let from_story = *from_story;
        let loaded = screen.take_loaded();
        self.card_screen = None;
        match (outcome, loaded) {
            (Outcome::Done, Some(g)) => {
                self.sys.blackout.start(&mut self.sys.effects, 1, 0x7F);
                self.adv.scene.clear(&mut self.sys.effects);
                let seed = self.sys.rng.next();
                self.adv.boot(seed);
                self.adv.load(&g, &mut self.sys, &mut self.gfx, font, &mut self.atlas, false);
                self.sys.blackout.end(&mut self.sys.effects, 10);
                self.mode = Mode::Story;
            }
            _ if from_story => self.adv.back_from_card(&mut self.sys),
            _ => {
                self.sys.blackout.start(&mut self.sys.effects, 1, 0x7F);
                self.menu.back_to_room();
            }
        }
    }

    /// Le pointeur de la souris (propre au portage) : l'Escalibolg de
    /// Dokuro, dans la planche « parts » (inutilisée par le jeu), retournée
    /// pour que son bout clouté pointe en haut à gauche ; par-dessus tout.
    fn draw_pointer(&mut self) {
        let Some((x, y)) = self.pointer else { return };
        let [w, h] = POINTER_SIZE;
        let (tx, ty) = POINTER_TIP;
        self.gfx.tran(POINTER_SHEET, [390.0, 449.0, w, h], [x - tx + w, y - ty, -w, h], POINTER_PRIO, 0x80, Blend::Normal, GREY);
    }

    /// Les options s'ouvrent (AD_OptionStart).
    fn open_options(&mut self, origin: options::Origin) {
        let Some(font) = self.font.as_ref() else { return };
        let mut pen = Pen { font, atlas: &mut self.atlas, gfx: &mut self.gfx };
        self.options = Some(Options::start(origin, &mut self.sys, &mut pen));
    }

    /// Une image des options : un clic sur une jauge la règle, sur une
    /// entrée ou une valeur il vaut ○ ; une fois fermées, la chambre ou le
    /// menu du jeu reprend.
    fn step_options(&mut self, mut buttons: [bool; BUTTONS], click: Option<(f32, f32)>) {
        let (Some(o), Some(font)) = (self.options.as_mut(), self.font.as_ref()) else { return };
        if let Some(at) = click
            && o.click(&mut self.sys, at)
        {
            buttons[Button::Circle as usize] = true;
        }
        self.sys.input.update(buttons);
        let mut pen = Pen { font, atlas: &mut self.atlas, gfx: &mut self.gfx };
        let done = o.step(&mut self.sys, &mut pen);
        if o.origin() == options::Origin::Story {
            // L'histoire attend (State::Options), mais ses fenêtres et ses
            // fondus vont leur train.
            self.adv.step(&mut self.sys, &mut self.gfx, font, &mut self.atlas);
        }
        if o.take_save() {
            self.settings = self.sys.settings;
        }
        if done {
            match o.origin() {
                options::Origin::Room => self.menu.back_from_options(),
                // Les bruitages en boucle reprennent (AD_LoopSERePlay).
                options::Origin::Story => {
                    self.sys.resume_loops();
                    self.adv.open_menu(&mut self.sys, font, &mut self.atlas, &mut self.gfx);
                }
            }
            self.options = None;
        }
    }

    /// L'image (AD_GPTran, puis les effets et les fondus).
    pub fn draw(&mut self) {
        if let Some((player, _)) = &self.movie {
            player.draw(&mut self.gfx);
            return;
        }
        self.draw_pointer();
        // Les tests (debug) : DOKURO_ZONES dessine les zones de la souris.
        if cfg!(debug_assertions) && std::env::var_os("DOKURO_ZONES").is_some() {
            pointer::draw(&mut self.gfx, &self.sys.zones);
        }
        if let Some((screen, _)) = &self.card_screen {
            // AD_GPTranSubModeSet(5) : seul l'écran de la carte se dessine.
            screen.draw(&mut self.gfx, &self.card, Some(&self.atlas));
            self.sys.blackout.draw(&mut self.gfx);
            return;
        }
        match self.mode {
            Mode::Title => self.title.draw(&mut self.gfx),
            Mode::Menu => match &self.library {
                Some(lib) => lib.draw(&mut self.gfx, &self.adv.album, Some(&self.atlas)),
                None => self.menu.draw(&mut self.gfx, &self.sys, Some(&self.atlas)),
            },
            Mode::Story => self.adv.draw(&mut self.gfx, &self.sys, Some(&self.atlas)),
        }
        if let Some(o) = &self.options {
            o.draw(&mut self.gfx, &self.sys, Some(&self.atlas));
        }
        self.sys.effects.draw(&mut self.gfx, &self.sys.camera);
        self.sys.dim.draw(&mut self.gfx);
        self.sys.blackout.draw(&mut self.gfx);
    }
}
