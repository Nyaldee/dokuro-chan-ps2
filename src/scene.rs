//! La scène de l'histoire : le décor (AD_BG*), la caméra (AD_BGTarget*,
//! AD_CameraProc), les personnages (AD_TrChara*) et les sept plans de
//! fondu (AD_BlackOut*). Voir docs/moteur.md.

use std::sync::Arc;

use crate::data::{self, Uni};
use crate::engine::{Camera, EffectDef, Effects, Key, SCREEN};
use crate::gfx::{Gfx, GREY};
use crate::original::Iso;
use crate::rumble::Rumble;
use crate::savedata;
use crate::shake::Shake;
use crate::story::{Batch, World};

/// Les priorités des sept plans de fondu.
const PLANE_PRIO: [i32; 7] = [0x1C, 0x39, 0x3A, 0x9E, 0xDA, 0xE4, 0xFA];
/// La priorité du décor, et celle des personnages (plus leur ordre).
const BG_PRIO: i32 = 0x0A;
const CHARA_PRIO: i32 = 0x1E;

/// Comment un lot succède au précédent (AD_AdvBthAfterProc, +0x3a2 du lot).
/// Le jeu réécrit alors les transitions que le script a posées :
///
/// - `Plain` (0) : rien n'est réécrit ;
/// - `Flash` (1) : un changement de scène (ezeffect_scenechange) ; tout est
///   décalé de la durée de son éclair, le décor change d'un coup au milieu,
///   les personnages paraissent et s'en vont en sa durée ;
/// - `Into` (2) : d'un lot de texte (sortes 0 à 3) vers une autre sorte ; le
///   décor change d'un coup à la fin du passage (sa durée), les sons et
///   l'attente du décor sont décalés d'autant, les personnages paraissent et
///   s'en vont en sa durée, les éclairs sautent ;
/// - `Switch` (3) : d'une sorte autre que le texte vers une autre encore ;
///   le décor change d'un coup aussitôt, la musique part aussitôt, les
///   éclairs sautent.
///
/// Le portage ne réécrivait rien (bug du portage) : un lot de vidéo qui
/// change de décor en fondu enchaîné de 90 images (après la page qui
/// précède, 101:sure1) laissait ce fondu commencer avant la vidéo et le
/// reprendre après, sous les yeux : l'ancien décor revenait. Le jeu, lui, le
/// change d'un coup sous le noir du passage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pass {
    Plain,
    Flash(i32),
    Into(i32),
    Switch,
}

impl Pass {
    /// L'attente du décor (+0x3aa) après réécriture, que les apparitions
    /// des personnages attendent.
    pub fn bg_wait(self, wait: i32) -> i32 {
        match self {
            Pass::Plain => wait,
            Pass::Into(d) => d,
            Pass::Flash(_) | Pass::Switch => 0,
        }
    }

    /// Le retard ajouté aux bruitages et aux sons en boucle (le changement
    /// de sorte ne les avance pas : `Switch` les laisse).
    pub fn sound_delay(self) -> i32 {
        match self {
            Pass::Flash(d) | Pass::Into(d) => d,
            Pass::Plain | Pass::Switch => 0,
        }
    }

    /// Le retard de la musique : celui du script, décalé ou remplacé.
    pub fn music_delay(self, delay: i32) -> i32 {
        match self {
            Pass::Plain => delay,
            Pass::Flash(d) => delay + d,
            Pass::Into(d) => d,
            Pass::Switch => 0,
        }
    }
}

/// Les images de l'histoire, chargées à la demande : décors (BACK.UNI),
/// personnages (CHARA.UNI), planches (ETC.UNI) ; celles qui ne servent
/// plus sont libérées.
pub struct Images {
    iso: Arc<Iso>,
    back: Option<Uni>,
    chara: Option<Uni>,
    etc: Option<Uni>,
    loaded: Vec<String>,
}

