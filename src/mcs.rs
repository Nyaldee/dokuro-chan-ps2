//! L'écran de la carte mémoire (MemorycardPart, mcs*) pour sauvegarder et
//! charger une partie : douze fichiers en deux colonnes, l'en-tête du
//! fichier choisi (la date), un message en bas, les questions はい / いいえ.
//! Le portage suit le chemin d'une carte présente et initialisée ; les
//! fichiers sont dans sa sauvegarde (`savedata::Card`). Voir
//! docs/moteur.md.

use crate::font::{Atlas, Font, Glyph, Pen, sprintf};
use crate::gfx::{Blend, Gfx, GREY};
use crate::platform;
use crate::pointer::{self, Pick, Zone};
use crate::savedata::{Card, Game, SLOTS};
use crate::sys::Sys;

/// Le fond (SYSTEM.UNI 14) et la planche des sprites (MEMORY.UNI 10, une
/// image TIM2).
pub const BACK: &str = "save_bg";
pub const SHEET: &str = "mcs";
/// La priorité de tout l'écran (z de artsSprDraw, MkWindowSet).
const PRIO: i32 = 0xF5;
/// Les messages (table 0x1dfbe0, `Tools/mcs_messages.py`) dont se sert le
/// portage.
const CHECKING: usize = 0x17;
const MAKING_GAME_DATA: usize = 0x19;
const SAVING: usize = 0x1A;
const LOADING: usize = 0x1C;
const GAME_DATA_MADE: usize = 0x28;
const SAVED: usize = 0x29;
const NO_GAME_DATA_LOAD: usize = 0x33;
const NO_GAME_DATA_SAVE: usize = 0x35;
const SELECT_SAVE: usize = 0x36;
const SELECT_LOAD: usize = 0x37;
const QUIT_SAVE: usize = 0x39;
const QUIT_LOAD: usize = 0x3A;
const SAVE_NEW: usize = 0x3D;
const OVERWRITE: usize = 0x3E;
const LOAD: usize = 0x3F;
const QUIT_SAVE_AFTER: usize = 0x45;
const TABLE: usize = 0x1DFBE0;
/// Les phrases des fichiers et de l'en-tête : le préfixe des textes,
/// « NON DATA », la date, l'heure.
const PREFIX: usize = 0x362530;
const NON_DATA: usize = 0x362550;
const DATE: usize = 0x3625E0;
const HOUR: usize = 0x362600;
/// Les opérations de la carte se font aussitôt dans le portage : leur
/// message reste ce nombre d'images (le jeu attend la carte).
const BUSY: i32 = 0x1E;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Mode {
    Load,
    Save,
}

/// Où en est l'écran (uRam00365a14 et l'automate mcsm).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Stage {
    /// mcsFadeIn : le noir s'efface (0x80 à 0, 8 par image).
    FadeIn,
    /// Un message pendant une opération de la carte, puis la suite.
    Busy(usize, i32, After),
    /// Une question はい / いいえ (sorte 1).
    Ask(usize),
    /// Un message qui attend ○ (sorte 2).
    Wait(usize),
    /// mcsmSelectFile.
    Select,
    /// mcsFadeOut : le noir revient ; puis la fin.
    FadeOut(Outcome),
    Done(Outcome),
}

/// Ce qui suit une opération de la carte.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum After {
    /// La carte est vue : la question des données du jeu, ou les fichiers.
    Checked,
    /// Les en-têtes sont lus : le choix d'un fichier.
    Headers,
    GameDataMade,
    Saved,
    Loaded,
}

/// Comment l'écran s'est fermé : fait (1) ou abandonné (−1).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    Done,
    Cancelled,
}

