//! Les dessins d'une image, comme les fait le jeu (GP_Tran, GP_Tran_Center,
//! GP_Paint) : chacun a une priorité, et l'image est dessinée de la plus
//! basse à la plus haute (à priorité égale, dans l'ordre des appels).
//! Couleurs et opacités du GS : 0x80 vaut 1.

use std::collections::HashMap;

use miniquad::TextureId;

pub use crate::render::Blend;
use crate::render::Renderer;

/// Une image du jeu chargée : sa texture et sa taille.
#[derive(Clone, Copy)]
struct Tex {
    id: TextureId,
    width: f32,
    height: f32,
}

struct Sprite {
    texture: Option<TextureId>,
    corners: [(f32, f32); 4],
    uv: [(f32, f32); 4],
    color: [f32; 4],
    blend: Blend,
    prio: i32,
}

/// Couleur et opacité d'un dessin (0x80 = 1), prémultipliées.
fn color(alpha: i32, rgb: [i32; 3]) -> [f32; 4] {
    let a = alpha.clamp(0, 0x80) as f32 / 128.0;
    let c = |v: i32| v.clamp(0, 0xFF) as f32 / 128.0 * a;
    [c(rgb[0]), c(rgb[1]), c(rgb[2]), a]
}

/// Couleur neutre des dessins (0x7f, comme les appels du jeu).
pub const GREY: [i32; 3] = [0x7F; 3];

#[derive(Default)]
pub struct Gfx {
    textures: HashMap<String, Tex>,
    sprites: Vec<Sprite>,
    /// Des images à envoyer à la carte graphique avant la prochaine image,
    /// et des textures à libérer.
    uploads: Vec<(String, crate::data::Image)>,
    drops: Vec<String>,
}

impl Gfx {
    pub fn add_texture(&mut self, name: &str, id: TextureId, width: u32, height: u32) {
        self.textures.insert(name.to_string(), Tex { id, width: width as f32, height: height as f32 });
    }

    /// Une image chargée en cours de jeu (un décor, un personnage, les
    /// glyphes d'une page) : envoyée avant la prochaine image, à la place
    /// d'une image de même nom.
    pub fn load(&mut self, name: &str, image: crate::data::Image) {
        self.uploads.retain(|(n, _)| n != name);
        self.uploads.push((name.to_string(), image));
    }

    /// L'image est-elle chargée (ou en passe de l'être) ?
    pub fn has(&self, name: &str) -> bool {
        self.textures.contains_key(name) || self.uploads.iter().any(|(n, _)| n == name)
    }

    /// Libère une image chargée en cours de jeu.
    pub fn unload(&mut self, name: &str) {
        self.uploads.retain(|(n, _)| n != name);
        self.drops.push(name.to_string());
    }

    /// Les images à envoyer et à libérer (avant de dessiner l'image).
    pub fn upload(&mut self, renderer: &mut Renderer) {
        for name in self.drops.drain(..) {
            if let Some(t) = self.textures.remove(&name) {
                renderer.delete(t.id);
            }
        }
        for (name, img) in self.uploads.drain(..) {
            let id = renderer.texture(img.width, img.height, &img.rgba);
            if let Some(old) = self.textures.insert(name, Tex { id, width: img.width as f32, height: img.height as f32 }) {
                renderer.delete(old.id);
            }
        }
    }

    /// Les coordonnées de texture d'un rectangle source (pixels). Le GS
    /// ajoute un demi-texel (u × 16 + 8) parce que ses pixels ont leur
    /// centre sur les entiers ; ceux d'OpenGL l'ont déjà au milieu : sans
    /// rien ajouter, chaque pixel tombe aussi au centre d'un texel.
    fn uv(t: &Tex, src: [f32; 4]) -> [(f32, f32); 4] {
        let [u, v, w, h] = src;
        let (u0, v0, u1, v1) = (u / t.width, v / t.height, (u + w) / t.width, (v + h) / t.height);
        [(u0, v0), (u1, v0), (u0, v1), (u1, v1)]
    }