impl Images {
    pub fn new(iso: Arc<Iso>) -> Images {
        let open = |p: &str| Uni::open(&iso, p);
        Images { back: open("UNION/BACK.UNI"), chara: open("UNION/CHARA.UNI"), etc: open("UNION/ETC.UNI"), iso, loaded: Vec::new() }
    }

    /// Charge une image d'un paquet sous un nom (si elle ne l'est pas).
    fn ensure(&mut self, gfx: &mut Gfx, key: &str, which: Pack, index: i32) -> bool {
        if gfx.has(key) {
            return true;
        }
        let uni = match which {
            Pack::Back => self.back.as_ref(),
            Pack::Chara => self.chara.as_ref(),
            Pack::Etc => self.etc.as_ref(),
        };
        let Some(img) = uni.and_then(|u| u.read(&self.iso, index as u32)).and_then(|d| data::art2(&d)) else { return false };
        gfx.load(key, img);
        self.loaded.push(key.to_string());
        true
    }

    /// Libère les images chargées qui ne servent plus.
    pub fn trim(&mut self, gfx: &mut Gfx, used: &[&str]) {
        self.loaded.retain(|k| {
            let keep = used.contains(&k.as_str());
            if !keep {
                gfx.unload(k);
            }
            keep
        });
    }

    /// Un décor par son nom (bg_entry).
    pub fn bg(&mut self, gfx: &mut Gfx, world: &World, name: &[u8]) -> Option<String> {
        let &(_, index) = world.bgs.iter().find(|(n, _)| n == name)?;
        let key = format!("bg:{}", String::from_utf8_lossy(name));
        self.ensure(gfx, &key, Pack::Back, index).then_some(key)
    }

    /// Une image de personnage ou une planche par son nom (chara_gp_entry,
    /// thum_gp_entry).
    pub fn image(&mut self, gfx: &mut Gfx, world: &World, name: &[u8]) -> Option<String> {
        let &(_, index) = world.images.iter().rev().find(|(n, _)| n == name)?;
        let key = format!("img:{}", String::from_utf8_lossy(name));
        let pack = if index >= 1000 { Pack::Chara } else { Pack::Etc };
        self.ensure(gfx, &key, pack, index).then_some(key)
    }
}

/// Les paquets d'images de l'histoire.
#[derive(Clone, Copy)]
enum Pack {
    Back,
    Chara,
    Etc,
}

/// Une image de décor posée : sa texture.
#[derive(Clone)]
struct Back {
    name: Vec<u8>,
    key: String,
}

/// Le décor : l'image en place et celle qui arrive (fondu enchaîné), en
/// `count` images sur `dur`, après `delay` images ; celle qu'une image
/// bonus a remplacée.
#[derive(Default)]
struct Bg {
    saved: Option<Back>,
    current: Option<Back>,
    next: Option<Back>,
    count: i32,
    dur: i32,
    delay: i32,
}

impl Bg {
    /// AD_BGPrintChange (sorte 0, la seule des scripts). Faux si c'est déjà
    /// le décor.
    fn change(&mut self, back: Back, dur: i32, delay: i32) -> bool {
        if self.next.as_ref().or(self.current.as_ref()).is_some_and(|b| b.name == back.name) {
            return false;
        }
        if let Some(n) = self.next.take() {
            self.current = Some(n);
        }
        self.next = Some(back);
        self.count = 0;
        self.dur = dur.max(1);
        self.delay = delay;
        true
    }

    /// AD_BGProc.
    fn step(&mut self) {
        if self.next.is_none() {
            return;
        }
        if self.delay > 0 {
            self.delay -= 1;
            return;
        }
        self.count += 1;
        if self.count >= self.dur {
            self.current = self.next.take();
        }
    }

    /// L'avancement du fondu (0 à 1).
    fn progress(&self) -> f32 {
        if self.next.is_none() { 1.0 } else { self.count as f32 / self.dur as f32 }
    }
}

/// Un morceau de personnage dessiné : l'image, le corps (rectangle), le
/// visage (planche et rectangle), la place du visage, le décalage.
#[derive(Clone, Default)]
struct Look {
    image: String,
    body: [f32; 4],
    face: Option<(String, [f32; 4])>,
    face_at: (f32, f32),
    offset: (f32, f32),
}