pub struct Mcs {
    mode: Mode,
    stage: Stage,
    /// Le noir par-dessus (0x80 : noir).
    black: i32,
    /// Le fichier choisi (1 à 12), la réponse choisie (0 はい, 1 いいえ).
    slot: usize,
    answer: usize,
    /// Les fenêtres affichées : la liste des fichiers (avec curseur,
    /// numéros, commandes), l'en-tête du fichier.
    files: bool,
    header: bool,
    /// La répétition des flèches (mcsGetKey) : compte, palier.
    repeat: (i32, i32),
    /// Le message, les titres des fichiers, l'en-tête, mis en glyphes.
    message: Vec<Glyph>,
    titles: Vec<Vec<Glyph>>,
    header_text: Vec<Glyph>,
    /// La partie à sauvegarder ; la partie chargée.
    game: Option<Game>,
    loaded: Option<Game>,
}

impl Mcs {
    /// mcsInit, mcsLoadObject, mcsLoadWait : l'écran s'ouvre depuis le noir.
    pub fn start(mode: Mode, game: Option<Game>) -> Mcs {
        Mcs {
            mode,
            stage: Stage::FadeIn,
            black: 0x80,
            slot: 1,
            answer: 1,
            files: false,
            header: false,
            repeat: (0, 0),
            message: Vec::new(),
            titles: Vec::new(),
            header_text: Vec::new(),
            game,
            loaded: None,
        }
    }

    /// La fin, une fois le noir revenu.
    pub fn outcome(&self) -> Option<Outcome> {
        match self.stage {
            Stage::Done(o) => Some(o),
            _ => None,
        }
    }

    /// La partie chargée, à prendre.
    pub fn take_loaded(&mut self) -> Option<Game> {
        self.loaded.take()
    }

    /// mcsEnableWindow : un message de la table.
    fn show(&mut self, n: usize, pen: &mut Pen) {
        let text = pen.font.string(pen.font.word(TABLE + n * 0x1C) as usize).to_vec();
        self.message = pen.glyphs(&text);
        self.stage = match kind(pen.font, n) {
            1 => {
                self.answer = 1;
                Stage::Ask(n)
            }
            2 => Stage::Wait(n),
            _ => self.stage,
        };
    }

    /// Un message pendant une opération.
    fn busy(&mut self, n: usize, after: After, pen: &mut Pen) {
        self.show(n, pen);
        self.stage = Stage::Busy(n, BUSY, after);
    }

    /// Les titres des fichiers (mcsDrawFrameTitleWindow) et l'en-tête du
    /// fichier choisi (mcsPutHeaderDataWindow, mcsSetHeaderData).
    fn refresh(&mut self, card: &Card, pen: &mut Pen) {
        let prefix = pen.font.string(PREFIX).to_vec();
        self.titles = card
            .slots
            .iter()
            .map(|s| {
                let mut t = prefix.clone();
                t.extend_from_slice(s.as_ref().map_or(pen.font.string(NON_DATA), |g| &g.title));
                pen.glyphs(&t)
            })
            .collect();
        let mut t = prefix;
        if let Some(g) = &card.slots[self.slot - 1] {
            let [year, month, day, hour, minute] = g.time;
            t.extend(sprintf(pen.font.string(DATE), &[year, month, day]));
            t.push(b' ');
            t.extend(sprintf(pen.font.string(HOUR), &[hour, minute]));
            t.extend_from_slice(b"#n");
        }
        self.header_text = pen.glyphs(&t);
    }

    /// Les fichiers sont lus : leur liste s'affiche, le curseur sur le
    /// premier (SetDefaultCursor), le message du choix.
    fn open_files(&mut self, card: &Card, pen: &mut Pen) {
        self.files = true;
        self.header = true;
        self.slot = 1;
        self.refresh(card, pen);
        self.select(pen);
    }

    /// Retour au choix d'un fichier (prc_ynNextMain, cas 7).
    fn select(&mut self, pen: &mut Pen) {
        self.show(if self.mode == Mode::Save { SELECT_SAVE } else { SELECT_LOAD }, pen);
        self.stage = Stage::Select;
    }

