//! Le moteur du jeu dont se sert le mini-jeu : manette (AD_InputKey),
//! bruitages différés (AD_SEReservePlay), effets à images-clés
//! (AD_EffectMake), caméra du décor (AD_BGTargetSet, AD_CameraProc), fondus
//! au noir (AD_BlackOut), nombres aléatoires.

use crate::gfx::Gfx;

/// L'écran de la PS2.
pub const SCREEN: (f32, f32) = (640.0, 448.0);

// --- Manette --------------------------------------------------------------

/// Les boutons, numérotés comme le jeu (AD_InputKey2) : △ □ × ○, L1 R1 L2
/// R2, Start, croix.
#[derive(Clone, Copy)]
pub enum Button {
    Triangle = 0,
    Square = 1,
    Cross = 2,
    Circle = 3,
    L1 = 4,
    R1 = 5,
    L2 = 6,
    R2 = 7,
    Start = 8,
    /// Échap : le menu du jeu (□) pendant l'histoire, le retour (×)
    /// ailleurs, comme dans un visual novel sur PC (hors du jeu de la PS2).
    Menu = 9,
    /// Deux touches du clavier propres à la course (Espace et Ctrl), qui y
    /// valent ↑ (sauter) et ↓ (glisser) : hors du jeu de la PS2. Dans
    /// l'histoire, Ctrl tenu avance rapidement.
    Jump = 10,
    Slide = 11,
    Up = 0xC,
    Right = 0xD,
    Down = 0xE,
    Left = 0xF,
    /// La molette de la souris, un cran vers le haut ou vers le bas (hors du
    /// jeu de la PS2 : chaque partie du jeu en fait ce qu'il faut).
    WheelUp = 0x10,
    WheelDown = 0x11,
    /// A : la lecture automatique dans l'histoire, L2 ailleurs (hors du jeu
    /// de la PS2).
    Auto = 0x12,
}

/// Le nombre de boutons : ceux de la PS2 et les touches propres au PC.
pub const BUTTONS: usize = 0x13;

/// Depuis combien d'images chaque bouton est tenu.
#[derive(Default)]
pub struct Input {
    held: [u32; BUTTONS],
}

impl Input {
    /// Une image : les boutons tenus.
    pub fn update(&mut self, down: [bool; BUTTONS]) {
        for (h, d) in self.held.iter_mut().zip(down) {
            *h = if d { *h + 1 } else { 0 };
        }
    }

    fn pressed(&self, b: usize) -> bool {
        self.held[b] == 1
    }

    /// Le bouton vient d'être appuyé (les touches propres au PC comprises).
    pub fn tapped(&self, b: Button) -> bool {
        self.pressed(b as usize)
    }

    /// AD_InputKey : 0 à 9, un bouton (△ □ × ○ …) ; les touches logiques
    /// (manette réglée comme au départ) : 0x1e et 0x20 retour (×), 0x1f et
    /// 0x21 valider (○), 0x26 □, 0x27 L1, 0x28 R1. Vrai à l'image de
    /// l'appui.
    pub fn key(&self, n: i32) -> bool {
        match n {
            0..=9 => self.pressed((n % 10) as usize),
            0x1E | 0x20 => self.pressed(Button::Cross as usize),
            0x1F | 0x21 => self.pressed(Button::Circle as usize),
            0x26 => self.pressed(Button::Square as usize),
            0x27 => self.pressed(Button::L1 as usize),
            0x28 => self.pressed(Button::R1 as usize),
            _ => false,
        }
    }

    /// AD_InputKey2 (n, delay, -1) == 2 : le bouton n (0 à 9) est tenu
    /// depuis plus de `delay` images.
    pub fn held(&self, n: usize, delay: u32) -> bool {
        self.held[n] > delay
    }

    /// AD_InputDirect : direction 0 haut, 1 bas, 2 gauche, 3 droite ; à
    /// l'appui (`immediate`), puis tenue plus de `delay` images, toutes les
    /// `interval` images.
    pub fn direct(&self, dir: i32, delay: u32, interval: u32, immediate: bool) -> bool {
        let b = [Button::Up, Button::Down, Button::Left, Button::Right][(dir & 3) as usize] as usize;
        let h = self.held[b];
        (immediate && h == 1) || (h > delay && h.is_multiple_of(interval.max(1)))
    }
}