/// Un personnage à l'écran (AD_TrChara*).
struct Actor {
    name: Vec<u8>,
    /// L'image et l'expression demandées (pour une partie sauvegardée).
    pose: (Vec<u8>, Vec<u8>),
    look: Option<Look>,
    /// L'ancienne apparence pendant un changement en fondu.
    old: Option<Look>,
    blend: (i32, i32),
    index: i32,
    /// La position : départ, arrivée, image et durée du mouvement.
    x: (f32, f32, i32, i32),
    /// L'opacité : départ, arrivée, image et durée ; retard.
    alpha: (f32, f32, i32, i32),
    delay: i32,
    /// Effacé à la fin de la disparition.
    remove: bool,
    rgb: [i32; 3],
}

impl Actor {
    fn x(&self) -> f32 {
        let (from, to, t, d) = self.x;
        if t >= d {
            return to;
        }
        // AD_TrCharaDestSet : en sinus, de −90° à 90°.
        let a = (t as f32 / d as f32 * 180.0 - 90.0).to_radians();
        from + (to - from) / 2.0 * (a.sin() + 1.0)
    }

    fn opacity(&self) -> f32 {
        let (from, to, t, d) = self.alpha;
        if t >= d {
            return to;
        }
        let a = (t as f32 / d as f32 * 180.0 - 90.0).to_radians();
        from + (to - from) * (a.sin() + 1.0) / 2.0
    }

    fn step(&mut self) {
        if self.delay > 0 {
            self.delay -= 1;
            return;
        }
        if self.x.2 < self.x.3 {
            self.x.2 += 1;
        }
        if self.alpha.2 < self.alpha.3 {
            self.alpha.2 += 1;
        }
        if self.blend.0 < self.blend.1 {
            self.blend.0 += 1;
            if self.blend.0 >= self.blend.1 {
                self.old = None;
            }
        }
    }

    fn done(&self) -> bool {
        self.delay <= 0 && self.x.2 >= self.x.3 && self.alpha.2 >= self.alpha.3 && self.blend.0 >= self.blend.1
    }
}

/// Un plan de fondu (AD_BlackOut*) : 0 rien, 1 apparaît, 2 tenu, 3
/// s'efface ; sa couleur, son opacité, sa forme et ses passages à venir.
#[derive(Clone, Copy)]
struct Plane {
    state: u8,
    count: i32,
    dur: i32,
    alpha: i32,
    rgb: [i32; 3],
    shape: (u8, u8),
    repeats: i32,
    /// Les effets du fondu en cours (1, ou une bande chacun).
    parts: usize,
}

impl Default for Plane {
    fn default() -> Plane {
        Plane { state: 0, count: 0, dur: 1, alpha: 0x7F, rgb: [0; 3], shape: (0, 0), repeats: 0, parts: 1 }
    }
}

/// Le groupe des effets d'un plan.
fn plane_group(p: usize) -> i32 {
    0x100 + p as i32
}

/// Un rectangle de la scène (coin, taille) à l'écran, à travers une
/// caméra (décalage et zoom, autour du centre de l'écran).
#[allow(clippy::too_many_arguments)]
fn place(gfx: &mut Gfx, (cx, cy, zoom): (f32, f32, f32), key: &str, src: [f32; 4], dst: [f32; 4], prio: i32, alpha: i32, rgb: [i32; 3]) {
    let centre = [dst[0] + dst[2] / 2.0 - cx, dst[1] + dst[3] / 2.0 - cy, dst[2], dst[3]];
    gfx.tran_center(Some(key), zoom, src, centre, prio, alpha, rgb, 0.0, Some((SCREEN.0 / 2.0, SCREEN.1 / 2.0)));
}

