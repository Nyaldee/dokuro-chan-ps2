//! L'historique (AD_BackLog*) : les 150 dernières pages lues, chacune avec
//! son nom, sa voix et le style de sa fenêtre. Ouvert sur une page qui
//! attend, il montre la page la plus récente au centre de l'écran, les
//! précédentes au-dessus et les suivantes au-dessous, chacune dans sa
//! fenêtre (10 pixels entre deux). Deux icônes : la voix de la page (○ la
//! fait réentendre), et le retour à cette page (△), dont l'histoire garde
//! l'état (AD_RewindSave, à l'attente de chaque page).

use std::collections::VecDeque;

use crate::dialog::{Action, Windows};
use crate::font::{Atlas, Glyph};
use crate::gfx::{Gfx, GREY};
use crate::savedata;

/// Les pages gardées (0x96).
const SIZE: usize = 0x96;
/// Les fenêtres visibles : la page en cours, deux de plus au-dessus et deux
/// au-dessous (AD_BackLogModeStart en ouvre jusqu'à trois et deux).
const NAMES: [&str; 5] = ["save0", "save1", "save2", "save3", "save4"];
/// Les icônes, dans « win2 » (lues par le registre gp, 0x365780 à
/// 0x365798) : la voix et le retour, 64 × 53, centrées en (38, 204) et
/// (38, 254), prio 0xe3.
const MIC: [f32; 4] = [126.0, 0.0, 64.0, 53.0];
const REWIND: [f32; 4] = [126.0, 53.0, 64.0, 53.0];

/// Une page : ses glyphes, la taille de sa fenêtre (AD_BackLogSizeSet :
/// une page fait 0x253 de large et 0x8c de haut avec un nom, 0x69 sans ;
/// un choix est plus étroit), sa voix, le style de sa fenêtre.
pub struct Entry {
    pub glyphs: Vec<Glyph>,
    pub width: i32,
    pub height: i32,
    pub voice: Option<u32>,
    pub style: usize,
    /// La partie à l'attente de cette page, pour y revenir.
    pub rewind: Option<savedata::Game>,
}

#[derive(Default)]
pub struct Backlog {
    entries: VecDeque<Entry>,
    /// La page au centre (0 : la plus récente).
    scroll: usize,
    windows: Windows,
    /// La page montrée par chaque fenêtre.
    slots: [Option<usize>; 5],
    /// La dernière page attend son état (0x1b9bbe8), les images que dure
    /// le défilement (gpffff8a00).
    pending: bool,
    busy: i32,
    /// Les icônes : opacité (de 12 en 12), battement de la voix (degrés), et
    /// si elles sont dessinées à cette image.
    mic: (i32, i32, bool),
    back: (i32, bool),
}

impl Backlog {
    /// AD_BackLogSizeSet : une page de plus (la plus ancienne s'en va) ;
    /// une page de texte attendra son état (page).
    pub fn push(&mut self, entry: Entry, page: bool) {
        if self.entries.len() == SIZE {
            self.entries.pop_back();
        }
        self.entries.push_front(entry);
        self.pending = page;
    }

    /// La dernière page attend-elle son état ?
    pub fn pending(&self) -> bool {
        self.pending
    }

    /// AD_BackLogSubSaveProc : l'état de la partie à l'attente de la dernière
    /// page, qui devient un point de retour.
    pub fn keep(&mut self, game: savedata::Game) {
        if let Some(e) = self.entries.front_mut()
            && std::mem::take(&mut self.pending)
        {
            e.rewind = Some(game);
        }
    }

    /// La partie où revient △ : celle de la page au centre, sauf la plus
    /// récente.
    pub fn rewind(&self) -> Option<&savedata::Game> {
        self.entries.get(self.scroll).filter(|_| self.scroll > 0).and_then(|e| e.rewind.as_ref())
    }

    /// Les icônes ne sont plus dessinées (l'historique se ferme).
    pub fn icons_off(&mut self) {
        self.mic.2 = false;
        self.back.1 = false;
    }

    /// La page glisse encore.
    pub fn busy(&self) -> bool {
        self.busy > 0
    }

