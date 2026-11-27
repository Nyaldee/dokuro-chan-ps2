//! L'écran de l'album (AD_CgLibMain, AD_CgLibKasolProc, AD_CgLibTran),
//! ouvert par l'entrée 3 de la chambre. Sept pages d'onglets à gauche,
//! chacune de seize vignettes par feuille (jusqu'à quatre feuilles,
//! numérotées en bas), « 戻る » sous les onglets ; le cadre « cursor »
//! (Cursor_Tran5) va de l'un à l'autre. Une vignette vue s'ouvre en grand
//! (les vidéos, pas encore portées, ne s'ouvrent pas). Voir
//! docs/moteur.md.

use crate::album::{self, Album};
use crate::dialog::{Action, Windows};
use crate::font::{self, Atlas, Font};
use crate::gfx::{Blend, Gfx, GREY};
use crate::pointer::{self, Pick, Zone};
use crate::story::World;
use crate::sys::Sys;

/// Le fond de l'album et ses onglets (SYSTEM.UNI 15 et 20 : AD_BGMake,
/// AD_GpEntry dans le code du jeu, pas dans les scripts).
pub const BACK: &str = "cglib";
pub const ICONS: &str = "cgicon";
/// Les pages de l'album, les vignettes d'une feuille.
const PAGES: usize = 7;
const PER_SHEET: usize = 16;
/// La fenêtre du pourcentage et son texte (« #Speed[0]達成度　%3d％ »).
const WINDOW: &str = "cg";
const PERCENT: usize = 0x3638B0;
/// Les priorités des onglets et vignettes, du cadre.
const PRIO: i32 = 0xA0;
const FRAME_PRIO: i32 = 0xDC;


/// Une place de l'album : image ou vidéo, son numéro, sa vignette (planche
/// et rectangle).
#[derive(Clone)]
struct Slot {
    movie: bool,
    id: i16,
    sheet: Option<String>,
    rect: [f32; 4],
}

/// Où est le curseur (cRam01da7311) : 0 les vignettes, 4 les onglets, 5
/// les feuilles, 6 « 戻る ». Les autres sous-états de l'album sont des
/// étapes (`Stage`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum At {
    Thumbs,
    Pages,
    Sheets,
    Back,
}