#[derive(Default)]
pub struct Scene {
    bg: Bg,
    /// La caméra du décor en place (le jeu en a une par décor, 0x1be926c) ;
    /// pendant un fondu enchaîné, ce que voyait celle du décor qui s'en va.
    pub camera: Camera,
    old_view: Option<(f32, f32, f32)>,
    actors: Vec<Actor>,
    planes: [Plane; 7],
    shake: Shake,
}

impl Scene {
    /// L'apparence d'un personnage : son image, son corps (chara_pt_set) et
    /// son expression (chara_emo_set).
    fn look(world: &World, images: &mut Images, gfx: &mut Gfx, chara: &[u8], image: &[u8], emotion: &[u8]) -> Option<Look> {
        let (_, _, _, rect, place) = world.patterns.iter().rev().find(|p| p.0 == image && p.1 == chara)?;
        let key = images.image(gfx, world, &world.patterns.iter().rev().find(|p| p.0 == image)?.2)?;
        let face = world
            .emotions
            .iter()
            .rev()
            .find(|e| e.0 == emotion && e.1 == chara && e.2 == image)
            .filter(|e| e.4[2] > 0 && e.4[3] > 0)
            .and_then(|e| Some((images.image(gfx, world, &e.3)?, e.4.map(|v| v as f32))));
        Some(Look {
            image: key,
            body: rect.map(|v| v as f32),
            face,
            face_at: (place[0] as f32, place[1] as f32),
            offset: (place[2] as f32, place[3] as f32),
        })
    }