// --- Nombres aléatoires ---------------------------------------------------

/// rand() de la bibliothèque C du jeu (newlib) : un générateur congruentiel
/// de 64 bits, les bits 32 à 62 en sortie.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed)
    }

    pub fn next(&mut self) -> i32 {
        self.0 = self.0.wrapping_mul(0x5851_F42D_4C95_7F2D).wrapping_add(1);
        ((self.0 >> 32) & 0x7FFF_FFFF) as i32
    }
}

// --- Effets ---------------------------------------------------------------

/// Une image-clé d'effet : durée (images), opacité, couleur, échelle et
/// position visées (-5000 : inchangée), le freinage de la vitesse en mode 0
/// (par image ; +0xc), et la rotation ajoutée à chaque image tant qu'elle
/// dure (degrés ; +0x14).
#[derive(Clone, Copy)]
pub struct Key {
    pub dur: i32,
    pub alpha: i32,
    pub rgb: [i32; 3],
    pub scale: f32,
    pub x: i32,
    pub y: i32,
    pub brake: f32,
    pub spin: f32,
}

/// Position inchangée.
pub const KEEP: i32 = -5000;

impl Key {
    /// AD_EffectGetReady : une image-clé vide.
    pub const NONE: Key = Key { dur: -1, alpha: 0x80, rgb: [0x80; 3], scale: 1.0, x: KEEP, y: KEEP, brake: 0.0, spin: 0.0 };
}

/// Un effet tel qu'on le crée (AD_EffectGetReady puis les champs voulus).
#[derive(Clone)]
pub struct EffectDef {
    /// 0 : supprimé à la fin ; 1 : dessiné par la caméra ; 2 : fondu (gardé
    /// à la fin). Les deux derniers restent sur leur dernière image.
    pub kind: i32,
    /// Images avant de commencer.
    pub delay: i32,
    /// Interpolation de la position : 0 aucune (l'effet avance à sa vitesse),
    /// 1 linéaire, 2 cosinus, 3 en ralentissant, 4 en accélérant.
    pub mode: i32,
    /// L'image (None : rectangle uni).
    pub texture: Option<&'static str>,
    pub size: (f32, f32),
    pub prio: i32,
    pub src: [f32; 4],
    pub group: i32,
    pub alpha: i32,
    pub rgb: [i32; 3],
    pub scale: f32,
    /// En mode 0, la vitesse (pixels par image ; +0x54) et sa direction
    /// (degrés, sens inverse des aiguilles d'une montre ; +0x58).
    pub speed: f32,
    pub heading: f32,
    pub pos: (i32, i32),
    /// L'angle (degrés, sens inverse des aiguilles d'une montre ; +0x5c),
    /// que font tourner les images-clés.
    pub angle: f32,
    pub keys: [Key; 8],
}

impl EffectDef {
    pub fn ready() -> EffectDef {
        EffectDef {
            kind: 0,
            delay: 0,
            mode: 1,
            texture: None,
            size: (0.0, 0.0),
            prio: 0,
            src: [0.0; 4],
            group: 0,
            alpha: 0x7F,
            rgb: [0x7F; 3],
            scale: 1.0,
            speed: 0.0,
            heading: 0.0,
            pos: (0, 0),
            angle: 0.0,
            keys: [Key::NONE; 8],
        }
    }
}

struct Effect {
    def: EffectDef,
    stage: usize,
    counter: i32,
}

/// Image-clé « tenue » : l'effet reste sur ses dernières valeurs.
const HOLD: usize = 8;

pub struct Effects {
    slots: Vec<Option<Effect>>,
    /// Première place à essayer (la plus basse libérée).
    next: usize,
}

impl Default for Effects {
    fn default() -> Effects {
        Effects { slots: (0..200).map(|_| None).collect(), next: 0 }
    }
}

