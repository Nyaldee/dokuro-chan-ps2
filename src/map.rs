//! La carte des journées (AD_Map*) : quatre lieux, chacun avec sa vignette,
//! son état (0 caché, 1 grisé, 2 choisissable), son script et la valeur du
//! drapeau « switch » qu'il donne, son commentaire ; les têtes des
//! personnages présents ; un fond qui défile. Les lieux sont posés par le
//! script de la carte (map_pos_*), lancé avant chaque carte.

use crate::engine::{EffectDef, Effects, Key};
use crate::gfx::{Blend, Gfx, GREY};
use crate::pointer::{self, Pick, Zone};
use crate::story::World;
use crate::sys::Sys;

/// Les images de la carte (SYSTEM.UNI 30, 31, 32).
pub const PRTS1: &str = "map_prts1";
pub const PRTS2: &str = "map_prts2";
pub const BACK: &str = "map_back";

/// Un lieu (AD_MapInit) : la vignette (dans PRTS1, ou PRTS2 grisée), son
/// centre, et le lieu voisin dans chaque direction (haut, bas, gauche,
/// droite ; AD_MapPosCursorMoveSet).
type Place = ([f32; 4], (f32, f32), [usize; 4]);

const PLACES: [Place; 4] = [
    ([0.0, 0.0, 209.0, 158.0], (180.0, 138.0), [3, 2, 3, 1]),
    ([210.0, 0.0, 209.0, 158.0], (460.0, 138.0), [2, 3, 0, 2]),
    ([0.0, 159.0, 209.0, 158.0], (180.0, 315.0), [0, 1, 1, 3]),
    ([210.0, 159.0, 209.0, 158.0], (460.0, 315.0), [1, 0, 2, 0]),
];
/// Les têtes des cinq personnages (AD_MapMarkDataSet, dans PRTS2).
const MARKS: [[f32; 4]; 5] = [[0.0, 392.0, 76.0, 62.0], [192.0, 392.0, 81.0, 62.0], [77.0, 392.0, 59.0, 62.0], [274.0, 392.0, 63.0, 62.0], [137.0, 392.0, 54.0, 62.0]];
/// Les cadres du curseur (PRTS1) : le lieu choisi (rouge), les autres
/// (bleus).
const FRAME_ON: [f32; 4] = [222.0, 318.0, 217.0, 167.0];
const FRAME_OFF: [f32; 4] = [0.0, 318.0, 216.0, 165.0];

pub struct Map {
    /// Le lieu choisi (0x1c27f00), et le défilement du fond.
    pub cursor: usize,
    scroll: i32,
}

impl Map {
    /// AD_MapStart : le curseur sur le premier lieu choisissable.
    pub fn start(world: &World) -> Map {
        let cursor = (0..4).find(|&k| state(world, k) == 2).unwrap_or(0);
        Map { cursor, scroll: 0x3C }
    }

    /// AD_MapProc : les flèches (AD_MapPosCursorMoveSet : on va de voisin
    /// en voisin jusqu'à un lieu choisissable), la souris sur une vignette
    /// (AD_MapHitJudgProc). Rend le lieu choisi par ○ ou un clic.
    pub fn step(&mut self, sys: &mut Sys, world: &World) -> Option<usize> {
        self.scroll = (self.scroll + 1) % 0x27F;
        // AD_MapHitJudgProc : les vignettes des lieux choisissables.
        let zones: Vec<_> = (0..4).filter(|&k| state(world, k) == 2).map(|k| {
            let (icon, (cx, cy), _) = PLACES[k];
            Zone::centered(cx, cy, icon[2], icon[3], k)
        }).collect();
        let Pick { hovered, clicked } = pointer::pick(sys, &zones);
        if let Some(k) = hovered {
            self.cursor = k;
        }
        if sys.input.key(0x1F) || clicked.is_some() {
            return Some(self.cursor);
        }
        let vertical = if sys.input.direct(0, 10, 10, true) {
            Some(0)
        } else if sys.input.direct(1, 10, 10, true) {
            Some(1)
        } else {
            None
        };
        let horizontal = if sys.input.direct(2, 10, 10, true) {
            Some(2)
        } else if sys.input.direct(3, 10, 10, true) {
            Some(3)
        } else {
            None
        };
        for dir in [vertical, horizontal].into_iter().flatten() {
            // AD_MapCursorProc2 : de voisin en voisin (le jeu boucle sans
            // fin s'il n'y a aucun lieu choisissable ; il y en a toujours).
            let mut k = self.cursor;
            for _ in 0..4 {
                k = PLACES[k].2[dir];
                if state(world, k) == 2 {
                    self.cursor = k;
                    break;
                }
            }
        }
        None
    }