    /// AD_AdvBthAfterProc (la scène) : le lot devient l'état de la scène,
    /// selon la façon dont il succède au précédent (`pass`). Vrai si le décor
    /// change (l'album le note vu).
    pub fn apply(&mut self, b: &Batch, pass: Pass, world: &World, images: &mut Images, gfx: &mut Gfx, effects: &mut Effects) -> bool {
        let mut changed = false;
        // Le décor : la transition du script, sauf au changement de scène
        // (d'un coup au milieu de l'éclair) et au changement de sorte de lot
        // (d'un coup, à la fin du passage, ou aussitôt).
        if let Some(bg) = &b.bg
            && let Some(key) = images.bg(gfx, world, &bg.name)
        {
            let (dur, delay) = match pass {
                Pass::Plain => (i32::from(bg.dur), i32::from(bg.delay)),
                Pass::Flash(d) | Pass::Into(d) => (1, d),
                Pass::Switch => (1, 0),
            };
            let delay = delay + b.wait_del + 2;
            if self.bg.change(Back { name: bg.name.clone(), key }, dur, delay) {
                self.new_camera();
                changed = true;
            }
        }
        // La caméra : chaque étape posée (la première efface la suite),
        // avec son tremblement.
        let mut slot = 0;
        for (k, t) in b.targets[0].iter().enumerate() {
            if t.dur < 0 {
                continue;
            }
            if k == 0 {
                self.camera.prepare();
                self.shake.prepare();
            }
            self.camera.set(i32::from(t.dur), i32::from(t.x), i32::from(t.y), i32::from(t.zoom));
            self.shake.set_step(slot, t.shake);
            slot += 1;
        }
        // Les fondus ; les éclairs n'ont pas lieu au changement de sorte de
        // lot.
        for (p, f) in b.fades.iter().enumerate() {
            let Some(f) = f else { continue };
            if f.kind == 2 && matches!(pass, Pass::Into(_) | Pass::Switch) {
                continue;
            }
            let rgb = world.colors.get(f.color.max(0) as usize).copied().flatten().map_or([0; 3], |c| c.map(i32::from));
            let dur = i32::from(f.dur).max(1);
            match f.kind {
                0 => self.fade(p, effects, dur, i32::from(f.alpha), rgb, (f.a, f.b), 0, true),
                1 => {
                    let alpha = self.planes[p].alpha;
                    self.fade(p, effects, dur, alpha, self.planes[p].rgb, (f.a, f.b), 0, false);
                }
                _ => self.fade(p, effects, dur, i32::from(f.alpha), rgb, (f.a, f.b), 1, true),
            }
        }
        // Les personnages.
        for c in &b.charas {
            if c.tally == 0 {
                continue;
            }
            let (ones, tens, hundreds) = (c.tally % 10, c.tally / 10 % 10, c.tally / 100 % 10);
            if ones == 2 {
                self.actors.retain(|a| a.name != c.name);
                continue;
            }
            let k = match self.actors.iter().position(|a| a.name == c.name) {
                Some(k) => k,
                None => {
                    self.actors.push(Actor {
                        name: c.name.clone(),
                        pose: Default::default(),
                        look: None,
                        old: None,
                        blend: (0, 0),
                        index: 0,
                        x: (320.0, 320.0, 0, 0),
                        alpha: (0.0, 0.0, 0, 0),
                        delay: 0,
                        remove: false,
                        rgb: [0x80; 3],
                    });
                    self.actors.len() - 1
                }
            };
            let a = &mut self.actors[k];
            // La durée (+0x22c) et le retard (+0x246) du script, que le
            // changement de scène et le passage depuis une page réécrivent
            // (AD_AdvBthAfterProc, d'après le chiffre des centaines puis des
            // dizaines) : une apparition ou une disparition dure l'éclair ou
            // le passage, sans retard ; un déplacement se fait d'un coup à la
            // fin ; un changement d'image seul attend la fin.
            let (mut dur, mut delay) = (i32::from(c.durs[0]), i32::from(c.delay).max(0));
            if let Pass::Flash(x) | Pass::Into(x) = pass {
                match hundreds {
                    1..=3 => (dur, delay) = (x, 0),
                    4 => (dur, delay) = (1, x),
                    _ if tens > 0 => delay = x,
                    _ => {}
                }
            }
            let dur = dur.max(1);
            // Une apparition attend aussi les disparitions et le décor, dont
            // l'attente devient celle du passage (+0x3aa réécrit).
            a.delay = delay + if hundreds == 1 { b.wait_del + pass.bg_wait(b.wait_bg) } else { 0 };
            if tens >= 1 {
                let look = Scene::look(world, images, gfx, &c.name, &c.images[0], &c.emotions[0]);
                if tens == 2 && a.look.is_some() && a.opacity() > 0.0 {
                    a.old = a.look.take();
                    a.blend = (0, dur);
                }
                a.look = look;
                a.pose = (c.images[0].clone(), c.emotions[0].clone());
                a.index = i32::from(c.index);
                a.rgb = c.rgb[0].map(|v| if v == 0xFF { 0x80 } else { i32::from(v) });
            }
            match hundreds {
                1 => {
                    a.x = (f32::from(c.values[0]), f32::from(c.values[1]), 0, dur);
                    a.alpha = (0.0, 1.0, 0, dur);
                    a.remove = false;
                }
                2 | 3 => {
                    a.alpha = (a.opacity(), 0.0, 0, dur);
                    a.remove = true;
                }
                4 => a.x = (a.x(), f32::from(c.values[0]), 0, dur),
                _ => {}
            }
        }
        // Les images qui ne sont plus à l'écran sont libérées.
        let mut used: Vec<&str> = [&self.bg.saved, &self.bg.current, &self.bg.next].into_iter().flatten().map(|b| b.key.as_str()).collect();
        for a in &self.actors {
            for l in [&a.look, &a.old].into_iter().flatten() {
                used.push(&l.image);
                if let Some((sheet, _)) = &l.face {
                    used.push(sheet);
                }
            }
        }
        images.trim(gfx, &used);
        changed
    }

    /// Un fondu sur un plan : apparaître (`on`) ou s'effacer, plein écran ou
    /// en bandes, `repeats` passages de plus (un éclair).
    #[allow(clippy::too_many_arguments)]
    fn fade(&mut self, p: usize, effects: &mut Effects, dur: i32, alpha: i32, rgb: [i32; 3], shape: (u8, u8), repeats: i32, on: bool) {
        effects.delete(2, Some(plane_group(p)));
        let (from, to) = if on { (0, alpha) } else { (alpha, 0) };
        let parts = make_fade(effects, p, dur, from, to, rgb, shape);
        self.planes[p] = Plane { state: if on { 1 } else { 3 }, count: 0, dur, alpha, rgb, shape, repeats, parts };
    }