impl Effects {
    /// AD_EffectMake : place l'effet ; rend sa place.
    pub fn make(&mut self, def: EffectDef) -> Option<usize> {
        let i = (self.next..self.slots.len()).find(|&i| self.slots[i].is_none())?;
        self.slots[i] = Some(Effect { def, stage: 0, counter: 0 });
        self.next = i + 1;
        Some(i)
    }

    fn free(&mut self, i: usize) {
        self.slots[i] = None;
        self.next = self.next.min(i);
    }

    /// AD_EffectUseDel : supprime les effets de ce genre (et de ce groupe).
    pub fn delete(&mut self, kind: i32, group: Option<i32>) {
        for i in 0..self.slots.len() {
            if self.slots[i].as_ref().is_some_and(|e| e.def.kind == kind && group.is_none_or(|g| g == e.def.group)) {
                self.free(i);
            }
        }
    }

    /// Un effet de ce genre et de ce groupe est-il arrivé à sa fin ?
    pub fn held(&self, kind: i32, group: i32) -> bool {
        self.slots.iter().flatten().any(|e| e.def.kind == kind && e.def.group == group && e.stage == HOLD)
    }

    /// AD_EffectUseGet(kind, group, 1) : combien d'effets de ce genre et de
    /// ce groupe sont arrivés à leur fin.
    pub fn held_count(&self, kind: i32, group: i32) -> usize {
        self.slots.iter().flatten().filter(|e| e.def.kind == kind && e.def.group == group && e.stage == HOLD).count()
    }

    /// AD_EffectProc : une image ; puis, comme le fait AD_EffectTran avant de
    /// dessiner, un effet commencé en mode 0 avance à sa vitesse, freinée
    /// par son image-clé (la première, une fois l'effet tenu).
    pub fn step(&mut self) {
        for i in 0..self.slots.len() {
            self.count(i);
            let Some(e) = &mut self.slots[i] else { continue };
            let d = &mut e.def;
            if d.delay < 1 && d.mode == 0 {
                let (sin, cos) = d.heading.to_radians().sin_cos();
                d.pos = ((d.pos.0 as f32 + d.speed * cos) as i32, (d.pos.1 as f32 - d.speed * sin) as i32);
                d.speed = (d.speed - d.keys[if e.stage == HOLD { 0 } else { e.stage }].brake).max(0.0);
            }
        }
    }

    /// AD_EffectCountProc (ou le retard qui s'écoule) pour un effet.
    fn count(&mut self, i: usize) {
        let Some(e) = &mut self.slots[i] else { return };
        if e.def.delay >= 1 {
            e.def.delay -= 1;
            return;
        }
        if e.stage == HOLD {
            return;
        }
        let k = e.def.keys[e.stage];
        if e.counter < k.dur {
            // L'image-clé fait tourner l'effet.
            e.counter += 1;
            e.def.angle += k.spin;
            return;
        }
        // Image-clé atteinte : ses valeurs deviennent le départ.
        e.def.alpha = k.alpha;
        e.def.scale = k.scale;
        e.def.rgb = k.rgb;
        e.counter = 0;
        if e.def.mode > 0 {
            if k.x > KEEP {
                e.def.pos.0 = k.x;
            }
            if k.y > KEEP {
                e.def.pos.1 = k.y;
            }
        }
        e.stage += 1;
        if e.stage < HOLD && e.def.keys[e.stage].dur > 0 {
            return;
        }
        if e.def.kind == 0 {
            self.free(i);
        } else {
            e.stage = HOLD;
        }
    }

