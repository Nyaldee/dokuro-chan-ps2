//! Les fenêtres du jeu (AD_Win*, AD_Window*) et leur texte : un cadre en
//! neuf morceaux (« win1 » de SYSTEM.UNI : un fond et un contour) qui
//! s'ouvre et se ferme en grandissant depuis son centre, et suit jusqu'à
//! cinq étapes (position, taille, opacités, style). S'y ajoutent les deux
//! fenêtres dont se sert le menu du challenge : l'information
//! (AD_InformationWin*) et la question はい / いいえ (AD_CheckWin*), avec son
//! curseur (Cursor_*).

use crate::font::{Atlas, Glyph};
use crate::pointer::{self, Zone};
use crate::sys::Sys;
use crate::gfx::{Blend, Gfx, GREY};

/// L'image des cadres, et celle de la fenêtre de texte de l'histoire (style
/// 0).
pub const TEXTURE: &str = "win1";
pub const MAINWIN: &str = "mainwin";
/// L'image du curseur.
pub const CURSOR: &str = "kasol";
/// Valeur inchangée d'une étape.
const KEEP: i32 = -5000;

/// Une étape (AD_WinActSet) : durée, sorte (1 ouvre, 2 ferme, 3 ferme et
/// supprime), position du centre et taille, priorité, opacités du contour
/// et du fond, style.
#[derive(Clone, Copy)]
pub struct Action {
    pub dur: i32,
    pub kind: i32,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub prio: i32,
    pub frame: i32,
    pub back: i32,
    pub style: i32,
}

impl Action {
    /// AD_WinActGetReady : rien ne change.
    pub const READY: Action = Action { dur: -1, kind: 0, x: KEEP, y: KEEP, w: KEEP, h: KEEP, prio: -1, frame: KEEP, back: KEEP, style: -1 };
}

/// Le texte d'une fenêtre : ses glyphes, son décalage dans la fenêtre,
/// l'ombre (palette).
struct Text {
    glyphs: Vec<Glyph>,
    offset: (i32, i32),
    shadow: Option<u8>,
}

/// Une fenêtre (AD_WinMake).
struct Win {
    name: &'static str,
    /// 0 : se ferme ; 1 : s'ouvre ; 2 : cachée ; 3 : ouverte.
    state: u8,
    /// L'étape en cours (6 : aucune) et son compte (remis à zéro à chaque
    /// étape ; le jeu l'oublie : bug corrigé, voir le README).
    step: usize,
    counter: i32,
    actions: [Action; 6],
    /// Ce que valent position, taille, priorité, opacités et style quand
    /// l'étape en cours commence.
    rect: [i32; 4],
    prio: i32,
    alpha: [i32; 2],
    style: usize,
    /// L'ouverture (1 : depuis le centre) et le point fixe (5 : le centre,
    /// comme sur un pavé numérique).
    zoom: u8,
    anchor: u8,
    text: Text,
    /// Le texte est affiché (fenêtre ouverte jusqu'au bout), depuis
    /// clock images ; skip : tout d'un coup (AD_WinTxtSpeedChange(0)).
    printed: bool,
    clock: f32,
    skip: bool,
}

/// Les morceaux d'un style de cadre (AD_GameWinSet) : les bords puis le
/// centre du fond, les bords du contour, la taille d'un coin, les coins du
/// fond et du contour. Les styles 2 à 4 sont décalés de 52 pixels chacun.
struct Masu9 {
    back_edges: [[i32; 4]; 4],
    back_center: [i32; 4],
    frame_edges: [(i32, i32); 4],
    corner: (i32, i32),
    back_corners: [(i32, i32); 4],
    frame_corners: [(i32, i32); 4],
}

const STYLE: Masu9 = Masu9 {
    back_edges: [[0, 76, 24, 4], [28, 76, 24, 4], [24, 52, 4, 24], [24, 80, 4, 24]],
    back_center: [24, 76, 4, 4],
    frame_edges: [(0, 24), (28, 24), (24, 0), (24, 28)],
    corner: (24, 24),
    back_corners: [(0, 52), (28, 52), (0, 80), (28, 80)],
    frame_corners: [(0, 0), (28, 0), (0, 28), (28, 28)],
};

/// Le décalage d'un style dans l'image (1 et 5 bleus, 2, 3 rose, 4 gris).
fn style_shift(style: usize) -> Option<i32> {
    match style {
        1 | 5 => Some(0),
        2..=4 => Some((style as i32 - 1) * 0x34),
        _ => None,
    }
}