    /// Une image (mcsMain et les états de MemorycardPart). La carte change
    /// quand une partie est sauvegardée.
    pub fn step(&mut self, sys: &mut Sys, card: &mut Card, pen: &mut Pen) {
        match self.stage {
            Stage::FadeIn => {
                self.black -= 8;
                if self.black <= 0 {
                    self.black = 0;
                    // mcsPreCheckMc : la carte est regardée.
                    self.busy(CHECKING, After::Checked, pen);
                }
            }
            Stage::Busy(n, timer, after) => {
                if timer > 0 {
                    self.stage = Stage::Busy(n, timer - 1, after);
                    return;
                }
                match after {
                    After::Checked if !card.game_data => self.show(if self.mode == Mode::Save { NO_GAME_DATA_SAVE } else { NO_GAME_DATA_LOAD }, pen),
                    // mcsmLoadHeaderList : les en-têtes un à un.
                    After::Checked => self.busy(CHECKING, After::Headers, pen),
                    After::Headers => self.open_files(card, pen),
                    After::GameDataMade => self.show(GAME_DATA_MADE, pen),
                    After::Saved => {
                        self.refresh(card, pen);
                        self.show(SAVED, pen);
                    }
                    After::Loaded => self.stage = Stage::FadeOut(Outcome::Done),
                }
            }
            Stage::Ask(n) => self.ask(n, sys, card, pen),
            Stage::Wait(n) => {
                if sys.input.key(3) || sys.click.is_some() {
                    match n {
                        GAME_DATA_MADE => self.busy(CHECKING, After::Headers, pen),
                        _ => self.stage = Stage::FadeOut(Outcome::Done),
                    }
                }
            }
            Stage::Select => self.choose(sys, card, pen),
            Stage::FadeOut(o) => {
                self.black += 8;
                if self.black >= 0x80 {
                    self.black = 0x80;
                    self.stage = Stage::Done(o);
                }
            }
            Stage::Done(_) => {}
        }
    }

    /// mcsSelectWindow et prc_ynNextMain : ↑ ↓ (ou la souris) choisissent la
    /// réponse, ○ répond, ✕ répond いいえ.
    fn ask(&mut self, n: usize, sys: &mut Sys, card: &mut Card, pen: &mut Pen) {
        let [_, _, w, h] = ANSWER_BOX;
        let zones: [Zone<usize>; 2] = std::array::from_fn(|k| Zone::new([512.0, ANSWER_Y[k] as f32, w as f32, h as f32], k));
        let Pick { hovered, clicked } = pointer::pick(sys, &zones);
        if let Some(k) = hovered {
            self.answer = k;
        }
        let keys = self.keys(sys);
        if keys & (UP | DOWN) != 0 {
            self.answer ^= 1;
            return;
        }
        let yes = if sys.input.key(3) || clicked.is_some() {
            self.answer == 0
        } else if sys.input.key(2) {
            false
        } else {
            return;
        };
        match (n, yes) {
            (NO_GAME_DATA_SAVE, true) => {
                // La création des données du jeu, avec douze fichiers vides.
                card.game_data = true;
                card.slots = Default::default();
                self.busy(MAKING_GAME_DATA, After::GameDataMade, pen);
            }
            (NO_GAME_DATA_SAVE, false) => self.show(QUIT_SAVE_AFTER, pen),
            // いいえ : la carte est regardée de nouveau.
            (NO_GAME_DATA_LOAD | QUIT_SAVE_AFTER, false) => self.busy(CHECKING, After::Checked, pen),
            (NO_GAME_DATA_LOAD | QUIT_SAVE_AFTER | QUIT_SAVE | QUIT_LOAD, true) => self.stage = Stage::FadeOut(Outcome::Cancelled),
            (SAVE_NEW | OVERWRITE, true) => {
                // mcsGetTime, McSaveFile.
                if let Some(mut g) = self.game.clone() {
                    g.time = platform::local_time();
                    card.slots[self.slot - 1] = Some(g);
                }
                self.busy(SAVING, After::Saved, pen);
            }
            (LOAD, true) => {
                self.loaded = card.slots[self.slot - 1].clone();
                self.busy(LOADING, After::Loaded, pen);
            }
            _ => self.select(pen),
        }
    }