    /// Un changement de décor est-il encore en attente ou en cours ?
    pub fn bg_changing(&self) -> bool {
        self.bg.next.is_some()
    }

    /// Une image : décor, caméra (et son tremblement, qui donne sa force à
    /// la vibration), personnages, fondus.
    pub fn step(&mut self, effects: &mut Effects, rumble: &mut Rumble) {
        self.bg.step();
        if self.bg.next.is_none() {
            self.old_view = None;
        }
        let before = self.camera.running().map(|r| r.0);
        self.camera.step();
        if let Some(k) = before
            && self.camera.running().map(|r| r.0) != Some(k)
        {
            self.shake.settle(k);
        }
        rumble.add(self.shake.step(self.camera.running()));
        for a in &mut self.actors {
            a.step();
        }
        self.actors.retain(|a| !(a.remove && a.done()));
        for p in 0..7 {
            let plane = &mut self.planes[p];
            if matches!(plane.state, 1 | 3) {
                plane.count += 1;
                if plane.count >= plane.dur || effects.held_count(2, plane_group(p)) >= plane.parts {
                    plane.state = if plane.state == 1 { 2 } else { 0 };
                    effects.delete(2, Some(plane_group(p)));
                    plane.count = 0;
                }
            } else if plane.repeats > 0 {
                // Un éclair repart dans l'autre sens.
                plane.repeats -= 1;
                let on = plane.state == 0;
                let (from, to) = if on { (0, plane.alpha) } else { (plane.alpha, 0) };
                plane.state = if on { 1 } else { 3 };
                let (dur, rgb, shape) = (plane.dur, plane.rgb, plane.shape);
                plane.parts = make_fade(effects, p, dur, from, to, rgb, shape);
            }
        }
    }

    /// AD_AdvSaveProc (la scène) : le décor, la caméra, les personnages à
    /// leur place, les plans 0 à 3 tenus.
    pub fn snapshot(&self) -> savedata::Scene {
        let actors = self
            .actors
            .iter()
            .filter(|a| !a.remove)
            .map(|a| savedata::Actor { name: a.name.clone(), image: a.pose.0.clone(), emotion: a.pose.1.clone(), index: a.index, x: a.x.1 as i32, rgb: a.rgb })
            .collect();
        let mut planes = [None; 4];
        for (slot, p) in planes.iter_mut().zip(&self.planes) {
            if p.state == 2 {
                *slot = Some(savedata::Plane { alpha: p.alpha, rgb: p.rgb, shape: p.shape });
            }
        }
        let bg = self.bg.next.as_ref().or(self.bg.current.as_ref()).map(|b| b.name.clone());
        savedata::Scene { bg, camera: self.camera.position(), actors, planes }
    }

    /// AD_AdvLoadAfterProc (la scène) : celle d'une partie reprise, d'un
    /// coup (les plans en une image).
    pub fn restore(&mut self, s: &savedata::Scene, world: &World, images: &mut Images, gfx: &mut Gfx, effects: &mut Effects) {
        self.clear(effects);
        if let Some(name) = &s.bg
            && let Some(key) = images.bg(gfx, world, name)
        {
            self.bg.current = Some(Back { name: name.to_vec(), key });
        }
        self.camera.place(s.camera);
        for a in &s.actors {
            let x = a.x as f32;
            self.actors.push(Actor {
                name: a.name.clone(),
                pose: (a.image.clone(), a.emotion.clone()),
                look: Scene::look(world, images, gfx, &a.name, &a.image, &a.emotion),
                old: None,
                blend: (0, 0),
                index: a.index,
                x: (x, x, 0, 0),
                alpha: (1.0, 1.0, 0, 0),
                delay: 0,
                remove: false,
                rgb: a.rgb,
            });
        }
        for (k, p) in s.planes.iter().enumerate() {
            if let Some(p) = p {
                self.fade(k, effects, 1, p.alpha, p.rgb, p.shape, 0, true);
            }
        }
    }