    /// AD_BackLogMicTran et AD_BackLogRewindTran, à chaque image : l'icône
    /// apparaît (1), bat (2, la voix joue : sa taille varie de ±10 %
    /// autour de 0,9) ou s'efface (0) ; None : pas dessinée.
    pub fn icons(&mut self, mic: i32, back: Option<i32>) {
        self.icons_off();
        let fade = |a: i32, on: bool| (a + if on { 12 } else { -12 }).clamp(0, 0x7F);
        self.mic.1 = if mic >= 2 { (self.mic.1 + 5) % 360 } else { 0 };
        self.mic.0 = fade(self.mic.0, mic >= 1);
        self.mic.2 = true;
        self.back.1 = back.is_some();
        if let Some(b) = back {
            self.back.0 = fade(self.back.0, b >= 1);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// AD_BackLogModeStart : les fenêtres s'ouvrent en 10 images autour de
    /// la page la plus récente.
    pub fn open(&mut self) {
        self.scroll = 0;
        self.windows = Windows::default();
        self.slots = [None; 5];
        self.place(10);
    }

    /// AD_BackLogModeEnd : les fenêtres se ferment en 10 images.
    pub fn close(&mut self) {
        self.icons_off();
        for (k, name) in NAMES.iter().enumerate() {
            if self.slots[k].take().is_some() {
                self.windows.act(name, &[Action { dur: 10, kind: 3, ..Action::READY }]);
            }
        }
    }

    /// AD_BackLogModeProc : ↑ (ou L1) remonte d'une page, ↓ (ou R1)
    /// redescend ; la page arrive en `dur` images.
    pub fn scroll(&mut self, older: bool, dur: i32) {
        let to = if older { (self.scroll + 1).min(self.entries.len().saturating_sub(1)) } else { self.scroll.saturating_sub(1) };
        if to != self.scroll {
            self.scroll = to;
            self.busy = dur;
            self.place(dur);
        }
    }

    /// Au centre : la page la plus récente ?
    pub fn at_newest(&self) -> bool {
        self.scroll == 0
    }

    pub fn voice(&self) -> Option<u32> {
        self.entries.get(self.scroll).and_then(|e| e.voice)
    }

    /// Les fenêtres des pages visibles à leur place : la page au centre en y
    /// = 0xe0, chaque voisine à la suite, 10 pixels plus loin.
    fn place(&mut self, dur: i32) {
        let entries = &self.entries;
        let Some(centre) = entries.get(self.scroll) else { return };
        let mut wanted: Vec<(usize, i32)> = Vec::new();
        // Au-dessus : la page au centre et deux plus anciennes.
        let mut y = (centre.height >> 1) + 0xE0;
        for (k, e) in entries.iter().enumerate().skip(self.scroll).take(3) {
            let h = e.height;
            y -= h >> 1;
            wanted.push((k, y));
            y -= (h >> 1) + 10;
        }
        // Au-dessous : deux plus récentes.
        let mut y = (centre.height >> 1) + 0xEA;
        for k in (self.scroll.saturating_sub(2)..self.scroll).rev() {
            let h = entries[k].height;
            y += h >> 1;
            wanted.push((k, y));
            y += (h >> 1) + 10;
        }
        // Une fenêtre par page, nommée d'après sa place dans l'anneau.
        for (slot, name) in NAMES.iter().enumerate() {
            let target = wanted.iter().find(|&&(k, _)| k % 5 == slot).copied();
            match (self.slots[slot], target) {
                (Some(k), Some((n, y))) if k == n => {
                    self.windows.act(name, &[Action { dur, kind: 1, y, ..Action::READY }]);
                }
                (_, Some((n, y))) => {
                    let e = &entries[n];
                    self.windows.create(name, e.style, 0xE1, 2, Some(8), (0x19, 0xC), e.glyphs.clone());
                    self.windows.act(name, &[Action { dur: 10, kind: 1, x: 0x15E, y, w: e.width, h: e.height, frame: 0, back: 100, ..Action::READY }]);
                    self.slots[slot] = Some(n);
                }
                (Some(_), None) => {
                    self.windows.act(name, &[Action { dur: 5, kind: 3, ..Action::READY }]);
                    self.slots[slot] = None;
                }
                (None, None) => {}
            }
        }
    }

    pub fn update(&mut self) {
        self.windows.update();
        self.busy = (self.busy - 1).max(0);
    }

    /// Les fenêtres, et les icônes demandées à cette image.
    pub fn draw(&self, gfx: &mut Gfx, atlas: Option<&Atlas>) {
        self.windows.draw(gfx, atlas);
        if self.mic.2 {
            let k = (self.mic.1 as f32).to_radians().sin() * 10.0 + 100.0;
            let (w, h) = ((64.0 * 90.0 / k).trunc(), (53.0 * 90.0 / k).trunc());
            gfx.tran_center(Some("win2"), 1.0, MIC, [38.0, 204.0, w, h], 0xE3, self.mic.0, GREY, 0.0, None);
        }
        if self.back.1 {
            gfx.tran_center(Some("win2"), 1.0, REWIND, [38.0, 254.0, 64.0, 53.0], 0xE3, self.back.0, GREY, 0.0, None);
        }
    }
}