    /// AD_MapStEffectMake : la carte arrive par-dessus la scène, en effets
    /// qui s'effacent au bout de 60 images. Le fond (deux copies côte à
    /// côte) défile d'un pixel par image, invisible 20 images puis opaque en
    /// 20 ; chaque vignette (la k-ième 5k images plus tard) paraît en
    /// grossissant à 1,3 en 10 images, revient à 1 (1,1 pour le lieu choisi)
    /// en 10, et les autres s'assombrissent (100) ; chaque cadre paraît dans
    /// un éclair blanc, 10 images après sa vignette (le choisi de 1,8 à 1,1).
    /// Rend la durée de l'arrivée (0x3e images), après laquelle la carte se
    /// dessine.
    pub fn arrival(&self, world: &World, effects: &mut Effects) -> i32 {
        let mut back = EffectDef::ready();
        back.mode = 0;
        back.texture = Some(BACK);
        back.src = [0.0, 0.0, 512.0, 448.0];
        back.size = (640.0, 448.0);
        back.prio = 0x5F;
        back.alpha = 0;
        back.speed = 1.0;
        back.pos = (0x140, 0xE0);
        back.keys[0] = Key { dur: 0x14, alpha: 0, ..Key::NONE };
        back.keys[1] = Key { dur: 0x14, alpha: 0x7F, ..Key::NONE };
        back.keys[2] = Key { dur: 0x14, alpha: 0x7F, ..Key::NONE };
        effects.make(back.clone());
        back.pos.0 -= 0x280;
        effects.make(back);
        for (k, &(icon, (cx, cy), _)) in PLACES.iter().enumerate() {
            let s = state(world, k);
            if s != 1 && s != 2 {
                continue;
            }
            let chosen = k == self.cursor;
            let n = k as i32;
            let mut d = EffectDef::ready();
            d.texture = Some(if s == 1 { PRTS2 } else { PRTS1 });
            d.src = icon;
            d.size = (icon[2], icon[3]);
            d.prio = 100;
            d.pos = (cx as i32, cy as i32);
            d.alpha = 0;
            d.delay = n * 5 + 0x14;
            d.keys[0] = Key { dur: 10, scale: 1.3, ..Key::NONE };
            d.keys[1] = Key { dur: 10, scale: if chosen { 1.1 } else { 1.0 }, ..Key::NONE };
            d.keys[2] = if chosen { Key { dur: 0x14 - n * 5, scale: 1.1, ..Key::NONE } } else { Key { dur: 0x14 - n * 5, rgb: [100; 3], ..Key::NONE } };
            effects.make(d);
            if s == 2 {
                let mut f = EffectDef::ready();
                let frame = if chosen { FRAME_ON } else { FRAME_OFF };
                let scale = if chosen { 1.1 } else { 1.0 };
                f.texture = Some(PRTS1);
                f.src = frame;
                f.size = (frame[2], frame[3]);
                f.prio = 0x65;
                f.pos = (cx as i32, cy as i32);
                f.alpha = 0;
                f.scale = if chosen { 1.8 } else { 1.0 };
                f.delay = n * 5 + 0x1E;
                f.keys[0] = Key { dur: 10, rgb: [0xFF; 3], scale, ..Key::NONE };
                f.keys[1] = Key { dur: 0x14 - n * 5, scale, ..Key::NONE };
                f.keys[2] = Key { scale, ..Key::NONE };
                effects.make(f);
            }
        }
        0x3E
    }