    /// AD_BGPrintChange : le nouveau décor a sa propre caméra, en (0, 0) à
    /// 100 % et sans tremblement ; celle du décor qui s'en va reste où elle
    /// en était jusqu'à la fin du fondu enchaîné. (Les scripts ne règlent
    /// jamais cette caméra autrement : camera_set n'y sert pas.)
    fn new_camera(&mut self) {
        self.old_view = Some(self.view());
        self.camera.place((0, 0, 100));
        self.shake.reset();
    }

    /// AD_TrCharaDeleteAll : les personnages s'en vont (un lieu choisi sur
    /// la carte).
    pub fn clear_charas(&mut self) {
        self.actors.clear();
    }

    /// Une image à la place du décor, d'un coup (l'image bonus d'un
    /// mini-jeu gagné) ; le décor est gardé.
    pub fn show_bg(&mut self, name: &[u8], key: String) {
        self.bg.saved = self.bg.next.take().or(self.bg.current.take());
        self.bg.current = Some(Back { name: name.to_vec(), key });
        self.new_camera();
        self.old_view = None;
    }

    /// AD_BGPrintChangeBk : le décor gardé revient (caméra en (0, 0) à
    /// 100 %).
    pub fn restore_bg(&mut self) {
        if let Some(b) = self.bg.saved.take() {
            self.bg.current = Some(b);
            self.new_camera();
            self.old_view = None;
        }
    }

    /// Un plan s'assombrit en `dur` images jusqu'à `alpha` (AD_BlackOutStart
    /// en noir, plein écran), ou s'éclaircit.
    pub fn darken(&mut self, p: usize, effects: &mut Effects, dur: i32, alpha: i32) {
        self.fade(p, effects, dur, alpha, [0; 3], (0, 0), 0, true);
    }

    pub fn lighten(&mut self, p: usize, effects: &mut Effects, dur: i32) {
        let alpha = self.planes[p].alpha;
        self.fade(p, effects, dur, alpha, [0; 3], (0, 0), 0, false);
    }

    /// Tout s'efface (fin de l'histoire, retour au menu).
    pub fn clear(&mut self, effects: &mut Effects) {
        *self = Scene::default();
        for p in 0..7 {
            effects.delete(2, Some(plane_group(p)));
        }
    }

    /// Ce que voit la caméra : décalage et zoom, tremblement compris.
    fn view(&self) -> (f32, f32, f32) {
        let (x, y, z) = self.camera.view;
        let (sx, sy, sz) = self.shake.offset;
        (x + sx as f32, y + sy as f32, z + sz)
    }

    /// Un rectangle de la scène (coin, taille) à l'écran, à travers la
    /// caméra (zoom autour du centre de l'écran).
    #[allow(clippy::too_many_arguments)]
    fn place(&self, gfx: &mut Gfx, key: &str, src: [f32; 4], dst: [f32; 4], prio: i32, alpha: i32, rgb: [i32; 3]) {
        place(gfx, self.view(), key, src, dst, prio, alpha, rgb);
    }