impl Win {
    /// AD_WindowTranProc1 : où en est la fenêtre dans l'étape en cours.
    fn now(&self) -> ([i32; 4], [i32; 2], f32) {
        let mut rect = self.rect;
        let mut alpha = self.alpha;
        let mut t = 1.0;
        if self.step < 6 {
            let a = self.actions[self.step];
            t = (self.counter as f32 / a.dur as f32).clamp(0.0, 1.0);
            let lerp = |from: i32, to: i32| if to == KEEP { from } else { (from as f32 + t * (to - from) as f32) as i32 };
            for (v, to) in rect.iter_mut().zip([a.x, a.y, a.w, a.h]) {
                *v = lerp(*v, to);
            }
            alpha = [lerp(alpha[0], a.frame), lerp(alpha[1], a.back)];
        }
        (rect, alpha, t)
    }

    /// AD_WindowStateProc : l'étape finie devient l'état de la fenêtre.
    fn settle(&mut self, a: Action) {
        for (v, to) in self.rect.iter_mut().zip([a.x, a.y, a.w, a.h]) {
            if to != KEEP {
                *v = to;
            }
        }
        for (v, to) in self.alpha.iter_mut().zip([a.frame, a.back]) {
            if to != KEEP {
                *v = to;
            }
        }
        if a.prio != -1 {
            self.prio = a.prio;
        }
        if a.style >= 0 {
            self.style = a.style as usize;
        }
    }

    /// AD_WindowTranProc2 et 3 : à l'ouverture et à la fermeture, la
    /// fenêtre grandit ou rapetisse (sinus), autour de son point fixe. Le
    /// jeu a aussi deux ouvertures en deux temps (3 et 4), dont ces
    /// fenêtres ne se servent pas.
    fn zoomed(&self, mut rect: [i32; 4], t: f32) -> [i32; 4] {
        let (w0, h0) = (rect[2].abs(), rect[3].abs());
        rect[2] = w0;
        rect[3] = h0;
        if self.zoom == 1 {
            let s = (((t * 180.0 - 90.0) as i16 as f32).to_radians().sin() + 1.0) / 2.0;
            let k = if self.state == 1 { s } else { 1.0 - s };
            rect[2] = (rect[2] as f32 * k) as i32;
            rect[3] = (rect[3] as f32 * k) as i32;
        }
        if rect[2] < 1 || rect[3] < 1 || self.zoom == 2 {
            return rect;
        }
        let fx = (1.0 - rect[2] as f32 / w0 as f32) / 2.0;
        let fy = (1.0 - rect[3] as f32 / h0 as f32) / 2.0;
        match self.anchor {
            1 | 4 | 7 => rect[0] = (rect[0] as f32 - w0 as f32 * fx) as i32,
            3 | 6 | 9 => rect[0] = (rect[0] as f32 + w0 as f32 * fx) as i32,
            _ => {}
        }
        match self.anchor {
            1..=3 => rect[1] = (rect[1] as f32 + h0 as f32 * fy) as i32,
            7..=9 => rect[1] = (rect[1] as f32 - h0 as f32 * fy) as i32,
            _ => {}
        }
        rect
    }
}