    /// AD_MapSystemTran : le fond qui défile (AD_MapTran), les cadres
    /// (AD_MapCursorTran), les vignettes et les têtes (AD_MapIconTran).
    pub fn draw(&self, gfx: &mut Gfx, world: &World) {
        for x in [self.scroll, self.scroll - 0x27F] {
            gfx.tran(BACK, [0.0, 0.0, 512.0, 448.0], [x as f32, 0.0, 640.0, 448.0], 0x5F, 0x7F, Blend::Normal, GREY);
        }
        for (k, &(icon, (cx, cy), _)) in PLACES.iter().enumerate() {
            let s = state(world, k);
            if s == 0 {
                continue;
            }
            let chosen = k == self.cursor;
            if s == 2 {
                let (frame, scale) = if chosen { (FRAME_ON, 1.1) } else { (FRAME_OFF, 1.0) };
                gfx.tran_center(Some(PRTS1), scale, frame, [cx, cy, frame[2], frame[3]], 0x65, 0x7F, GREY, 0.0, None);
            }
            let texture = if s == 1 { PRTS2 } else { PRTS1 };
            let (scale, alpha) = if chosen { (1.1, 0x7F) } else { (1.0, 100) };
            gfx.tran_center(Some(texture), scale, icon, [cx, cy, icon[2], icon[3]], 100, alpha, GREY, 0.0, None);
            if s == 1 {
                continue;
            }
            // Les têtes, de droite à gauche depuis le coin bas droit.
            let (w, h) = (icon[2], icon[3]);
            let (mut x, y) = if chosen { (cx + w / 2.0 * 1.1 - 10.0, cy + h / 2.0 * 1.1 - 8.0) } else { (cx + w / 2.0 - 5.0, cy + h / 2.0 - 5.0) };
            for (m, src) in MARKS.iter().enumerate() {
                if mark(world, m) == Some(k) {
                    x -= (src[2] as i32 >> 1) as f32;
                    gfx.tran_center(Some(PRTS2), 1.1, *src, [x, y - (src[3] as i32 >> 1) as f32, src[2], src[3]], 100, 0x7F, GREY, 0.0, None);
                    x -= (src[2] as i32 >> 1) as f32;
                }
            }
        }
    }
}

/// AD_MapOkEffectMake : le lieu choisi. Sa vignette clignote deux fois
/// par-dessus elle-même (à l'échelle 1,1 : rose puis blanche, de 0 à 0x50
/// d'opacité et retour, quatre fois 5 images) ; trois cadres rouges, à 10
/// images d'écart, s'agrandissent de 1,1 à 1,4 en s'effaçant (0x14 images).
pub fn chosen(k: usize, effects: &mut Effects) {
    let Some(&(icon, (cx, cy), _)) = PLACES.get(k) else { return };
    let pos = (cx as i32, cy as i32);
    let mut d = EffectDef::ready();
    d.texture = Some(PRTS1);
    d.src = icon;
    d.size = (icon[2], icon[3]);
    d.prio = 0x65;
    d.pos = pos;
    d.alpha = 0;
    d.scale = 1.1;
    let flash = |alpha, rgb| Key { dur: 5, alpha, rgb, scale: 1.1, ..Key::NONE };
    d.keys[0] = flash(0x50, [0xFF, 0xDC, 0xDC]);
    d.keys[1] = flash(0, [0x7F; 3]);
    d.keys[2] = flash(0x50, [0xDC; 3]);
    d.keys[3] = flash(0, [0x7F; 3]);
    effects.make(d);
    let mut f = EffectDef::ready();
    f.texture = Some(PRTS1);
    f.src = FRAME_ON;
    f.size = (FRAME_ON[2], FRAME_ON[3]);
    f.prio = 0x66;
    f.pos = pos;
    f.alpha = 0x50;
    f.scale = 1.1;
    f.keys[0] = Key { dur: 0x14, alpha: 0, rgb: [0xFF; 3], scale: 1.4, ..Key::NONE };
    for n in 0..3 {
        f.delay = n * 10;
        effects.make(f.clone());
    }
}

/// L'état d'un lieu (0 s'il n'est pas posé).
fn state(world: &World, k: usize) -> i32 {
    world.places.get(k).map_or(0, |p| p.state)
}

/// Le lieu où est la tête d'un personnage (la dernière place posée).
fn mark(world: &World, m: usize) -> Option<usize> {
    world.marks.iter().rev().find(|&&(n, _)| n == m as i32).and_then(|&(_, p)| usize::try_from(p).ok())
}