    pub fn draw(&self, gfx: &mut Gfx) {
        // Le décor : 512 × 448 de l'image, sur tout l'écran ; celui qui s'en
        // va, à travers sa propre caméra.
        let full = [0.0, 0.0, SCREEN.0, SCREEN.1];
        if let Some(b) = &self.bg.current {
            place(gfx, self.old_view.unwrap_or_else(|| self.view()), &b.key, [0.0, 0.0, 512.0, 448.0], full, BG_PRIO, 0x7F, GREY);
        }
        if let Some(b) = &self.bg.next {
            let alpha = (self.bg.progress() * 127.0) as i32;
            self.place(gfx, &b.key, [0.0, 0.0, 512.0, 448.0], full, BG_PRIO + 1, alpha, GREY);
        }
        // Les personnages : le corps en bas d'une toile de 512 × 512 centrée
        // en (x, 448 − 256 − décalage), le visage par-dessus.
        for a in &self.actors {
            let opacity = a.opacity();
            let x = a.x();
            let prio = CHARA_PRIO + a.index;
            let mut layers = Vec::new();
            if let Some(old) = &a.old {
                let k = a.blend.0 as f32 / a.blend.1.max(1) as f32;
                layers.push((old, 1.0 - k));
                if let Some(l) = &a.look {
                    layers.push((l, k));
                }
            } else if let Some(l) = &a.look {
                layers.push((l, 1.0));
            }
            for (look, k) in layers {
                let alpha = (opacity * k * 128.0) as i32;
                if alpha <= 0 {
                    continue;
                }
                let [_, _, w, h] = look.body;
                let cx = x - look.offset.0;
                let cy = SCREEN.1 - 256.0 - look.offset.1;
                let (left, top) = (cx - w / 2.0, cy + 256.0 - h);
                self.place(gfx, &look.image, look.body, [left, top, w, h], prio, alpha, a.rgb);
                if let Some((sheet, src)) = &look.face {
                    let dst = [left + look.face_at.0, top + look.face_at.1, src[2], src[3]];
                    self.place(gfx, sheet, *src, dst, prio, alpha, a.rgb);
                }
            }
        }
        // Les plans de fondu tenus.
        for (p, plane) in self.planes.iter().enumerate() {
            if plane.state == 2 {
                gfx.paint([0.0, 0.0, SCREEN.0, SCREEN.1], PLANE_PRIO[p], plane.alpha, plane.rgb);
            }
        }
    }
}

/// AD_BlackOutMakeEffect : un fondu plein écran, ou en bandes (1 : 40
/// verticales de 16 pixels ; 2 : 56 horizontales de 8), chacune retardée
/// selon l'ordre `b` (AD_BlackOutMakeEffectProc). Rend le nombre d'effets.
fn make_fade(effects: &mut Effects, p: usize, dur: i32, from: i32, to: i32, rgb: [i32; 3], (a, b): (u8, u8)) -> usize {
    let base = |pos: (i32, i32), size: (f32, f32)| {
        let mut d = EffectDef::ready();
        d.kind = 2;
        d.group = plane_group(p);
        d.prio = PLANE_PRIO[p];
        d.alpha = from;
        d.rgb = rgb;
        d.pos = pos;
        d.size = size;
        d
    };
    let (count, size) = match a {
        1 => (0x28, 0x10),
        2 => (0x38, 8),
        _ => {
            let mut d = base((0x140, 0xE0), SCREEN);
            d.keys[0] = Key { dur, alpha: to, rgb, ..Key::NONE };
            effects.make(d);
            return 1;
        }
    };
    let half = dur >> 1;
    for k in 0..count {
        let at = strip(b, k, count) * size + (size >> 1);
        let mut d = if a == 1 { base((at, 0xE0), (size as f32, SCREEN.1)) } else { base((0x140, at), (SCREEN.0, size as f32)) };
        d.keys[0] = Key { dur: half * k / count, alpha: from, rgb, ..Key::NONE };
        d.keys[1] = Key { dur: half, alpha: to, rgb, ..Key::NONE };
        effects.make(d);
    }
    count as usize
}

/// AD_BlackOutMakeEffectProc : la place de la k-ième bande (sur n) selon
/// l'ordre.
fn strip(order: u8, k: i32, n: i32) -> i32 {
    match order {
        0 => k,
        1 => n - k - 1,
        // Une bande du début, une de la fin : la k-ième impaire va en
        // n − (k >> 1) − 1. (Le portage la mettait une place trop tôt : la
        // dernière bande restait vide, une autre servait deux fois.)
        2 => {
            if k % 2 == 0 { k / 2 } else { n - k / 2 - 1 }
        }
        3 => {
            if k % 2 == 0 { (n - k - 1) / 2 } else { n - (n - k - 1) / 2 - 1 }
        }
        _ => {
            if k % 2 == 0 { k } else { n - k }
        }
    }
}