    #[allow(clippy::too_many_arguments)]
    fn push(&mut self, texture: Option<&str>, src: [f32; 4], corners: [(f32, f32); 4], prio: i32, alpha: i32, blend: Blend, rgb: [i32; 3]) {
        let (texture, uv) = match texture {
            Some(name) => {
                let Some(t) = self.textures.get(name).copied() else { return };
                (Some(t.id), Gfx::uv(&t, src))
            }
            None => (None, [(0.0, 0.0); 4]),
        };
        self.sprites.push(Sprite { texture, corners, uv, color: color(alpha, rgb), blend, prio });
    }

    /// GP_Tran : `src` (u, v, l, h) de l'image vers `dst` (x, y, l, h, coin
    /// haut gauche) ; une largeur ou une hauteur négative retourne l'image.
    #[allow(clippy::too_many_arguments)]
    pub fn tran(&mut self, texture: &str, src: [f32; 4], dst: [f32; 4], prio: i32, alpha: i32, blend: Blend, rgb: [i32; 3]) {
        let [x, y, w, h] = dst;
        if w == 0.0 || h == 0.0 || src[2] == 0.0 || src[3] == 0.0 {
            return;
        }
        let corners = [(x.floor(), y.floor()), (x.floor() + w, y.floor()), (x.floor(), y.floor() + h), (x.floor() + w, y.floor() + h)];
        self.push(Some(texture), src, corners, prio, alpha, blend, rgb);
    }

    /// GP_Tran_Center : `dst` donne le centre (x, y) et la taille ; agrandi
    /// de `scale` autour de `pivot` (son propre centre par défaut), tourné
    /// de `angle` degrés (sens inverse des aiguilles d'une montre).
    #[allow(clippy::too_many_arguments)]
    pub fn tran_center(&mut self, texture: Option<&str>, scale: f32, src: [f32; 4], dst: [f32; 4], prio: i32, alpha: i32, rgb: [i32; 3], angle: f32, pivot: Option<(f32, f32)>) {
        let [x, y, w, h] = dst;
        if w == 0.0 || h == 0.0 {
            return;
        }
        let (px, py) = pivot.unwrap_or((x, y));
        let (x0, y0, x1, y1) = (x - w / 2.0, y - h / 2.0, x + w / 2.0, y + h / 2.0);
        let corners = if angle == 0.0 {
            // GP_AngleProc : coin et taille en pixels entiers.
            let (left, top) = ((px + scale * (x0 - px)).floor(), (py + scale * (y0 - py)).floor());
            let (w, h) = ((w * scale).trunc(), (h * scale).trunc());
            [(left, top), (left + w, top), (left, top + h), (left + w, top + h)]
        } else {
            let (sin, cos) = (-angle).to_radians().sin_cos();
            let corner = |cx: f32, cy: f32| {
                let (dx, dy) = ((cx - px) * scale, (cy - py) * scale);
                ((px + dx * cos - dy * sin).floor(), (py + dx * sin + dy * cos).floor())
            };
            [corner(x0, y0), corner(x1, y0), corner(x0, y1), corner(x1, y1)]
        };
        self.push(texture, src, corners, prio, alpha, Blend::Normal, rgb);
    }

    /// GP_Paint : un rectangle uni (coin haut gauche).
    pub fn paint(&mut self, dst: [f32; 4], prio: i32, alpha: i32, rgb: [i32; 3]) {
        let [x, y, w, h] = dst;
        self.push(None, [0.0; 4], [(x, y), (x + w, y), (x, y + h), (x + w, y + h)], prio, alpha, Blend::Normal, rgb);
    }

    /// L'image : les dessins par priorité croissante.
    pub fn render(&mut self, renderer: &mut Renderer) {
        self.sprites.sort_by_key(|s| s.prio);
        for s in self.sprites.drain(..) {
            renderer.quad(s.texture, s.corners, s.uv, s.color, s.blend);
        }
    }
}