    /// mcsmSelectFile : ↑ ↓ d'un fichier, ← → d'une colonne à l'autre (la
    /// souris choisit le fichier survolé) ; ○ choisit le fichier (un clic
    /// aussi), ✕ propose de quitter.
    fn choose(&mut self, sys: &mut Sys, card: &Card, pen: &mut Pen) {
        let before = self.slot;
        let zones: Vec<_> = (0..SLOTS).map(|k| {
            let (x, y) = slot_place(k);
            Zone::new([x as f32, y as f32, SLOT_SIZE.0 as f32, SLOT_SIZE.1 as f32], k + 1)
        }).collect();
        let Pick { hovered, clicked } = pointer::pick(sys, &zones);
        if let Some(k) = hovered {
            self.slot = k;
        }
        if sys.input.key(3) || clicked.is_some() {
            // mcsmSelectFilePushReturn : en chargement, un fichier vide ne
            // répond pas.
            let saved = card.slots[self.slot - 1].is_some();
            match (self.mode, saved) {
                (Mode::Save, true) => self.show(OVERWRITE, pen),
                (Mode::Save, false) => self.show(SAVE_NEW, pen),
                (Mode::Load, true) => self.show(LOAD, pen),
                (Mode::Load, false) => {}
            }
            return;
        }
        if sys.input.key(2) {
            self.show(if self.mode == Mode::Save { QUIT_SAVE } else { QUIT_LOAD }, pen);
            return;
        }
        let keys = self.keys(sys);
        if keys & UP != 0 {
            self.slot = if self.slot == 1 { SLOTS } else { self.slot - 1 };
        } else if keys & DOWN != 0 {
            self.slot = self.slot % SLOTS + 1;
        } else if keys & (LEFT | RIGHT) != 0 {
            self.slot = if self.slot <= 6 { self.slot + 6 } else { self.slot - 6 };
        }
        if self.slot != before {
            self.refresh(card, pen);
        }
    }

    /// mcsGetKey : une flèche, à l'appui, puis après 0x14 images, puis
    /// toutes les 8 (la première tenue l'emporte : ↑, ↓, ←, →).
    fn keys(&mut self, sys: &Sys) -> u32 {
        let held = [UP, DOWN, LEFT, RIGHT].into_iter().zip(0..).find(|&(_, d)| sys.input.direct(d, 0, 1, false));
        let Some((bit, _)) = held else {
            self.repeat = (0, 0);
            return 0;
        };
        let (count, step) = &mut self.repeat;
        *count += 1;
        let fire = match *step {
            0 => true,
            1 => *count > 0x14,
            _ => *count > 8,
        };
        if !fire {
            return 0;
        }
        *step = (*step + 1).min(2);
        *count = 0;
        bit
    }