    /// AD_EffectTran : les effets commencés.
    pub fn draw(&self, gfx: &mut Gfx, camera: &Camera) {
        for e in self.slots.iter().flatten() {
            let d = &e.def;
            if d.delay >= 1 {
                continue;
            }
            let (t, k) = if e.stage == HOLD { (0.0, Key::NONE) } else { (e.counter as f32 / e.def.keys[e.stage].dur as f32, e.def.keys[e.stage]) };
            let lerp = |a: f32, b: f32| a + t * (b - a);
            let alpha = lerp(d.alpha as f32, k.alpha as f32) as i32;
            let scale = lerp(d.scale, k.scale);
            let along = |from: i32, to: i32| {
                let (from, to) = (from as f32, to as f32);
                if to < -4999.0 {
                    return from;
                }
                match d.mode {
                    // Mode 0 : la position ne suit que la vitesse de l'effet
                    // (voir `step`).
                    0 => from,
                    2 => from - (to - from) * ((t * 180.0).to_radians().cos() - 1.0) / 2.0,
                    3 => from + (to - from) * (t * 90.0).to_radians().sin(),
                    4 => from + (to - from) * (1.0 - (t * 90.0 + 90.0).to_radians().sin()),
                    _ => from + t * (to - from),
                }
            };
            let (x, y) = if e.stage == HOLD { (d.pos.0 as f32, d.pos.1 as f32) } else { (along(d.pos.0, k.x), along(d.pos.1, k.y)) };
            let dst = [x, y, d.size.0, d.size.1];
            if d.kind == 1 {
                camera.draw(gfx, d.texture.unwrap_or_default(), d.src, dst, d.prio, alpha);
            } else {
                // Les images comme les rectangles unis prennent la couleur
                // et l'angle (en degrés entiers) de l'effet, autour de leur
                // centre.
                let rgb = [0, 1, 2].map(|c| lerp(d.rgb[c] as f32, k.rgb[c] as f32) as i32);
                let rgb = if e.stage == HOLD { d.rgb } else { rgb };
                gfx.tran_center(d.texture, scale, d.src, dst, d.prio, alpha, rgb, d.angle as i32 as f32, None);
            }
        }
    }
}

// --- Caméra du décor ------------------------------------------------------

/// Une étape de la caméra : durée, zoom (centièmes ; -1 : inchangé),
/// position.
#[derive(Clone, Copy)]
struct Target {
    dur: i32,
    zoom: i32,
    x: i32,
    y: i32,
}

/// AD_BGTargetSet et AD_CameraProc : la caméra passe d'étape en étape
/// (cinq au plus), en ligne droite.
pub struct Camera {
    x: i32,
    y: i32,
    zoom: i32,
    targets: [Option<Target>; 5],
    /// Étape en cours (-1 : à l'arrêt) et son image.
    index: i32,
    counter: i32,
    /// Ce que voient les dessins : décalage et zoom.
    pub view: (f32, f32, f32),
}

impl Default for Camera {
    fn default() -> Camera {
        Camera { x: 0, y: 0, zoom: 100, targets: [None; 5], index: -1, counter: 0, view: (0.0, 0.0, 1.0) }
    }
}

impl Camera {
    fn current(&self) -> Option<Target> {
        usize::try_from(self.index).ok().and_then(|i| self.targets[i])
    }

    /// AD_BGTargetPrepare : la caméra garde où elle en est, la suite est
    /// effacée.
    pub fn prepare(&mut self) {
        if let Some(t) = self.current().filter(|t| t.dur > 0 && t.zoom >= 0) {
            let k = self.counter as f32 / t.dur as f32;
            self.x = (self.x as f32 + k * (t.x - self.x) as f32) as i32;
            self.y = (self.y as f32 + k * (t.y - self.y) as f32) as i32;
            self.zoom = (self.zoom as f32 + k * (t.zoom - self.zoom) as f32) as i32;
        }
        self.counter = 0;
        self.index = 0;
        self.targets = [None; 5];
    }

    /// L'étape en cours : son numéro, son compte, sa durée.
    pub fn running(&self) -> Option<(usize, i32, i32)> {
        let t = self.current()?;
        Some((self.index as usize, self.counter, t.dur))
    }

    /// Où la caméra s'est posée : x, y, zoom (en centièmes).
    pub fn position(&self) -> (i32, i32, i32) {
        (self.x, self.y, self.zoom)
    }

    /// La caméra posée d'un coup (une partie reprise).
    pub fn place(&mut self, (x, y, zoom): (i32, i32, i32)) {
        *self = Camera { x, y, zoom, view: (x as f32, y as f32, zoom as f32 / 100.0), ..Camera::default() };
    }