/// AD_WindowMasu9Tran : le fond (opacité `back`) puis le contour (`frame`),
/// en coins, bords et centre autour du centre `rect`. Une fenêtre plus
/// petite que deux coins les écrase.
fn masu9(gfx: &mut Gfx, style: usize, rect: [i32; 4], prio: i32, frame: i32, back: i32) {
    if style == 0 {
        // AD_WindowMasu1Tran : le style 0 est une seule image (« mainwin »),
        // fond (0, 110) puis cadre (0, 0), 512 × 110, à la taille de la
        // fenêtre.
        let [cx, cy, w, h] = rect.map(|v| v as f32);
        let dst = [cx - (w as i32 >> 1) as f32, cy - (h as i32 >> 1) as f32, w, h];
        gfx.tran(MAINWIN, [0.0, 110.0, 512.0, 110.0], dst, prio, back, Blend::Normal, GREY);
        gfx.tran(MAINWIN, [0.0, 0.0, 512.0, 110.0], dst, prio, frame, Blend::Normal, GREY);
        return;
    }
    let Some(shift) = style_shift(style) else { return };
    let s = &STYLE;
    let [cx, cy, w, h] = rect;
    let (cw, ch) = s.corner;
    let (half_w, half_h) = (w >> 1, h >> 1);
    let fx = (w as f32 / (cw * 2) as f32).clamp(0.0, 1.0);
    let fy = (h as f32 / (ch * 2) as f32).clamp(0.0, 1.0);
    let mut draw = |src: [i32; 4], dst: [i32; 4], alpha: i32| {
        let [u, v, sw, sh] = src;
        gfx.tran(TEXTURE, [(u + shift) as f32, v as f32, sw as f32, sh as f32], dst.map(|n| n as f32), prio, alpha, Blend::Normal, GREY);
    };
    // Les coins.
    let dw = if fx < 1.0 { half_w } else { cw };
    let dh = if fy < 1.0 { half_h } else { ch };
    for i in 0..4 {
        let x = if i % 2 == 0 { cx - half_w } else { cx + (half_w - dw) };
        let y = if i < 2 { cy - half_h } else { cy + (half_h - dh) };
        let (bu, bv) = s.back_corners[i];
        let (fu, fv) = s.frame_corners[i];
        draw([bu, bv, cw, ch], [x, y, dw, dh], back);
        draw([fu, fv, cw, ch], [x, y, dw, dh], frame);
    }
    // Les bords : gauche et droite, puis haut et bas.
    let (mut x, mut y) = (0, 0);
    for i in 0..4 {
        let [u, v, ew, eh] = s.back_edges[i];
        let (dw, dh);
        if i < 2 {
            if h <= ch * 2 {
                continue;
            }
            dh = h - ch * 2;
            dw = if fx >= 1.0 { ew } else { half_w };
            if i == 0 {
                x = cx - half_w;
                y = cy - (dh >> 1);
            } else {
                x = cx + half_w - dw;
            }
        } else {
            if w <= cw * 2 {
                continue;
            }
            dw = w - cw * 2;
            dh = if fy >= 1.0 { eh } else { half_h };
            if i == 2 {
                x = cx - (dw >> 1);
                y = cy - half_h;
            } else {
                y = cy + half_h - dh;
            }
        }
        let (fu, fv) = s.frame_edges[i];
        draw([u, v, ew, eh], [x, y, dw, dh], back);
        draw([fu, fv, ew, eh], [x, y, dw, dh], frame);
    }
    if cw * 2 < w && ch * 2 < h {
        draw(s.back_center, [cx - half_w + cw, cy - half_h + ch, w - cw * 2, h - ch * 2], back);
    }
}

/// Un curseur (Cursor_* ; style 0 : « kasol »), nommé comme dans le jeu.
struct Cursor {
    name: &'static str,
    from: (i32, i32),
    to: (i32, i32),
    counter: i32,
    dur: i32,
    alpha: i32,
    target: i32,
    step: i32,
    /// 1 : apparaît ; 2 : disparaît ; 3 : posé ; 4 : glisse ; 5 : estompé
    /// (immobile, Cursor_Move ne le déplace plus).
    state: u8,
    prio: i32,
}

/// EtNumChase : une valeur avance vers sa cible d'au plus |step| ; tout
/// près, de 90 % de l'écart (et la rejoint sous 1). Un pas nul n'avance
/// jamais.
fn chase(value: i32, target: i32, step: i32) -> i32 {
    let s = step.abs();
    if value == target {
        return value;
    }
    let near = |gap: i32| (gap as f32 / s as f32) * s as f32 * 0.9;
    if target < value {
        if target < value - s {
            return value - s;
        }
        let f = near(value - target);
        if f < 1.0 { target } else { (value as f32 - f) as i32 }
    } else {
        if value + s < target {
            return value + s;
        }
        let f = near(target - value);
        if f < 1.0 { target } else { (value as f32 + f) as i32 }
    }
}

impl Cursor {
    /// Où en est le glissement (Cursor_AllTran : t = compte / durée en
    /// glissant, 1 sinon).
    fn t(&self) -> f32 {
        if self.state == 4 { self.counter as f32 / self.dur as f32 } else { 1.0 }
    }

    fn now(&self) -> (i32, i32) {
        let t = self.t();
        let at = |a: i32, b: i32| (a as f32 + (b - a) as f32 * t) as i32;
        (at(self.from.0, self.to.0), at(self.from.1, self.to.1))
    }