    /// mcsDraw (le fond, le cadre du message, de l'en-tête, le mode, les
    /// fenêtres, les numéros, le curseur, les commandes), puis le noir.
    pub fn draw(&self, gfx: &mut Gfx, card: &Card, atlas: Option<&Atlas>) {
        gfx.tran(BACK, [0.0, 0.0, 512.0, 448.0], [0.0, 0.0, 640.0, 448.0], PRIO, 0x80, Blend::Normal, GREY);
        let sprite = |gfx: &mut Gfx, src: [i32; 4], x: i32, y: i32| {
            gfx.tran(SHEET, src.map(|v| v as f32), [x as f32, y as f32, src[2] as f32, src[3] as f32], PRIO, 0x80, Blend::Normal, GREY);
        };
        sprite(gfx, [0xE, 400, 0x1E7, 0x6B], 18, 325);
        if self.files {
            sprite(gfx, [9, 0xA5, 0x11C, 0x42], 23, 249);
        }
        let mode = if self.mode == Mode::Save { [0x16A, 6, 0x5F, 0x20] } else { [0x16A, 0x33, 0x65, 0x1F] };
        sprite(gfx, mode, 500, 12);
        // mcsWindow : la question, le message, l'en-tête, les fichiers.
        let text = |gfx: &mut Gfx, glyphs: &[Glyph], x: i32, y: i32| {
            if let Some(atlas) = atlas {
                atlas.draw(gfx, glyphs, (x, y), PRIO + 1, 0, None, None);
            }
        };
        if let Stage::Ask(_) = self.stage {
            sprite(gfx, ANSWER_BOX, 512, ANSWER_Y[0]);
            sprite(gfx, [0xC, 0x161, 0x35, 0x1A], 545, ANSWER_Y[0]);
            sprite(gfx, ANSWER_BOX, 512, ANSWER_Y[1]);
            sprite(gfx, [0x54, 0x161, 0x4B, 0x1C], 535, ANSWER_Y[1]);
            let y = if self.answer == 0 { 330 } else { 368 };
            sprite(gfx, [0xB, 0x124, 0xF, 0x2A], 505, y);
            sprite(gfx, [0x12A, 0x123, 0xF, 0x2A], 625, y);
        }
        text(gfx, &self.message, 37, 330);
        if self.header {
            text(gfx, &self.header_text, 39, 257);
        }
        if !self.files {
            return self.draw_black(gfx);
        }
        // mcsDrawSaveFrame et les titres : en chargement, un fichier vide est
        // sombre.
        for k in 0..SLOTS {
            let (col, row) = ((k / 6) as i32, (k % 6) as i32);
            let (x, y) = slot_place(k);
            let dark = self.mode == Mode::Load && card.slots[k].is_none();
            sprite(gfx, [8, if dark { 0x30 } else { 8 }, SLOT_SIZE.0, SLOT_SIZE.1], x, y);
            if let Some(t) = self.titles.get(k) {
                text(gfx, t, x + 0x28, y + 4);
            }
            sprite(gfx, [0x16A + col * 0x2D, 0xD0 + row * 0x1F, 0x2D, 0x1F], col * 0x130 + 0x19, row * 0x21 + 0x32);
        }
        let k = (self.slot - 1) as i32;
        sprite(gfx, [0xB, 0x124, 300, 0x27], (k / 6) * 0x131 + 0xF, (k % 6) * 0x21 + 0x2D);
        sprite(gfx, [0xAC, 0xF3, 0x76, 0x1C], 340, 270);
        sprite(gfx, [0x169, 100, 0x5D, 0x1F], 352, 270);
        sprite(gfx, [0xAC, 0xF3, 0x76, 0x1C], 487, 270);
        sprite(gfx, [0x169, 0xAC, 0x5D, 0x1F], 499, 270);
        self.draw_black(gfx);
    }

    /// Le noir des fondus d'entrée et de sortie, par-dessus tout.
    fn draw_black(&self, gfx: &mut Gfx) {
        if self.black > 0 {
            gfx.paint([0.0, 0.0, 640.0, 448.0], 500, self.black, [0; 3]);
        }
    }
}

/// Les flèches de mcsGetKey.
const UP: u32 = 1;
const DOWN: u32 = 2;
const LEFT: u32 = 4;
const RIGHT: u32 = 8;

/// La sorte d'un message (0 rien à attendre, 1 question, 2 attendre ○, 3
/// une opération).
fn kind(font: &Font, n: usize) -> u32 {
    font.word(TABLE + n * 0x1C + 4)
}

/// La taille du cadre d'un fichier.
const SLOT_SIZE: (i32, i32) = (0x120, 0x1E);

/// La place du cadre d'un fichier (0 à 11, six par colonne).
fn slot_place(k: usize) -> (i32, i32) {
    let (col, row) = ((k / 6) as i32, (k % 6) as i32);
    (col * 0x131 + 0x14, row * 0x21 + 0x32)
}

/// Le cadre d'une réponse (はい en haut, いいえ en bas), en x = 512.
const ANSWER_BOX: [i32; 4] = [0xAC, 0xF3, 0x76, 0x1C];
const ANSWER_Y: [i32; 2] = [335, 375];