    /// AD_BGTargetSet : une étape de plus.
    pub fn set(&mut self, dur: i32, x: i32, y: i32, zoom: i32) {
        if let Some(slot) = self.targets.iter_mut().find(|t| t.is_none()) {
            *slot = Some(Target { dur, zoom, x, y });
        }
    }

    /// AD_CameraProc : une image.
    pub fn step(&mut self) {
        if self.index >= 0 {
            self.counter += 1;
            let t = self.current();
            if t.is_none_or(|t| t.dur <= self.counter) {
                if let Some(t) = t.filter(|t| t.zoom >= 0) {
                    (self.x, self.y, self.zoom) = (t.x, t.y, t.zoom);
                }
                self.counter = 0;
                self.index += 1;
                if !(self.index < 5 && self.targets[self.index as usize].is_some()) {
                    self.index = -1;
                }
            }
        }
        self.view = match self.current().filter(|t| t.zoom >= 0) {
            Some(t) => {
                let k = self.counter as f32 / t.dur.max(1) as f32;
                let at = |a: i32, b: i32| a as f32 + k * (b - a) as f32;
                (at(self.x, t.x), at(self.y, t.y), at(self.zoom, t.zoom) / 100.0)
            }
            None => (self.x as f32, self.y as f32, self.zoom as f32 / 100.0),
        };
    }

    /// AD_GpCameraTran : `dst` (centre, taille) moins la caméra, agrandi du
    /// zoom autour du centre de l'écran.
    pub fn draw(&self, gfx: &mut Gfx, texture: &'static str, src: [f32; 4], dst: [f32; 4], prio: i32, alpha: i32) {
        let (cx, cy, zoom) = self.view;
        let dst = [dst[0] - cx, dst[1] - cy, dst[2], dst[3]];
        gfx.tran_center(Some(texture), zoom, src, dst, prio, alpha, crate::gfx::GREY, 0.0, Some((SCREEN.0 / 2.0, SCREEN.1 / 2.0)));
    }
}

// --- Fondus au noir -------------------------------------------------------

/// AD_BlackOutStart / End (plein écran, première couche d'un plan) : un
/// rectangle noir qui apparaît ou s'efface en `dur` images, et reste entre
/// les deux. Le plan 6, le plus haut, sert aux passages ; le plan 5
/// assombrit sous les fenêtres.
pub struct BlackOut {
    /// Le plan (0 à 6).
    layer: usize,
    /// 0 : rien ; 1 : apparaît ; 2 : noir ; 3 : s'efface.
    state: u8,
    count: i32,
    dur: i32,
    /// L'opacité visée au dernier départ : celle dont part l'effacement
    /// (0x7f au démarrage du jeu, AD_BlackOutInit).
    alpha: i32,
}

/// Les priorités des sept plans de fondu.
const BLACKOUT_PRIO: [i32; 7] = [0x1C, 0x39, 0x3A, 0x9E, 0xDA, 0xE4, 0xFA];

impl BlackOut {
    /// Un plan sans fondu.
    pub fn new(layer: usize) -> BlackOut {
        BlackOut { layer, state: 0, count: 0, dur: 1, alpha: 0x7F }
    }

    /// Le plan 6, noir d'emblée (on arrive au mini-jeu depuis le menu,
    /// déjà noir).
    pub fn black() -> BlackOut {
        BlackOut { state: 2, ..BlackOut::new(6) }
    }

    /// Le groupe de ses effets (plan + couche × 7).
    fn group(&self) -> i32 {
        self.layer as i32
    }

    fn effect(&self, effects: &mut Effects, dur: i32, from: i32, to: i32) {
        let mut def = EffectDef::ready();
        def.kind = 2;
        def.group = self.group();
        def.prio = BLACKOUT_PRIO[self.layer];
        def.alpha = from;
        def.rgb = [0; 3];
        def.pos = (320, 224);
        def.size = SCREEN;
        def.keys[0] = Key { dur, alpha: to, rgb: [0; 3], ..Key::NONE };
        effects.make(def);
    }

    pub fn start(&mut self, effects: &mut Effects, dur: i32, alpha: i32) {
        let dur = dur.max(1);
        effects.delete(2, Some(self.group()));
        self.effect(effects, dur, 0, alpha);
        *self = BlackOut { state: 1, count: 0, dur, alpha, ..*self };
    }