/// La place (centre, taille) de ce que vise le curseur : une vignette
/// (colonne, rangée), un onglet (sa rangée), une feuille (sa colonne),
/// « 戻る ».
fn place((at, col, row): (At, i32, i32)) -> [f32; 4] {
    match at {
        At::Thumbs => [(col * 0x73 + 0xD2) as f32, (row * 0x4B + 0x70) as f32, 90.0, 63.0],
        At::Pages => [80.0, (row * 40 + 0x3C) as f32, 120.0, 30.0],
        At::Sheets => [(col * 100 + 0xF0) as f32, 410.0, 45.0, 30.0],
        At::Back => [80.0, 400.0, 120.0, 30.0],
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    /// L'image bonus (AD_CgBonusGetJudg(2)) : elle attend ○, puis le noir.
    Bonus,
    BonusOut,
    /// L'album paraît (état 0x14).
    Open,
    Browse,
    /// Une image en grand (sous-états 1 à 3) : le plan 5 au noir, l'image ;
    /// ○ ou ✕ ; le plan 5 au noir, l'album.
    ViewIn,
    View,
    ViewOut,
    /// Une vidéo (sous-état 1) : confiée au jeu, qui rappelle `movie_done`.
    Movie,
    /// Le retour à la chambre (sous-états 7 et 8).
    Leave,
    Done,
}

/// Le cadre du curseur (Cursor_Make de sorte 5, Cursor_Tran5) : centre et
/// taille de départ et d'arrivée, l'avancée du glissement, l'opacité qui
/// va vers son but, la course des traits clairs (sur 120 images), et la
/// couleur qui dérive vers une couleur tirée au hasard.
struct Frame {
    from: [f32; 4],
    to: [f32; 4],
    count: i32,
    dur: i32,
    alpha: i32,
    goal: i32,
    step: i32,
    phase: i32,
    rgb: [i32; 3],
    target: [i32; 3],
}

/// EtColorSet 1 à 7 : les couleurs vers lesquelles dérive le cadre.
const COLORS: [[i32; 3]; 7] = [[0xFF; 3], [0xFF, 0x1E, 0x1E], [0x1E, 0xFF, 0x1E], [0x1E, 0x1E, 0xFF], [0xF0, 0xFF, 0], [0xCC, 0, 0xFF], [0, 0xFD, 0xFF]];

impl Frame {
    fn new(at: [f32; 4], alpha: i32, dur: i32) -> Frame {
        Frame { from: at, to: at, count: 1, dur: 1, alpha: 0, goal: alpha, step: (alpha / dur.max(1)).max(1), phase: 0, rgb: [0; 3], target: [0; 3] }
    }

    /// Cursor_Move : le cadre glisse vers une place en `dur` images.
    fn glide(&mut self, to: [f32; 4], dur: i32) {
        self.from = self.now();
        self.to = to;
        self.count = 0;
        self.dur = dur.max(1);
    }

    /// Cursor_Show, Cursor_Hidden, Cursor_Del : l'opacité va vers `alpha`
    /// en `dur` images.
    fn fade(&mut self, alpha: i32, dur: i32) {
        self.goal = alpha;
        self.step = ((alpha - self.alpha).abs() / dur.max(1)).max(1);
    }

    /// L'avancée du glissement, adoucie (sin(t × 180 − 90) + 1) / 2.
    fn now(&self) -> [f32; 4] {
        let t = self.count as f32 / self.dur as f32;
        let k = ((t * 180.0 - 90.0).to_radians().sin() + 1.0) / 2.0;
        std::array::from_fn(|i| self.from[i] + k * (self.to[i] - self.from[i]))
    }

    /// Cursor_Proc, et la couleur (au dessin, dans le jeu) : un pas vers la
    /// couleur visée ; atteinte, une autre au hasard.
    fn step(&mut self, sys: &mut Sys) {
        if self.alpha != self.goal {
            self.alpha = if self.alpha < self.goal { (self.alpha + self.step).min(self.goal) } else { (self.alpha - self.step).max(self.goal) };
        }
        self.phase = (self.phase + 1) % 0x78;
        if self.count < self.dur {
            self.count += 1;
        }
        for (c, t) in self.rgb.iter_mut().zip(self.target) {
            *c += (t - *c).signum();
        }
        if self.rgb == self.target {
            self.target = COLORS[(sys.rng.next() % 7) as usize];
        }
    }

    /// Cursor_Tran5 : un rectangle de traits (GP_Paint_Line, de demi-largeur
    /// 3 en halo, 1 ou 2 au bord), et des traits clairs qui le parcourent :
    /// sur les bords haut et bas de gauche à droite (et inversement), puis
    /// sur les côtés, une demi-période plus tard.
    fn draw(&self, gfx: &mut Gfx) {
        let [cx, cy, w, h] = self.now();
        let (left, right) = ((cx - w / 2.0).trunc(), (cx + w / 2.0).trunc());
        let (top, bottom) = ((cy - h / 2.0).trunc(), (cy + h / 2.0).trunc());
        let a = self.alpha;
        let line = |gfx: &mut Gfx, x0: f32, y0: f32, x1: f32, y1: f32, prio: i32, half: f32, alpha: i32| {
            let (x0, x1) = (x0.min(x1), x0.max(x1));
            let (y0, y1) = (y0.min(y1), y0.max(y1));
            gfx.paint([x0 - if x0 == x1 { half } else { 0.0 }, y0 - if y0 == y1 { half } else { 0.0 }, (x1 - x0).max(0.0) + if x0 == x1 { 2.0 * half } else { 0.0 }, (y1 - y0).max(0.0) + if y0 == y1 { 2.0 * half } else { 0.0 }], prio, alpha, self.rgb);
        };
        let p = FRAME_PRIO;
        line(gfx, left - 2.0, top, right + 2.0, top, p - 1, 3.0, a);
        line(gfx, right + 2.0, bottom, left - 2.0, bottom, p - 1, 3.0, a);
        line(gfx, right, top - 2.0, right, bottom + 2.0, p - 1, 3.0, a);
        line(gfx, left, bottom + 2.0, left, top - 2.0, p - 1, 3.0, a);
        line(gfx, left, top, right, top, p, 1.0, a * 2);
        line(gfx, right, bottom, left, bottom, p, 1.0, a * 2);
        line(gfx, right, top, right, bottom, p, 2.0, a * 2);
        line(gfx, left, bottom, left, top, p, 2.0, a * 2);
        // Les traits clairs : sur 60 images ils s'allongent, sur les 60
        // suivantes ils s'effacent par leur départ.
        let run = |phase: i32, len: f32| -> (f32, f32) { if phase < 0x3C { (0.0, phase as f32 / 60.0 * len) } else { ((phase - 0x3C) as f32 / 60.0 * len, len) } };
        let (from, to) = run(self.phase, w);
        line(gfx, left + from, top, left + to, top, p + 1, 4.0, a);
        line(gfx, right - to, bottom, right - from, bottom, p + 1, 4.0, a);
        let (from, to) = run((self.phase + 0x3C) % 0x78, h);
        line(gfx, right, top + from, right, top + to, p + 1, 4.0, a * 2);
        line(gfx, left, bottom - to, left, bottom - from, p + 1, 4.0, a * 2);
    }
}

pub struct Library {
    stage: Stage,
    at: At,
    /// La page (gpffff8a08), la feuille (8a0a), la colonne et la rangée du
    /// curseur (8a0c, 8a0e ; sur les onglets, la rangée est la page visée,
    /// sur les feuilles la colonne est la feuille visée).
    page: usize,
    sheet: usize,
    col: i32,
    row: i32,
    timer: i32,
    pages: Vec<Vec<Slot>>,
    /// Le pourcentage de chaque page (0x1ba9238).
    percent: [i32; PAGES],
    /// Le décor en grand (une image de l'album, ou la bonus).
    shown: Option<String>,
    /// La vidéo demandée au jeu.
    movie: Option<i16>,
    frame: Option<Frame>,
    windows: Windows,
}

impl Library {
    /// AD_CgLibMain, état 0 : les places de l'album (dans l'ordre des pages
    /// et des places ; une page s'arrête à sa première place vide), le
    /// pourcentage de chaque page, et l'image bonus de la troisième sorte
    /// si elle vient d'être gagnée (`bonus` : son décor, chargé).
    pub fn new(world: &World, album: &Album, sheet: impl Fn(&[u8]) -> Option<String>, bonus: Option<String>, sys: &mut Sys) -> Library {
        let pages: Vec<Vec<Slot>> = (0..PAGES as i32)
            .map(|page| {
                let mut slots = Vec::new();
                while let Some(e) = world.cg.iter().find(|e| e.page == page && e.slot == slots.len() as i32) {
                    let id = if e.movie { album::movie_id(world, &e.name) } else { album::bg_id(world, &e.name) };
                    slots.push(Slot { movie: e.movie, id: id.unwrap_or(-1), sheet: sheet(&e.sheet), rect: e.rect.map(f32::from) });
                }
                slots
            })
            .collect();
        let percent = std::array::from_fn(|k| {
            let slots = &pages[k];
            let seen = slots.iter().filter(|s| album.is_seen(s.id, s.movie)).count() as i32;
            if slots.is_empty() { 0 } else { seen * 100 / slots.len() as i32 }
        });
        let mut lib = Library { stage: Stage::Open, at: At::Pages, page: 0, sheet: 0, col: 0, row: 0, timer: 0, pages, percent, shown: None, movie: None, frame: None, windows: Windows::default() };
        if let Some(key) = bonus {
            // L'image bonus sort du noir en 0x14 images.
            lib.shown = Some(key);
            sys.blackout.end(&mut sys.effects, 0x14);
            sys.lock = 0x14;
            lib.stage = Stage::Bonus;
        }
        lib
    }

    /// Une image : vrai quand l'album est refermé (le noir tombé).
    /// iew charge le décor d'une image de l'album.
    pub fn step(&mut self, sys: &mut Sys, album: &Album, font: &Font, atlas: &mut Atlas, gfx: &mut Gfx, mut view: impl FnMut(&mut Gfx, i16) -> Option<String>) -> bool {
        if let Some(f) = &mut self.frame {
            f.step(sys);
        }
        self.windows.update();
        match self.stage {
            Stage::Bonus => {
                if sys.menu_key(0x1F) || sys.click.is_some() {
                    sys.blackout.start(&mut sys.effects, 0x14, 0x7F);
                    self.timer = 0x14;
                    self.stage = Stage::BonusOut;
                }
            }
            // Le jeu inverse ici son attente (il décompte tant que le
            // compte est sous 1, et passe sinon) : l'album paraît sans
            // attendre le noir. Bug corrigé, voir le README.
            Stage::BonusOut => {
                self.timer -= 1;
                if self.timer < 0 {
                    self.stage = Stage::Open;
                }
            }
            Stage::Open => {
                // État 0x14 : le fond de l'album, le cadre sur le premier
                // onglet, la fenêtre du pourcentage ; le noir s'efface en
                // 0x1e images.
                self.shown = None;
                self.frame = Some(Frame::new([80.0, 60.0, 120.0, 30.0], 0x7F, 0x1E));
                sys.blackout.end(&mut sys.effects, 0x1E);
                sys.lock = 0x1E;
                let glyphs = self.percent_text(font, atlas, gfx);
                self.windows.create(WINDOW, 2, PRIO, 2, Some(8), (0x10, 4), glyphs);
                self.windows.act(WINDOW, &[Action { dur: 10, kind: 1, x: 0x1E0, y: 0x2D, w: 0xD2, h: 0x25, frame: 0x78, back: 0x78, ..Action::READY }]);
                self.at = At::Pages;
                self.stage = Stage::Browse;
            }
            Stage::Browse => self.browse(sys, album, font, atlas, gfx),
            Stage::ViewIn => {
                // Sous-état 1 : 0x14 images ; l'image paraît à l'avant-dernière,
                // le plan 5 s'éclaircit à la dernière. Une vidéo passe au
                // bout des 0x14 images (AD_MVPlay).
                let movie = self.current().filter(|s| s.movie).map(|s| s.id);
                if self.timer == 0 && movie.is_some() {
                    self.movie = movie;
                    self.stage = Stage::Movie;
                } else if self.timer == 0 {
                    sys.dim.end(&mut sys.effects, 0xF);
                    self.timer = 0xF;
                    self.stage = Stage::View;
                } else {
                    if self.timer == 1
                        && let Some(s) = self.current()
                        && !s.movie
                    {
                        self.shown = view(gfx, s.id);
                    }
                    self.timer -= 1;
                }
            }
            Stage::View => {
                let ok = sys.menu_key(0x1F) || sys.click.is_some();
                if ok || sys.menu_key(0x1E) {
                    sys.dim.start(&mut sys.effects, 0xF, 0x7F);
                    self.timer = 0xF;
                    sys.menu_sound(0);
                    self.stage = Stage::ViewOut;
                }
            }
            Stage::ViewOut => {
                if self.timer == 0 {
                    // L'album revient : le fond, le cadre, la fenêtre.
                    sys.dim.end(&mut sys.effects, 0xF);
                    self.shown = None;
                    self.timer = 0xF;
                    if let Some(f) = &mut self.frame {
                        f.fade(0x80, 1);
                    }
                    let glyphs = self.percent_text(font, atlas, gfx);
                    self.windows.set_text(WINDOW, glyphs);
                    self.windows.act(WINDOW, &[Action { dur: 2, kind: 1, ..Action::READY }]);
                    self.stage = Stage::Browse;
                } else {
                    self.timer -= 1;
                }
            }
            Stage::Movie => {}
            Stage::Leave => {
                self.timer -= 1;
                if self.timer < 0 {
                    self.stage = Stage::Done;
                }
            }
            Stage::Done => return true,
        }
        false
    }

    /// La vidéo à passer (son numéro dans movie_entry).
    pub fn take_movie(&mut self) -> Option<i16> {
        self.movie.take()
    }

    /// La vidéo est finie (sous-états 2 et 3) : la musique repart du début
    /// (AD_BGMRestart), l'album revient aussitôt.
    pub fn movie_done(&mut self, sys: &mut Sys) {
        if self.stage == Stage::Movie {
            sys.restart_music(false);
            self.timer = 0;
            self.stage = Stage::ViewOut;
        }
    }

    /// La place sous le curseur des vignettes.
    fn current(&self) -> Option<&Slot> {
        self.pages[self.page].get(self.sheet * PER_SHEET + (self.row * 4 + self.col) as usize)
    }

    fn count(&self) -> i32 {
        self.pages[self.page].len() as i32
    }

    /// Le nombre de feuilles de la page (au moins une).
    fn sheets(&self) -> i32 {
        ((self.count() - 1).max(0) >> 4) + 1
    }

    fn percent_text(&self, font: &Font, atlas: &mut Atlas, gfx: &mut Gfx) -> Vec<font::Glyph> {
        let mut text = font.string(PERCENT).to_vec();
        if let Some(at) = text.windows(3).position(|w| w == b"%3d") {
            text.splice(at..at + 3, format!("{:3}", self.percent[self.page]).bytes());
        }
        font::typeset(font, atlas, gfx, &text, 0.0)
    }

    /// AD_CgLibKasolMove (Pos x, y, w, h) : le cadre glisse en 6 images
    /// vers ce que vise le curseur ; les touches attendent 4 images.
    fn move_frame(&mut self, sys: &mut Sys) {
        let to = place((self.at, self.col, self.row));
        if let Some(f) = &mut self.frame {
            f.glide(to, 6);
        }
        sys.lock = 4;
    }

    /// AD_CgLibKasolProc, sous-états 0, 4, 5 et 6.
    fn browse(&mut self, sys: &mut Sys, album: &Album, font: &Font, atlas: &mut Atlas, gfx: &mut Gfx) {
        if self.mouse(sys, album, font, atlas, gfx) {
            return;
        }
        // Partout : ✕ s'en va ; L1 et R1 changent de feuille, L2 et R2 de
        // page.
        if sys.menu_key(0x1E) {
            self.leave(sys);
            return;
        }
        let sheets = self.sheets();
        if sys.input.key(4) || sys.input.key(5) {
            let d = if sys.input.key(4) { -1 } else { 1 };
            self.sheet = (self.sheet as i32 + d).rem_euclid(sheets) as usize;
            sys.menu_sound(0);
            self.at = At::Sheets;
            self.col = self.sheet as i32;
            self.move_frame(sys);
            return;
        }
        if sys.input.key(6) || sys.input.key(7) {
            let d = if sys.input.key(6) { 6 } else { 8 };
            self.page = (self.page + d) % PAGES;
            self.sheet = self.sheet.min((self.sheets() - 1) as usize);
            self.update_percent(font, atlas, gfx);
            sys.menu_sound(0);
            self.at = At::Pages;
            self.col = 0;
            self.row = self.page as i32;
            self.move_frame(sys);
            return;
        }
        let dir = |sys: &Sys, d: i32| sys.input.direct(d, 10, 6, true);
        let count = self.count();
        let base = self.sheet as i32 * 16;
        match self.at {
            At::Thumbs => {
                if sys.input.key(3) {
                    self.open(sys, album);
                } else if dir(sys, 2) {
                    if self.col < 1 {
                        self.thumbs_to_pages();
                    } else {
                        self.col -= 1;
                    }
                    self.move_frame(sys);
                } else if dir(sys, 3) {
                    if self.col + 1 < 4 && self.col + 1 + self.row * 4 + base < count {
                        self.col += 1;
                    } else {
                        self.thumbs_to_pages();
                    }
                    self.move_frame(sys);
                } else if dir(sys, 0) {
                    if self.row < 1 {
                        self.at = At::Sheets;
                        self.col = self.col.min(sheets - 1);
                    } else {
                        self.row -= 1;
                    }
                    self.move_frame(sys);
                } else if dir(sys, 1) {
                    if self.row + 1 < 4 && self.col + (self.row + 1) * 4 + base < count {
                        self.row += 1;
                    } else {
                        self.at = At::Sheets;
                        self.col = self.col.min(sheets - 1);
                    }
                    self.move_frame(sys);
                }
            }
            At::Pages => {
                if dir(sys, 0) {
                    if self.row < 1 {
                        self.at = At::Back;
                    } else {
                        self.row -= 1;
                    }
                    self.move_frame(sys);
                } else if dir(sys, 1) {
                    if self.row + 1 < PAGES as i32 {
                        self.row += 1;
                    } else {
                        self.at = At::Back;
                    }
                    self.move_frame(sys);
                } else if sys.menu_key(0x1F) {
                    if self.page as i32 != self.row {
                        self.page = self.row as usize;
                        self.sheet = 0;
                        self.update_percent(font, atlas, gfx);
                        sys.menu_sound(0);
                    }
                } else if dir(sys, 3) || dir(sys, 2) {
                    // Vers les vignettes : la première colonne, ou la
                    // dernière ; la rangée la plus proche qui existe.
                    let right = dir(sys, 3);
                    self.col = if right { 0 } else { 3 };
                    self.row = [0, 0, 1, 1, 2, 3, 3][self.row.clamp(0, 6) as usize];
                    self.at = At::Thumbs;
                    if right {
                        while self.row > 0 && self.col + self.row * 4 + base >= count {
                            self.row -= 1;
                        }
                    } else {
                        let mut k = self.col + self.row * 4;
                        while k > 0 && k + base >= count {
                            k -= 1;
                        }
                        (self.col, self.row) = (k & 3, k >> 2);
                    }
                    self.move_frame(sys);
                }
            }
            At::Sheets => {
                if dir(sys, 2) {
                    if self.col < 1 {
                        self.at = At::Back;
                    } else {
                        self.col -= 1;
                    }
                    self.move_frame(sys);
                } else if dir(sys, 3) {
                    if self.col + 1 < 4 && self.col < ((count - 1).max(0) >> 4) {
                        self.col += 1;
                    } else {
                        self.at = At::Back;
                    }
                    self.move_frame(sys);
                } else if dir(sys, 0) || dir(sys, 1) {
                    // Vers les vignettes : en montant, la dernière rangée qui
                    // existe dans cette colonne ; en descendant, la première.
                    self.at = At::Thumbs;
                    self.row = if dir(sys, 0) { (0..4).rev().find(|r| self.col + r * 4 + base < count).unwrap_or(self.row) } else { 0 };
                    if self.row == 0 {
                        while self.col > 0 && self.col + base >= count {
                            self.col -= 1;
                        }
                    }
                    self.move_frame(sys);
                } else if sys.menu_key(0x1F) && self.sheet as i32 != self.col {
                    self.sheet = self.col as usize;
                    sys.menu_sound(0);
                }
            }
            At::Back => {
                if dir(sys, 0) {
                    self.at = At::Pages;
                    self.row = 6;
                    self.move_frame(sys);
                } else if dir(sys, 1) {
                    self.at = At::Pages;
                    self.row = 0;
                    self.move_frame(sys);
                } else if dir(sys, 3) {
                    self.at = At::Sheets;
                    self.col = 0;
                    self.move_frame(sys);
                } else if dir(sys, 2) {
                    self.at = At::Sheets;
                    self.col = (count - 1).max(0) >> 4;
                    self.move_frame(sys);
                } else if sys.menu_key(0x1F) {
                    self.leave(sys);
                }
            }
        }
    }

    /// Des vignettes vers les onglets : la page à hauteur de la rangée.
    fn thumbs_to_pages(&mut self) {
        self.at = At::Pages;
        self.row = match self.row {
            0 => 1,
            1 => 3,
            2 => 4,
            _ => 6,
        };
        self.col = 0;
    }

    fn update_percent(&mut self, font: &Font, atlas: &mut Atlas, gfx: &mut Gfx) {
        let glyphs = self.percent_text(font, atlas, gfx);
        self.windows.set_text(WINDOW, glyphs);
    }

    /// Sous-état 0, ○ : une image vue s'ouvre en grand (le plan 5 au noir en
    /// 0xf images, le cadre et la fenêtre s'en vont) ; pour une vidéo vue, la
    /// musique s'arrête (AD_BGMPause).
    fn open(&mut self, sys: &mut Sys, album: &Album) {
        let Some(s) = self.current() else { return };
        if !album.is_seen(s.id, s.movie) {
            return;
        }
        if s.movie {
            sys.pause_music();
        }
        sys.dim.start(&mut sys.effects, 0xF, 0x7F);
        self.timer = 0x14;
        sys.menu_sound(2);
        if let Some(f) = &mut self.frame {
            f.fade(0, 10);
        }
        self.windows.act(WINDOW, &[Action { dur: 10, kind: 2, ..Action::READY }]);
        self.stage = Stage::ViewIn;
    }

    /// Sous-état 7 : le noir en 10 images, le cadre et la fenêtre s'en vont.
    fn leave(&mut self, sys: &mut Sys) {
        sys.blackout.start(&mut sys.effects, 10, 0x7F);
        self.timer = 10;
        sys.menu_sound(0);
        if let Some(f) = &mut self.frame {
            f.fade(0, 10);
        }
        self.windows.act(WINDOW, &[Action { dur: 10, kind: 3, ..Action::READY }]);
        self.stage = Stage::Leave;
    }

    /// La souris, comme dans un visual novel sur PC : survoler une vignette,
    /// un onglet, une feuille ou « 戻る » y mène le curseur ; un clic vaut
    /// ○, le bouton droit ✕.
    fn mouse(&mut self, sys: &mut Sys, album: &Album, font: &Font, atlas: &mut Atlas, gfx: &mut Gfx) -> bool {
        let base = self.sheet as i32 * 16;
        let count = self.count();
        let mut targets: Vec<_> = (0..16).filter(|&k| k + base < count).map(|k| (At::Thumbs, k & 3, k >> 2)).collect();
        targets.extend((0..PAGES as i32).map(|p| (At::Pages, 0, p)));
        targets.extend((0..self.sheets()).map(|s| (At::Sheets, s, 0)));
        targets.push((At::Back, 0, 0));
        let zones: Vec<_> = targets.into_iter().map(|t| {
            let [x, y, w, h] = place(t);
            Zone::centered(x, y, w, h, t)
        }).collect();
        let Pick { hovered, clicked } = pointer::pick(sys, &zones);
        if let Some(t) = hovered
            && t != (self.at, self.col, self.row)
        {
            (self.at, self.col, self.row) = t;
            self.move_frame(sys);
        }
        let Some((at, col, row)) = clicked else { return false };
        match at {
            At::Thumbs => self.open(sys, album),
            At::Pages if self.page as i32 != row => {
                self.page = row as usize;
                self.sheet = 0;
                self.update_percent(font, atlas, gfx);
                sys.menu_sound(0);
            }
            At::Sheets if self.sheet as i32 != col => {
                self.sheet = col as usize;
                sys.menu_sound(0);
            }
            At::Back => self.leave(sys),
            _ => {}
        }
        true
    }

    /// AD_CgLibTran (et le décor) : le fond de l'album, ou l'image en grand ;
    /// les onglets, les feuilles, les vignettes (noires si elles ne sont pas
    /// vues), la fenêtre, le cadre.
    pub fn draw(&self, gfx: &mut Gfx, album: &Album, atlas: Option<&Atlas>) {
        let full = |gfx: &mut Gfx, key: &str| gfx.tran(key, [0.0, 0.0, 512.0, 448.0], [0.0, 0.0, 640.0, 448.0], 0x0A, 0x7F, Blend::Normal, GREY);
        full(gfx, self.shown.as_deref().unwrap_or(BACK));
        let browsing = matches!(self.stage, Stage::Open | Stage::Browse | Stage::ViewIn | Stage::Leave | Stage::Done);
        if !browsing {
            return;
        }
        {
            let icons = ICONS;
            for p in 0..PAGES as i32 {
                let alpha = if p == self.page as i32 { 0x7F } else { 0x5A };
                gfx.tran_center(Some(icons), 1.0, [0.0, (p * 32 + 1) as f32, 120.0, 30.0], place((At::Pages, 0, p)), PRIO, alpha, GREY, 0.0, None);
            }
            gfx.tran_center(Some(icons), 1.0, [0.0, 225.0, 120.0, 30.0], place((At::Back, 0, 0)), PRIO, 0x7F, GREY, 0.0, None);
            for s in 0..self.sheets().min(4) {
                let alpha = if s == self.sheet as i32 { 0x7F } else { 0x5A };
                gfx.tran_center(Some(icons), 1.0, [130.0, (s * 32 + 3) as f32, 45.0, 26.0], [(s * 100 + 0xF0) as f32, 410.0, 45.0, 26.0], PRIO, alpha, GREY, 0.0, None);
            }
        }
        for (k, slot) in self.pages[self.page].iter().skip(self.sheet * PER_SHEET).take(PER_SHEET).enumerate() {
            let (x, y) = (((k & 3) * 0x73 + 0xA5) as f32, ((k >> 2) * 0x4B + 0x50) as f32);
            match (&slot.sheet, album.is_seen(slot.id, slot.movie)) {
                (Some(sheet), true) => gfx.tran(sheet, slot.rect, [x, y, 90.0, 63.0], PRIO, 0x7F, Blend::Normal, GREY),
                _ => gfx.paint([x, y, 90.0, 63.0], PRIO, 0x80, [0; 3]),
            }
        }
        self.windows.draw(gfx, atlas);
        if let Some(f) = &self.frame
            && !matches!(self.stage, Stage::View | Stage::ViewOut)
        {
            f.draw(gfx);
        }
    }
}
