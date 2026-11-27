//! La souris, ajoutée par le portage (la PS2 n'en avait pas). Chaque écran
//! déclare ses zones — un rectangle de l'écran et l'élément qu'il désigne —
//! d'après les mêmes positions que son dessin ; `pick` rend l'élément
//! survolé et l'élément cliqué. Partout, survoler un élément fait comme y
//! amener le curseur, cliquer dessus comme ○ sur lui : chaque écran le
//! traduit en ce que ferait la manette, sans toucher à sa logique.
//!
//! En debug, DOKURO_ZONES=1 dessine le contour des zones de l'image.

use crate::gfx::Gfx;
use crate::sys::Sys;

/// Un rectangle de l'écran (coin haut gauche, taille ; points de l'écran
/// 640 × 448) et ce qu'il désigne.
#[derive(Clone, Copy, Debug)]
pub struct Zone<T> {
    pub rect: [f32; 4],
    pub target: T,
}

impl<T> Zone<T> {
    pub fn new(rect: [f32; 4], target: T) -> Zone<T> {
        Zone { rect, target }
    }

    /// Une zone donnée par son centre, comme les dessins centrés du jeu.
    pub fn centered(cx: f32, cy: f32, w: f32, h: f32, target: T) -> Zone<T> {
        Zone { rect: [cx - w / 2.0, cy - h / 2.0, w, h], target }
    }

    fn contains(&self, (x, y): (f32, f32)) -> bool {
        let [zx, zy, w, h] = self.rect;
        x >= zx && x < zx + w && y >= zy && y < zy + h
    }
}

/// L'élément sous la souris, et celui que touche le clic de l'image.
#[derive(Clone, Copy, Debug)]
pub struct Pick<T> {
    pub hovered: Option<T>,
    pub clicked: Option<T>,
}

/// L'élément survolé et l'élément cliqué parmi les zones d'un écran (la
/// première qui contient le point). Les zones sont notées pour
/// DOKURO_ZONES.
pub fn pick<T: Copy>(sys: &mut Sys, zones: &[Zone<T>]) -> Pick<T> {
    sys.zones.extend(zones.iter().map(|z| z.rect));
    let at = |p: Option<(f32, f32)>| p.and_then(|p| zones.iter().find(|z| z.contains(p)).map(|z| z.target));
    Pick { hovered: at(sys.mouse), clicked: at(sys.click) }
}

/// DOKURO_ZONES (debug) : le contour de chaque zone, par-dessus tout.
pub fn draw(gfx: &mut Gfx, zones: &[[f32; 4]]) {
    const PRIO: i32 = 0x2000;
    const RGB: [i32; 3] = [0xFF, 0x20, 0x20];
    for &[x, y, w, h] in zones {
        for edge in [[x, y, w, 1.0], [x, y + h - 1.0, w, 1.0], [x, y, 1.0, h], [x + w - 1.0, y, 1.0, h]] {
            gfx.paint(edge, PRIO, 0x80, RGB);
        }
    }
}