    /// Le point de départ d'une nouvelle étape : où le compte en est, en
    /// proportion de la durée (même hors glissement).
    fn reached(&self) -> (i32, i32) {
        let t = self.counter as f32 / self.dur as f32;
        let at = |a: i32, b: i32| (a as f32 + t * (b - a) as f32) as i32;
        (at(self.from.0, self.to.0), at(self.from.1, self.to.1))
    }

    /// Cursor_Proc ; faux une fois disparu.
    fn update(&mut self) -> bool {
        self.alpha = chase(self.alpha, self.target, self.step);
        if self.counter < self.dur {
            self.counter += 1;
            return true;
        }
        self.counter = 1;
        self.dur = 1;
        self.from = self.to;
        match self.state {
            5 => self.alpha = self.target,
            2 => return false,
            1 => {
                self.alpha = self.target;
                self.state = 3;
            }
            4 => self.state = 3,
            _ => {}
        }
        true
    }
}

/// Les fenêtres ouvertes, les curseurs (dix au plus, comme dans le jeu), la
/// question en cours.
#[derive(Default)]
pub struct Windows {
    wins: Vec<Win>,
    cursors: Vec<Cursor>,
    /// La réponse choisie (0 はい, 1 いいえ).
    choice: i32,
}

/// Le curseur de la question.
const CHECK_CURSOR: &str = "check";
/// Les réponses d'une question : centres (はい, いいえ), hauteur, taille.
const ANSWER_X: [i32; 2] = [0xE6, 0x19A];
const ANSWER_Y: i32 = 0xF4;
const ANSWER_SIZE: (i32, i32) = (0x78, 0x22);

impl Windows {
    fn get(&mut self, name: &str) -> Option<&mut Win> {
        self.wins.iter_mut().find(|w| w.name == name)
    }

    /// AD_WinMake, puis AD_WinStateSet (ouverture, point fixe, ombre,
    /// décalage du texte) et le texte (AD_WinTxtSet).
    fn make(&mut self, name: &'static str, style: usize, prio: i32, shadow: Option<u8>, offset: (i32, i32), glyphs: Vec<Glyph>) {
        self.create(name, style, prio, 1, shadow, offset, glyphs);
    }

    /// AD_WinMake et AD_WinStateSet : une fenêtre cachée, son ouverture (1 :
    /// depuis le centre ; 2 : en fondu), l'ombre du texte, la place du
    /// texte.
    #[allow(clippy::too_many_arguments)]
    pub fn create(&mut self, name: &'static str, style: usize, prio: i32, zoom: u8, shadow: Option<u8>, offset: (i32, i32), glyphs: Vec<Glyph>) {
        self.wins.retain(|w| w.name != name);
        self.wins.push(Win {
            name,
            state: 2,
            step: 6,
            counter: -1,
            actions: [Action::READY; 6],
            rect: [0; 4],
            prio,
            alpha: [0x7F; 2],
            style,
            zoom,
            anchor: 5,
            text: Text { glyphs, offset, shadow },
            printed: false,
            clock: 0.0,
            skip: false,
        });
    }

    /// AD_WinActSet : de nouvelles étapes. Pendant une ouverture ou une
    /// fermeture, elles suivent l'étape en cours ; sinon elles la
    /// remplacent (une fenêtre neuve prend d'emblée la place de la
    /// première).
    pub fn act(&mut self, name: &str, acts: &[Action]) {
        let Some(w) = self.get(name) else { return };
        if w.state <= 1 {
            w.actions[0] = w.actions[w.step.min(5)];
            for (slot, a) in w.actions[1..].iter_mut().zip(acts.iter().chain(std::iter::repeat(&Action::READY))) {
                *slot = *a;
            }
        } else {
            if w.step < 6 {
                // L'étape interrompue compte pour ce qu'elle a fait (le jeu
                // divise en entiers et la ramène à son début : bug corrigé,
                // voir le README).
                let a = w.actions[w.step];
                if a.dur > 0 {
                    let k = w.counter as f32 / a.dur as f32;
                    let push = |v: &mut i32, to: i32| {
                        if to != KEEP {
                            *v = (*v as f32 + k * (to - *v) as f32) as i32;
                        }
                    };
                    for (v, to) in w.rect.iter_mut().zip([a.x, a.y, a.w, a.h]) {
                        push(v, to);
                    }
                    for (v, to) in w.alpha.iter_mut().zip([a.frame, a.back]) {
                        push(v, to);
                    }
                    if a.prio != -1 {
                        w.prio = a.prio;
                    }
                    if a.style >= 0 {
                        w.style = a.style as usize;
                    }
                }
            } else if w.actions[0].dur < 0 {
                let a = acts[0];
                w.rect = [a.x, a.y, a.w, a.h];
            }
            for (slot, a) in w.actions.iter_mut().zip(acts.iter().chain(std::iter::repeat(&Action::READY))) {
                *slot = *a;
            }
            w.actions[5].dur = -1;
            let kind = w.actions[0].kind;
            if w.state == 3 {
                if kind == 2 || kind == 3 {
                    w.state = 0;
                    w.printed = false;
                }
            } else if kind == 1 {
                w.state = 1;
            }
            w.counter = 0;
        }
        w.step = 0;
    }