    /// L'effacement part de l'opacité du dernier départ, même si le plan
    /// n'est pas noir.
    pub fn end(&mut self, effects: &mut Effects, dur: i32) {
        let dur = dur.max(1);
        effects.delete(2, Some(self.group()));
        self.effect(effects, dur, self.alpha, 0);
        self.state = 3;
        self.count = 0;
        self.dur = dur;
    }

    /// Le plan est-il posé (en train d'apparaître ou noir : 0x1da753c) ?
    pub fn on(&self) -> bool {
        matches!(self.state, 1 | 2)
    }

    /// AD_BlackOutProc : une image.
    pub fn step(&mut self, effects: &mut Effects) {
        if matches!(self.state, 1 | 3) {
            self.count += 1;
            if self.count == self.dur || effects.held(2, self.group()) {
                self.state = if self.state == 1 { 2 } else { 0 };
                effects.delete(2, Some(self.group()));
                self.count = 0;
            }
        }
    }

    /// AD_BlackOutTran : le noir tenu.
    pub fn draw(&self, gfx: &mut Gfx) {
        if self.state == 2 {
            gfx.paint([0.0, 0.0, SCREEN.0, SCREEN.1], BLACKOUT_PRIO[self.layer], self.alpha, [0; 3]);
        }
    }
}

// --- Bruitages différés ---------------------------------------------------

/// AD_SEReservePlay / AD_SEReserveStop : des notes à jouer (ou un arrêt de
/// toutes, numéro -1) après quelques images.
#[derive(Default)]
pub struct SeQueue {
    pending: Vec<(i32, i32)>,
}

impl SeQueue {
    pub fn play(&mut self, id: i32, delay: i32) {
        if self.pending.len() < 50 {
            self.pending.push((id, delay.max(1)));
        }
    }

    pub fn stop(&mut self, delay: i32) {
        self.play(-1, delay);
    }

    /// AD_SEAtOnceProc : toutes les notes en attente, tout de suite.
    pub fn at_once(&mut self) -> Vec<i32> {
        self.pending.drain(..).map(|(id, _)| id).collect()
    }

    /// AD_SEProc : les notes dont le délai finit, dans l'ordre.
    pub fn step(&mut self) -> Vec<i32> {
        let mut due = Vec::new();
        self.pending.retain_mut(|(id, delay)| {
            *delay -= 1;
            if *delay == 0 {
                due.push(*id);
                return false;
            }
            true
        });
        due
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La caméra va d'un point à l'autre en ligne droite, puis s'arrête.
    #[test]
    fn camera() {
        let mut c = Camera::default();
        c.prepare();
        c.set(10, -120, 128, 200);
        for _ in 0..5 {
            c.step();
        }
        assert_eq!(c.view, (-60.0, 64.0, 1.5));
        for _ in 0..5 {
            c.step();
        }
        assert_eq!(c.view, (-120.0, 128.0, 2.0));
        c.prepare();
        c.set(0, 0, 40, 100);
        c.step();
        assert_eq!(c.view, (0.0, 40.0, 1.0));
    }

    /// Un effet passe ses images-clés puis disparaît.
    #[test]
    fn effect() {
        let mut fx = Effects::default();
        let mut def = EffectDef::ready();
        def.keys[0] = Key { dur: 2, ..Key::NONE };
        def.keys[1] = Key { dur: 1, ..Key::NONE };
        fx.make(def);
        // 2 + 1 images comptées, plus une image par passage d'image-clé.
        for _ in 0..4 {
            fx.step();
        }
        assert!(fx.slots[0].is_some());
        fx.step();
        assert!(fx.slots[0].is_none());
    }

    /// rand() de newlib, graine 1.
    #[test]
    fn rng() {
        let mut r = Rng::new(1);
        let first = r.next();
        assert_eq!(first, ((0x5851_F42D_4C95_7F2Du64.wrapping_add(1)) >> 32) as i32 & 0x7FFF_FFFF);
    }
}