    /// AD_WindowProc et Cursor_Proc : une image.
    pub fn update(&mut self) {
        self.wins.retain_mut(|w| {
            if w.printed {
                w.clock += 1.0;
            }
            if w.step >= 6 {
                return true;
            }
            w.counter += 1;
            let a = w.actions[w.step];
            if w.counter < a.dur {
                return true;
            }
            if a.kind == 3 {
                return false;
            }
            let opened = w.state <= 1;
            if opened {
                w.state += 2;
            }
            w.settle(a);
            w.step += 1;
            w.counter = 0;
            if w.step < 6 && w.actions[w.step].dur < 0 {
                w.step = 6;
            } else if w.state == 3 {
                if matches!(w.actions[w.step.min(5)].kind, 2 | 3) {
                    w.state = 0;
                }
            } else if w.actions[w.step.min(5)].kind == 1 {
                w.state = 1;
            }
            match w.state {
                3 if opened => {
                    w.printed = true;
                    w.clock = 0.0;
                    w.skip = false;
                }
                0 => w.printed = false,
                _ => {}
            }
            true
        });
        self.cursors.retain_mut(Cursor::update);
    }

    fn cursor(&mut self, name: &str) -> Option<&mut Cursor> {
        self.cursors.iter_mut().find(|c| c.name == name)
    }

    /// Cursor_Make : un curseur paraît en `(x, y)` (son centre), son
    /// opacité montant vers `alpha` en `dur` images ; sans effet si ce nom
    /// existe déjà ou si les dix places sont prises.
    pub fn cursor_make(&mut self, name: &'static str, (x, y): (i32, i32), prio: i32, alpha: i32, dur: i32) {
        if self.cursors.len() >= 10 || self.cursors.iter().any(|c| c.name == name) {
            return;
        }
        let dur = dur.max(1);
        self.cursors.push(Cursor { name, from: (x, y), to: (x, y), counter: 0, dur, alpha: 0, target: alpha, step: alpha / dur, state: 1, prio });
    }

    /// Cursor_Move : le curseur glisse vers `(x, y)` en `dur` images, son
    /// opacité vers `alpha` (pas un curseur estompé).
    pub fn cursor_move(&mut self, name: &str, to: (i32, i32), alpha: i32, dur: i32) {
        let Some(c) = self.cursor(name).filter(|c| c.state != 5) else { return };
        c.from = c.reached();
        c.counter = 0;
        c.dur = dur.max(1);
        c.state = 4;
        c.to = to;
        c.target = alpha;
        c.step = (alpha - c.alpha) / c.dur;
    }

    /// Cursor_Show : le curseur reparaît (opacité `alpha` en `dur` images).
    pub fn cursor_show(&mut self, name: &str, alpha: i32, dur: i32) {
        let Some(c) = self.cursor(name) else { return };
        c.target = alpha;
        c.state = 1;
        c.counter = 0;
        c.dur = dur.max(1);
        c.step = (alpha - c.alpha) / c.dur;
    }

    /// Cursor_Hidden : le curseur s'estompe à `alpha` et ne bouge plus. Le
    /// jeu compte le pas sur la durée précédente (1 le plus souvent : le
    /// curseur s'estompe d'un coup).
    pub fn cursor_hide(&mut self, name: &str, alpha: i32, dur: i32) {
        let Some(c) = self.cursor(name) else { return };
        c.target = alpha;
        c.state = 5;
        c.step = (alpha - c.alpha) / c.dur;
        c.counter = 0;
        c.dur = dur.max(1);
    }

    /// Cursor_Del : le curseur s'arrête où il en est et disparaît en `dur`
    /// images.
    pub fn cursor_del(&mut self, name: &str, dur: i32) {
        let Some(c) = self.cursor(name) else { return };
        c.from = c.reached();
        c.to = c.from;
        c.counter = 0;
        c.dur = dur.max(1);
        c.state = 2;
        c.target = 0;
        c.step = -c.alpha / c.dur;
        if c.step == 0 {
            c.step = -1;
        }
    }

    /// AD_WindowAllTran, RunFontSystem et Cursor_AllTran.
    pub fn draw(&self, gfx: &mut Gfx, atlas: Option<&Atlas>) {
        for w in self.wins.iter().filter(|w| w.state != 2) {
            let (rect, [frame, back], t) = w.now();
            let shape = if w.state != 3 && t < 1.0 { w.zoomed(rect, t) } else { rect };
            // L'ouverture 2 : en fondu.
            let fade = match w.state {
                1 if w.zoom == 2 => t,
                0 if w.zoom == 2 => 1.0 - t,
                _ => 1.0,
            };
            masu9(gfx, w.style, shape, w.prio, (frame as f32 * fade) as i32, (back as f32 * fade) as i32);
            if w.printed
                && let Some(atlas) = atlas
            {
                let origin = (rect[0] - (rect[2] >> 1) + w.text.offset.0, rect[1] - (rect[3] >> 1) + w.text.offset.1);
                atlas.draw(gfx, &w.text.glyphs, origin, w.prio, 0, w.text.shadow, (!w.skip).then_some(w.clock));
            }
        }
        for c in &self.cursors {
            let (x, y) = c.now();
            gfx.tran_center(Some(CURSOR), 1.0, [7.0, 0.0, 41.0, 34.0], [x as f32, y as f32, 41.0, 34.0], c.prio, c.alpha, GREY, 0.0, None);
        }
    }

    /// AD_WinTxtSet et AD_WinTxtSetEnd : un nouveau texte, qui apparaît
    /// depuis le début si la fenêtre est ouverte.
    pub fn set_text(&mut self, name: &str, glyphs: Vec<Glyph>) {
        if let Some(w) = self.get(name) {
            w.text.glyphs = glyphs;
            w.clock = 0.0;
            w.skip = false;
        }
    }

    /// MkFontTxtTimeGet : les images qu'il faut encore au texte pour
    /// paraître en entier (0 si la fenêtre n'est pas ouverte).
    pub fn text_time(&self, name: &str) -> i32 {
        self.wins.iter().find(|w| w.name == name && w.printed && !w.skip).map_or(0, |w| {
            let last = w.text.glyphs.iter().map(|g| g.time).fold(0.0f32, f32::min);
            (-(last + w.clock)).max(0.0) as i32
        })
    }

    /// AD_WinTxtNumGet : le nombre de glyphes du texte (0 si la fenêtre
    /// n'est pas ouverte).
    pub fn text_len(&self, name: &str) -> i32 {
        self.wins.iter().find(|w| w.name == name && w.state == 3).map_or(0, |w| w.text.glyphs.len() as i32)
    }

    /// AD_WinTxtStateSet(0, …) : le texte paraîtra d'un coup (vitesse 0), à
    /// la prochaine ouverture de la fenêtre.
    pub fn instant_text(&mut self, name: &str) {
        if let Some(w) = self.get(name) {
            for g in &mut w.text.glyphs {
                g.time = 0.0;
            }
        }
    }

    /// AD_WinTxtSpeedChange(0) : tout le texte d'un coup.
    pub fn skip_text(&mut self, name: &str) {
        if let Some(w) = self.get(name) {
            w.skip = true;
        }
    }

    /// La fenêtre est-elle ouverte (ou en train de s'ouvrir) ?
    pub fn is_open(&self, name: &str) -> bool {
        self.wins.iter().any(|w| w.name == name && matches!(w.state, 1 | 3))
    }

    /// AD_InformationWinMake : un message au centre de l'écran, à la
    /// taille de son texte, ouvert en `dur` images ; l'ombre en palette 8.
    pub fn inform(&mut self, glyphs: Vec<Glyph>, (cols, lines): (i32, i32), dur: i32, prio: i32) {
        let w = (f64::from(cols) * 12.5 + 44.0) as i32;
        let h = lines * 30 + 10;
        self.make("infor", 1, prio, Some(8), (0x14, 6), glyphs);
        self.act("infor", &[Action { dur, kind: 1, x: 0x140, y: 0xE0, w, h, frame: 0x7F, back: 0x50, ..Action::READY }]);
    }

    /// AD_InformationWinDel : le message reste 10 images, puis rétrécit
    /// jusqu'à disparaître en `dur` images.
    pub fn close_information(&mut self, dur: i32) {
        self.act("infor", &[Action { dur: 10, kind: 1, ..Action::READY }, Action { dur, kind: 3, w: 0, h: 0, ..Action::READY }]);
    }

    /// AD_CheckWinMake : la question (style 4) au-dessus de はい (style 4)
    /// et いいえ (style 3, choisi), et le curseur sur いいえ.
    pub fn ask(&mut self, question: Vec<Glyph>, yes: Vec<Glyph>, no: Vec<Glyph>, size: (i32, i32)) {
        self.choice = 1;
        let open = Action { dur: 10, kind: 1, frame: 0x7F, back: 0x7F, ..Action::READY };
        self.make("check", 4, 0xEB, None, (8, 5), question);
        self.act("check", &[Action { x: 0x140, y: 0xB4, w: size.0, h: size.1, ..open }]);
        let button = Action { y: ANSWER_Y, w: ANSWER_SIZE.0, h: ANSWER_SIZE.1, ..open };
        self.make("win0", 4, 0xEB, None, (0xF, 5), yes);
        self.act("win0", &[Action { x: ANSWER_X[0], ..button }]);
        self.make("win1", 3, 0xEB, None, (0x16, 5), no);
        self.act("win1", &[Action { x: ANSWER_X[1], frame: 100, back: 100, ..button }]);
        self.cursor_make(CHECK_CURSOR, (0x14A, ANSWER_Y), 0xEC, 0x7F, 1);
    }

    /// AD_CheckWinDel : tout se ferme en 10 images.
    fn close_question(&mut self) {
        let close = [Action { dur: 10, kind: 3, ..Action::READY }];
        for name in ["check", "win0", "win1"] {
            self.act(name, &close);
        }
        self.cursor_del(CHECK_CURSOR, 10);
    }

    /// AD_CheckWinChange : la réponse choisie passe au style 3, l'autre au
    /// style 4 ; le curseur glisse dessous en 6 images.
    fn choose(&mut self, choice: i32) {
        let name = |n: i32| if n == 0 { "win0" } else { "win1" };
        self.act(name(choice), &[Action { dur: 1, frame: 100, back: 100, style: 3, ..Action::READY }]);
        self.act(name(1 - choice), &[Action { dur: 1, frame: 0x80, back: 0x80, style: 4, ..Action::READY }]);
        self.cursor_move(CHECK_CURSOR, (choice * 0xB4 + 0x96, 0xF4), 0x80, 6);
    }

    /// AD_CheckWinProc : ○ répond, × répond いいえ, ← → changent de
    /// réponse ; la souris choisit la réponse survolée, un clic la donne.
    /// Rend la réponse donnée.
    pub fn answer(&mut self, sys: &mut Sys) -> Option<i32> {
        let zones: [Zone<i32>; 2] = std::array::from_fn(|n| Zone::centered(ANSWER_X[n] as f32, ANSWER_Y as f32, ANSWER_SIZE.0 as f32, ANSWER_SIZE.1 as f32, n as i32));
        let pick = pointer::pick(sys, &zones);
        if let Some(n) = pick.clicked.or(pick.hovered).filter(|&n| n != self.choice) {
            self.choice = n;
            sys.menu_sound(1);
            self.choose(n);
        }
        let before = self.choice;
        if sys.menu_key(0x1F) || pick.clicked.is_some() && sys.lock <= 0 {
            self.close_question();
            sys.lock_menu(0xC);
            sys.menu_sound(2);
            return Some(self.choice);
        }
        if sys.menu_key(0x1E) {
            self.close_question();
            sys.lock_menu(0xC);
            sys.menu_sound(0);
            return Some(1);
        }
        if sys.input.direct(2, 0x14, 7, true) {
            self.choice -= 1;
        } else if sys.input.direct(3, 0x14, 7, true) {
            self.choice += 1;
        }
        if self.choice != before {
            self.choice = self.choice.rem_euclid(2);
            sys.menu_sound(1);
            self.choose(self.choice);
        }
        None
    }
}
