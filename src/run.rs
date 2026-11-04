//! アドバンスド徒競走 (mini_run.cpp) : Sakura (en bas, au joueur) court
//! contre Dokuro (en haut) sur 8000. ○ en rafale fait courir, ↑ saute les
//! haies, ↓ glisse en avant. Les balles et les obstacles qui roulent
//! sonnent Sakura un moment ; une haie l'arrête net. Dokuro saute seule et
//! ralentit quand on la touche. En challenge, des têtes de mort à ramasser.
//!
//! États (valeurs du jeu) : 0 voix, 5 PUSH BUTTON, 10 « よーい スタート »,
//! 20 course, 30 Dokuro arrivée la première, 35 perdu, 40 Sakura arrivée la
//! première, 45 gagné.

use crate::engine::{EffectDef, Key};
use crate::sys::Sys;
use crate::gfx::{Blend, Gfx, GREY};
use crate::mini::{self, frame, Anim, Frame, Minigame};

/// Les images (RUN.UNI, dans l'ordre de Mini_RunInit).
pub const TEXTURES: [(&str, u32); 5] = [(CHARA, 0), (BG, 1), (PARTS, 2), (COMMON, 3), (CH, 4)];

const CHARA: &str = "run_chara";
const BG: &str = "run_bg";
const PARTS: &str = "run_pa-tu";
const COMMON: &str = "run_kyoutu";
const CH: &str = "run_ch";

/// Les animations (0x35d140 pour Dokuro, 0x35d080 pour Sakura), même
/// découpage pour les deux : course (0), saut (5), glissade (7), puis les
/// poses de fin : la perdante qui regarde l'autre finir (9), la perdante
/// (11), la gagnante (13).
const DOKURO: [Frame; 15] = [
    frame(128.0, 256.0, 128.0, 128.0, 10, 0),
    frame(256.0, 256.0, 128.0, 128.0, 10, 1),
    frame(128.0, 256.0, 128.0, 128.0, 10, 2),
    frame(0.0, 256.0, 128.0, 128.0, 10, 3),
    frame(-1.0, -1.0, -1.0, -1.0, -1, 4),
    frame(384.0, 256.0, 128.0, 128.0, 0x7FFF, 5),
    frame(-1.0, -1.0, -1.0, -1.0, -1, 6),
    frame(0.0, 384.0, 128.0, 128.0, 30, 7),
    frame(-1.0, -1.0, -1.0, -1.0, -1, 8),
    frame(128.0, 384.0, 128.0, 128.0, 0x7FFF, 9),
    frame(-1.0, -1.0, -1.0, -1.0, -1, 10),
    frame(256.0, 384.0, 128.0, 128.0, 0x7FFF, 11),
    frame(-1.0, -1.0, -1.0, -1.0, -1, 12),
    frame(384.0, 384.0, 128.0, 128.0, 0x7FFF, 13),
    frame(-1.0, -1.0, -1.0, -1.0, -1, 14),
];
const SAKURA: [Frame; 15] = [
    frame(128.0, 0.0, 128.0, 128.0, 10, 0),
    frame(256.0, 0.0, 128.0, 128.0, 10, 1),
    frame(128.0, 0.0, 128.0, 128.0, 10, 2),
    frame(0.0, 0.0, 128.0, 128.0, 10, 3),
    frame(-1.0, -1.0, -1.0, -1.0, -1, 4),
    frame(384.0, 0.0, 128.0, 128.0, 0x7FFF, 5),
    frame(-1.0, -1.0, -1.0, -1.0, -1, 6),
    frame(0.0, 175.0, 128.0, 81.0, 35, 7),
    frame(-1.0, -1.0, -1.0, -1.0, -1, 8),
    frame(128.0, 128.0, 128.0, 128.0, 0x7FFF, 9),
    frame(-1.0, -1.0, -1.0, -1.0, -1, 10),
    frame(256.0, 128.0, 128.0, 128.0, 0x7FFF, 11),
    frame(-1.0, -1.0, -1.0, -1.0, -1, 12),
    frame(384.0, 128.0, 128.0, 128.0, 0x7FFF, 13),
    frame(-1.0, -1.0, -1.0, -1.0, -1, 14),
];
const RUNNING: usize = 0;
const JUMP: usize = 5;
const SLIDE: usize = 7;
const WATCHING: usize = 9;
const LOSER: usize = 11;
const WINNER: usize = 13;

/// Une étape du parcours : distance, image, sorte (0 haie, 1 obstacle qui
/// roule, 2 balle qui rebondit, 3 objet qui monte et descend, 4 et 5 têtes
/// de mort du challenge (en bas, en haut), 6 départ, 7 ruban d'arrivée, 8
/// et 9 le ruban cassé).
struct Step {
    at: i32,
    src: [f32; 4],
    kind: i32,
}

const fn step(at: i32, u: f32, v: f32, w: f32, h: f32, kind: i32) -> Step {
    Step { at, src: [u, v, w, h], kind }
}

/// Le parcours (0x35d200), le même pour les deux couloirs.
const COURSE: [Step; 56] = [
    step(200, 81.0, 229.0, 103.0, 62.0, 6),
    step(600, 3.0, 186.0, 65.0, 65.0, 0),
    step(700, 5.0, 51.0, 72.0, 74.0, 4),
    step(800, 5.0, 51.0, 72.0, 74.0, 5),
    step(1050, 198.0, 184.0, 65.0, 30.0, 1),
    step(1100, 5.0, 51.0, 72.0, 74.0, 5),
    step(1200, 68.0, 186.0, 65.0, 65.0, 2),
    step(1600, 198.0, 184.0, 65.0, 30.0, 1),
    step(1700, 5.0, 51.0, 72.0, 74.0, 5),
    step(1740, 5.0, 51.0, 72.0, 74.0, 5),
    step(1780, 5.0, 51.0, 72.0, 74.0, 5),
    step(1820, 5.0, 51.0, 72.0, 74.0, 5),
    step(1860, 5.0, 51.0, 72.0, 74.0, 5),
    step(2000, 68.0, 186.0, 65.0, 65.0, 2),
    step(2450, 5.0, 51.0, 72.0, 74.0, 5),
    step(2500, 133.0, 186.0, 65.0, 65.0, 0),
    step(2570, 5.0, 51.0, 72.0, 74.0, 4),
    step(3200, 263.0, 186.0, 65.0, 65.0, 3),
    step(3300, 5.0, 51.0, 72.0, 74.0, 5),
    step(3700, 3.0, 186.0, 65.0, 65.0, 0),
    step(3770, 5.0, 51.0, 72.0, 74.0, 4),
    step(3810, 5.0, 51.0, 72.0, 74.0, 4),
    step(3850, 5.0, 51.0, 72.0, 74.0, 4),
    step(3890, 5.0, 51.0, 72.0, 74.0, 4),
    step(3930, 5.0, 51.0, 72.0, 74.0, 4),
    step(4200, 3.0, 186.0, 65.0, 65.0, 0),
    step(4200, 198.0, 184.0, 65.0, 30.0, 1),
    step(4270, 5.0, 51.0, 72.0, 74.0, 4),
    step(4550, 198.0, 184.0, 65.0, 30.0, 1),
    step(4565, 3.0, 186.0, 65.0, 65.0, 0),
    step(4635, 5.0, 51.0, 72.0, 74.0, 4),
    step(4675, 5.0, 51.0, 72.0, 74.0, 5),
    step(4715, 5.0, 51.0, 72.0, 74.0, 4),
    step(4755, 5.0, 51.0, 72.0, 74.0, 5),
    step(4795, 5.0, 51.0, 72.0, 74.0, 4),
    step(4835, 5.0, 51.0, 72.0, 74.0, 5),
    step(5100, 68.0, 186.0, 65.0, 65.0, 2),
    step(5400, 263.0, 186.0, 65.0, 65.0, 3),
    step(5800, 3.0, 186.0, 65.0, 65.0, 0),
    step(5915, 5.0, 51.0, 72.0, 74.0, 4),
    step(6000, 3.0, 186.0, 65.0, 65.0, 0),
    step(6140, 5.0, 51.0, 72.0, 74.0, 5),
    step(6180, 5.0, 51.0, 72.0, 74.0, 5),
    step(6450, 5.0, 51.0, 72.0, 74.0, 4),
    step(6500, 198.0, 184.0, 65.0, 30.0, 1),
    step(6500, 133.0, 186.0, 65.0, 65.0, 0),
    step(7000, 198.0, 184.0, 65.0, 30.0, 1),
    step(7100, 5.0, 51.0, 72.0, 74.0, 5),
    step(7300, 328.0, 186.0, 182.0, 65.0, 0),
    step(7700, 5.0, 51.0, 72.0, 74.0, 5),
    step(7800, 263.0, 186.0, 65.0, 65.0, 3),
    step(7900, 5.0, 51.0, 72.0, 74.0, 5),
    step(8200, 68.0, 186.0, 65.0, 65.0, 2),
    step(8250, 0.0, 133.0, 101.0, 95.0, 7),
    step(8250, 104.0, 133.0, 140.0, 95.0, 8),
    step(8250, 247.0, 133.0, 117.0, 95.0, 9),
];
/// L'arrivée.
const GOAL: f32 = 8000.0;

/// Les bandes du décor de chaque couloir (run_bg), dessinées dans cet
/// ordre : nuages, montagnes, arbres, piste. Ligne, hauteur, place (couloir
/// du haut ; +224 pour celui du bas), part de la vitesse.
const STRIPS: [(f32, f32, f32, f32); 4] = [(0.0, 123.0, 0.0, 0.1), (128.0, 108.0, 0.0, 0.05), (237.0, 40.0, 107.0, 0.2), (277.0, 153.0, 71.0, 1.0)];
const CLOUDS: usize = 0;

/// Le sol des deux couloirs, et ce qui y est posé.
const GROUND: [f32; 2] = [70.0, 294.0];
const FLOOR: [f32; 2] = [128.0, 352.0];
/// Les couloirs : Dokuro, Sakura.
const DOKURO_LANE: usize = 0;
const SAKURA_LANE: usize = 1;

/// Un objet du parcours sur la piste : étape, position, vitesse, état (0
/// libre, 1 sur la piste, 2 à hauteur de la coureuse).
#[derive(Clone, Copy, Default)]
struct Item {
    step: usize,
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    state: i32,
}

impl Item {
    fn kind(&self) -> i32 {
        COURSE[self.step].kind
    }

    fn size(&self) -> (f32, f32) {
        let s = COURSE[self.step].src;
        (s[2], s[3])
    }
}

/// Un couloir : la coureuse (place à l'écran, hauteur, distance, vitesse,
/// saut : 0 au sol, 1 appel, 2 montée, 3 chute), son décor et ses objets.
struct Lane {
    x: i32,
    y: f32,
    dist: f32,
    speed: f32,
    vy: f32,
    jump: i32,
    anim: Anim,
    scroll: [f32; 4],
    items: [Item; 10],
    /// La prochaine étape du parcours.
    next: usize,
}

impl Lane {
    fn new(lane: usize, frames: &'static [Frame]) -> Lane {
        Lane {
            x: 100,
            y: GROUND[lane],
            dist: 0.0,
            speed: 0.0,
            vy: 0.0,
            jump: 0,
            anim: Anim::new(frames, RUNNING),
            scroll: [640.0; 4],
            items: [Item::default(); 10],
            next: 0,
        }
    }

    /// Le décor recule de `by` (chaque bande selon sa part).
    fn scroll(&mut self, by: f32) {
        for (s, strip) in self.scroll.iter_mut().zip(STRIPS) {
            *s -= by * strip.3;
        }
    }

    /// Repartie si elle est à l'arrêt, sauf en l'air.
    fn restart(&mut self) {
        if self.anim.count == -1 && self.jump == 0 {
            self.anim.count = self.anim.now().dur;
        }
    }
}

pub struct Run {
    mode: i32,
    lanes: [Lane; 2],
    /// Sakura sonnée : compte de 1 à 100 (0 : en forme).
    stun: i32,
    timer: i32,
    state: i32,
    banner: i32,
    points: i32,
    voice_timer: i32,
}

impl Run {
    /// Mini_RunStart.
    pub fn start(mode: i32) -> Run {
        Run {
            mode,
            lanes: [Lane::new(DOKURO_LANE, &DOKURO), Lane::new(SAKURA_LANE, &SAKURA)],
            stun: 0,
            timer: 0,
            state: 0,
            banner: 0,
            points: 0,
            voice_timer: 0x96,
        }
    }

    /// État 20.
    fn race(&mut self, sys: &mut Sys) {
        // Dokuro accélère seule jusqu'à sa vitesse de pointe.
        let (top, accel) = if self.mode == 1 { (3.2, 0.05) } else { (2.5, 0.025) };
        let d = &mut self.lanes[DOKURO_LANE];
        d.scroll(d.speed);
        d.dist += d.speed;
        if d.speed < top {
            d.speed += accel;
        }
        if d.speed > 0.0 {
            d.restart();
        }
        self.dokuro(sys);
        // ↑ et ↓ comptent à l'appui seulement.
        if sys.input.direct(0, u32::MAX, 1, true) && self.lanes[SAKURA_LANE].jump == 0 {
            self.lanes[SAKURA_LANE].jump = 1;
        }
        let s = &mut self.lanes[SAKURA_LANE];
        if sys.input.direct(1, u32::MAX, 1, true) && s.anim.now().id < 4 && s.y == GROUND[SAKURA_LANE] && s.speed != 0.0 {
            s.anim.play(SLIDE);
            sys.se.play(0x20, 0);
            if self.stun == 0 {
                s.speed += 0.5;
            }
        }
        // ○ : un élan, sauf tant que Sakura est sonnée ; elle ralentit
        // d'elle-même.
        let (stop, wait, cap) = if self.mode == 1 { (0.2, 0x50, 4.5) } else { (0.3, 0x32, 3.4) };
        if s.speed > 0.0 {
            s.speed *= 0.99;
        }
        if s.speed < stop {
            s.speed = 0.0;
        }
        if sys.input.key(3) {
            if self.stun == 0 || self.stun > wait {
                s.speed += 0.5;
            }
            if s.speed > cap {
                s.speed = cap;
            }
            if s.anim.count == -1 {
                s.anim.count = s.anim.now().dur;
            }
        }
        self.sakura_hits(sys);
        let s = &mut self.lanes[SAKURA_LANE];
        s.scroll(s.speed);
        s.dist += s.speed;
        if s.speed == 0.0 && s.jump == 0 {
            s.anim = Anim::new(&SAKURA, RUNNING);
        }
        if self.stun != 0 {
            self.stun += 1;
        }
        if self.stun == 100 {
            self.stun = 0;
        }
        self.follow();
        for lane in &mut self.lanes {
            for s in &mut lane.scroll[1..] {
                if *s <= 0.0 {
                    *s = 640.0;
                }
            }
        }
        if self.lanes[DOKURO_LANE].dist >= GOAL {
            self.state = 30;
            self.finish(sys);
        }
        if self.lanes[SAKURA_LANE].dist >= GOAL {
            self.finish(sys);
            self.state = 40;
        }
        self.voice_timer += 1;
        if self.voice_timer > 500 && self.close_voices() && sys.rng.next() & 7 == 0 {
            self.voice_timer = 0;
            let id = match sys.rng.next() % 7 {
                0 => mini::cast_voice(0, 3, sys),
                1..=3 => mini::type_voice(1, (sys.rng.next() % 3) as usize, sys),
                _ => mini::cast_voice(2, (sys.rng.next() % 3 + 1) as usize, sys),
            };
            sys.voice(id);
        }
    }

    /// Les voix se taisent dans la dernière ligne droite.
    fn close_voices(&self) -> bool {
        self.lanes.iter().all(|l| l.dist < 7300.0)
    }

    /// Une coureuse arrive : tout s'arrête, « ゴール ! ».
    fn finish(&mut self, sys: &mut Sys) {
        self.stun = 0;
        self.lanes[DOKURO_LANE].speed = 0.0;
        self.lanes[SAKURA_LANE].speed = 0.0;
        sys.se.play(0x22, 0x14);
        goal_effect(sys);
    }

    /// Quand les deux coureuses sont à moins de 51 l'une de l'autre, leur
    /// écart se voit à l'écran : chacune s'écarte de x = 100, et son décor
    /// et ses objets suivent.
    fn follow(&mut self) {
        let gap = (self.lanes[DOKURO_LANE].dist - self.lanes[SAKURA_LANE].dist) as i32;
        if gap.abs() >= 0x33 {
            return;
        }
        for (lane, at) in self.lanes.iter_mut().zip([gap + 100, 100 - gap]) {
            let by = (lane.x - at) as f32;
            lane.scroll(by);
            for it in lane.items.iter_mut().filter(|i| i.state != 0 && i.kind() < 7) {
                it.x -= by;
            }
            lane.x = at;
        }
    }

    /// MiniRunDokuroProc : Dokuro saute les haies à sa hauteur ; un objet
    /// qui la touche la ralentit et part en l'air.
    fn dokuro(&mut self, sys: &mut Sys) {
        for k in 0..10 {
            let d = &mut self.lanes[DOKURO_LANE];
            let it = d.items[k];
            if it.state != 2 {
                continue;
            }
            match it.kind() {
                0 => {
                    if d.jump == 0 && d.y == GROUND[DOKURO_LANE] {
                        d.jump = 1;
                    }
                }
                1..=3 => {
                    let (w, h) = it.size();
                    let dx = it.x as i32 + (w as i32 >> 1) - (d.x + 0x5C);
                    let hh = h as i32 >> 1;
                    let dy = it.y as i32 + hh - (d.y as i32 + (0x75 - (d.anim.now().src[3] as i32 >> 1)));
                    if dx.abs() < 0x44 && dy < hh + 0x24 && -(hh + 0x12) < dy {
                        d.speed *= 0.7;
                        sys.se.play(0x29, 0);
                        bump_effect(sys, it.x as i32, it.y as i32, COURSE[it.step].src);
                        star_effect(sys, it.x as i32, it.y as i32);
                        d.items[k].state = 0;
                        if self.voice_timer > 400 && self.close_voices() && sys.rng.next() & 7 == 0 {
                            self.voice_timer = 0;
                            sys.voice(mini::cast_voice(1, 1, sys));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// MiniRunHitRight : ce que rencontre Sakura. Un obstacle qui roule,
    /// une balle ou un objet qui monte la sonnent ; une tête de mort se
    /// ramasse ; le reste (une haie) l'arrête.
    fn sakura_hits(&mut self, sys: &mut Sys) {
        let s = &mut self.lanes[SAKURA_LANE];
        let (x, y) = (s.x as f32, s.y);
        let body = s.anim.now().src[3];
        for it in s.items.iter_mut().filter(|i| i.state == 2) {
            let (w, h) = it.size();
            match it.kind() {
                6 => {}
                1 => {
                    let feet = it.y + 15.0;
                    if self.stun == 0 && it.x < x + 92.0 && x + 37.0 < it.x && y + (128.0 - body) < feet && feet < y + 117.0 {
                        sys.se.play(0xF, 0);
                        self.stun = 1;
                    }
                }
                2 | 3 => {
                    if self.stun == 0 {
                        let dx = it.x as i32 + (w as i32 >> 1) - (s.x + 0x40);
                        let hh = h as i32 >> 1;
                        let dy = it.y as i32 + hh - (y as i32 + (0x80 - (body as i32 >> 1)));
                        if dx.abs() < 0x36 && dy < hh + 0x12 && -(hh + 0x12) < dy {
                            sys.se.play(0xF, 0);
                            self.stun = 1;
                        }
                    }
                }
                k if k == 4 || (k == 5 && self.mode == 1) => {
                    if it.x + 2.0 < x + 92.0 && x + 55.0 < it.x + 38.0 && it.y < y + 117.0 && y + 14.0 < it.y + 40.0 {
                        self.points += 1;
                        it.state = 0;
                        pickup_effect(sys, s.x, y as i32);
                        sys.se.play(0x11, 0);
                    }
                }
                _ => {
                    if it.x < x + 93.0 && x + 60.0 < it.x + w && it.y < y + 100.0 && y + 14.0 < it.y + h {
                        s.speed = 0.0;
                        s.anim = Anim::new(&SAKURA, RUNNING);
                    }
                }
            }
        }
    }

    /// Mini_RunJump : l'appel, la montée, la chute, l'arrivée au sol.
    fn jump(&mut self, lane: usize, sys: &mut Sys) {
        let l = &mut self.lanes[lane];
        match l.jump {
            1 => {
                sys.se.play(0x21, 0);
                l.anim.play(JUMP);
                l.vy = -6.0;
                l.jump = 2;
            }
            2 => {
                l.vy += 0.25;
                if l.vy > 0.0 {
                    l.jump = 3;
                }
            }
            3 => l.vy = (l.vy + 0.25).min(6.0),
            _ => l.vy = 0.0,
        }
        l.y += l.vy;
        self.landing(lane);
        let l = &mut self.lanes[lane];
        if l.y > GROUND[lane] {
            l.y = GROUND[lane];
            l.jump = 0;
            l.anim.count = 0;
        }
    }

    /// MiniRunHitDown : on peut courir sur une haie. Quittée par l'avant
    /// ou l'arrière, la coureuse tombe ; en tombant, elle se pose dessus.
    /// Chaque coureuse est comparée à son propre sol (le jeu prend celui de
    /// Sakura pour les deux : bug corrigé, voir le README).
    fn landing(&mut self, lane: usize) {
        let racing = self.state == 20;
        let l = &mut self.lanes[lane];
        let x = l.x as f32;
        if racing && l.y != GROUND[lane] && l.jump == 0 {
            for it in l.items.iter().filter(|i| i.state == 2 && i.kind() == 0) {
                if x + 92.0 < it.x || it.x + it.size().0 < x + 51.0 {
                    l.jump = 3;
                    l.anim.play(JUMP);
                }
            }
        }
        if l.jump == 3 {
            for it in l.items.iter().filter(|i| i.state > 0 && i.kind() == 0) {
                let (w, h) = it.size();
                if it.x + 2.0 < x + 92.0 && x + 55.0 < it.x + w - 5.0 && it.y < l.y + 117.0 && l.y + 14.0 < it.y + h {
                    l.y = it.y - 117.0;
                    l.vy = 0.0;
                    l.jump = 0;
                    if l.anim.now().id > 4 {
                        l.anim.play(RUNNING);
                    }
                }
            }
        }
    }

    /// Mini_RunAnim : à la fin d'une séquence, les poses de fin restent,
    /// le reste reprend la course (à l'arrêt si la coureuse ne court pas).
    fn animate(&mut self, lane: usize) {
        let l = &mut self.lanes[lane];
        if l.anim.step().is_some() {
            match l.anim.now().id {
                12 => l.anim.play(LOSER),
                14 => l.anim.play(WINNER),
                _ => l.anim.at = RUNNING,
            }
            if l.speed > 0.0 {
                l.anim.count = l.anim.now().dur;
            }
        }
    }

    /// Mini_RunItem : les étapes du parcours arrivent sur la piste (une par
    /// image, à 640 devant), puis avancent, rebondissent et défilent.
    fn items(&mut self, lane: usize, sys: &mut Sys) {
        let racing = self.state == 20;
        let challenge = self.mode == 1;
        let l = &mut self.lanes[lane];
        let ahead = 100 - l.x;
        // Le ruban d'arrivée arrête la lecture du parcours avant sa fin.
        if let Some(step) = COURSE.get(l.next).filter(|s| s.kind < 8)
            && (step.at - ahead) as f32 <= l.dist + 640.0
            && let Some(it) = l.items.iter_mut().find(|i| i.state == 0)
        {
            let x = (step.at - ahead - l.dist as i32) as f32;
            let (top, floor) = (FLOOR[lane] - 128.0, FLOOR[lane]);
            *it = Item { step: l.next, x, y: floor, vx: 0.0, vy: 0.0, state: 1 };
            l.next += 1;
            match step.kind {
                2 => (it.vx, it.y, it.vy) = (-1.25, floor - 40.0, 0.28),
                1 => {
                    (it.vx, it.y) = (-3.0, floor - 40.0);
                    sys.se.play(0x24, 10);
                }
                3 => {
                    (it.y, it.vy) = (top, 1.5);
                    sys.se.play(0x25, 0);
                }
                // Les têtes de mort ne viennent qu'en challenge, dans le
                // couloir de Sakura.
                4 | 5 if !(challenge && lane == SAKURA_LANE) => it.state = 0,
                4 => it.y = 370.0,
                5 => it.y = 230.0,
                7 => it.x = (step.at - l.dist as i32) as f32,
                _ => {}
            }
        }
        for it in l.items.iter_mut().filter(|i| i.state > 0) {
            match it.kind() {
                // Sakura ou Dokuro passe le ruban : il se casse.
                7 if 0xFF < l.x && l.x < 0x141 => {
                    it.step = l.next;
                    l.next += 1;
                }
                8 if l.x > 0x140 => it.step = l.next,
                _ => {}
            }
            if racing {
                it.x -= l.speed;
            }
            it.x += it.vx;
            it.y += it.vy;
            match it.kind() {
                2 => {
                    it.vy += 0.28;
                    if it.y > FLOOR[lane] {
                        it.vy = -8.7;
                        sys.se.play(0x27, 0);
                    }
                }
                3 if it.y < FLOOR[lane] - 128.0 || FLOOR[lane] < it.y => it.vy = -it.vy,
                _ => {}
            }
            let w = it.size().0;
            it.state = if (l.x + 0x66) as f32 <= it.x || it.x + w <= (l.x + 0x16) as f32 { 1 } else { 2 };
            if it.x <= -w {
                it.state = 0;
            }
        }
    }

    /// La gagnante finit sa course vers la droite pendant que l'autre la
    /// regarde ; puis chacune prend sa pose. Vrai quand c'est fait.
    fn run_out(&mut self, winner: usize, sys: &mut Sys) -> bool {
        let loser = 1 - winner;
        let w = &mut self.lanes[winner];
        if w.x < 0x1C2 {
            w.speed = 2.0;
            w.x += 2;
            w.restart();
            let l = &mut self.lanes[loser];
            if l.anim.count == -1 {
                l.anim.play(WATCHING);
            }
            return false;
        }
        for l in &mut self.lanes {
            l.speed = 0.0;
        }
        self.lanes[winner].anim.play(WINNER);
        self.lanes[loser].anim.play(LOSER);
        sys.se.play(if winner == SAKURA_LANE { 7 } else { 8 }, 0);
        true
    }
}

impl Minigame for Run {
    /// Mini_RunProc.
    fn step(&mut self, sys: &mut Sys) -> bool {
        match self.state {
            35 | 45 => {
                let won = self.state == 45;
                if mini::ending(sys, &mut self.timer, won, false) {
                    sys.points = if won { self.points } else { 0 };
                    sys.won = won;
                    return true;
                }
                if self.banner < 0xB6 {
                    self.banner += 2;
                }
            }
            40 => {
                if self.run_out(SAKURA_LANE, sys) {
                    self.state = 45;
                }
            }
            30 => {
                if self.run_out(DOKURO_LANE, sys) {
                    self.state = 35;
                }
            }
            20 => self.race(sys),
            10 => {
                self.timer += 1;
                if self.timer == 0xD2 {
                    self.timer = 0;
                    self.state = 20;
                }
            }
            5 => {
                self.timer = (self.timer + 1) % 0x28;
                if mini::any_key(sys) {
                    self.timer = 0;
                    mini::start_effect(sys, COMMON, [276.0, 397.0, 236.0, 72.0]);
                    sys.se.play(0x23, 0xBE);
                    self.state = 10;
                }
            }
            0 if mini::intro(sys, &mut self.timer, 1) => self.state = 5,
            _ => {}
        }
        for lane in [DOKURO_LANE, SAKURA_LANE] {
            self.jump(lane, sys);
        }
        for lane in [DOKURO_LANE, SAKURA_LANE] {
            self.animate(lane);
        }
        for lane in [DOKURO_LANE, SAKURA_LANE] {
            self.items(lane, sys);
        }
        for lane in &mut self.lanes {
            let s = &mut lane.scroll[CLOUDS];
            *s -= 0.4;
            if *s <= 0.0 {
                *s = 640.0;
            }
        }
        false
    }

    /// Mini_RunTran.
    fn draw(&self, gfx: &mut Gfx, _sys: &Sys) {
        let tran = |gfx: &mut Gfx, tex: &'static str, src: [f32; 4], dst: [f32; 4], prio: i32| gfx.tran(tex, src, dst, prio, 0x7F, Blend::Normal, GREY);
        // Le décor défile en boucle : chaque bande en deux morceaux.
        for (lane, l) in self.lanes.iter().enumerate() {
            for (&s, (v, h, y, _)) in l.scroll.iter().zip(STRIPS) {
                let y = y + 224.0 * lane as f32;
                let cut = (s * 0.8) as i32 as f32;
                let at = s as i32 as f32;
                tran(gfx, BG, [512.0 - cut, v, cut, h], [0.0, y, at, h], 0x8C);
                tran(gfx, BG, [0.0, v, 512.0 - cut, h], [at, y, 640.0 - at, h], 0x8C);
            }
        }
        for k in 0..10 {
            for (lane, l) in self.lanes.iter().enumerate() {
                let it = &l.items[k];
                if it.state <= 0 {
                    continue;
                }
                let kind = it.kind();
                let src = COURSE[it.step].src;
                let (mut y, mut w, mut h, mut prio) = (it.y as i32 as f32, src[2], src[3], 0xA0);
                match kind {
                    6 => (y, h, prio) = (y + 21.0, h + 14.0, 0x9E),
                    4 | 5 if lane == SAKURA_LANE => (w, h) = (40.0, 40.0),
                    7.. => (y, prio) = (y - 10.0, 0x9E),
                    _ => {}
                }
                // Le couloir du bas prend aussi les têtes de mort dans
                // l'image du challenge.
                let tex = if kind < [4, 6][lane] { CH } else { PARTS };
                tran(gfx, tex, src, [it.x as i32 as f32, y, w, h], prio);
            }
        }
        if self.mode == 1 {
            tran(gfx, CH, [5.0, 51.0, 72.0, 74.0], [440.0, 393.0, 40.0, 40.0], 0xA3);
            mini::digit(gfx, CH, self.points / 10, [500.0, 400.0, 21.0, 30.0], 0xA3);
            mini::digit(gfx, CH, self.points % 10, [530.0, 400.0, 21.0, 30.0], 0xA3);
        }
        // Le parcours entre les deux couloirs, et où en sont les coureuses.
        tran(gfx, PARTS, [0.0, 4.0, 80.0, 61.0], [32.0, 194.0, 80.0, 61.0], 0xA0);
        tran(gfx, PARTS, [77.0, 33.0, 7.0, 3.0], [113.0, 223.0, 418.0, 3.0], 0x9F);
        tran(gfx, PARTS, [81.0, 3.0, 70.0, 61.0], [532.0, 193.0, 70.0, 61.0], 0xA0);
        let mark = |dist: f32, add: i32| ((dist * 0.06) as i32 + add) as f32;
        tran(gfx, PARTS, [197.0, 0.0, 50.0, 42.0], [mark(self.lanes[DOKURO_LANE].dist, 0x36), 184.0, 50.0, 42.0], 0xA0);
        tran(gfx, PARTS, [151.0, 0.0, 43.0, 45.0], [mark(self.lanes[SAKURA_LANE].dist, 0x39), 218.0, 43.0, 45.0], 0xA0);
        // Sakura clignote quand elle est sonnée.
        let s = &self.lanes[SAKURA_LANE];
        if self.stun % 10 <= 4 {
            let src = s.anim.now().src;
            tran(gfx, CHARA, src, [s.x as f32, (s.y as i32 + 0x80 - src[3] as i32) as f32, src[2], src[3]], 0xA2);
        }
        let d = &self.lanes[DOKURO_LANE];
        tran(gfx, CHARA, d.anim.now().src, [d.x as f32, d.y as i32 as f32, 128.0, 128.0], 0xA1);
        let b = self.banner as f32;
        match self.state {
            5 => mini::draw_push(gfx, COMMON, self.timer),
            35 => {
                tran(gfx, COMMON, [203.0, 194.0, 165.0, 103.0], [237.0, b, 165.0, 103.0], 0xA2);
                gfx.paint([0.0, 0.0, 640.0, 448.0], 0xA0, 0x40, [0x14; 3]);
            }
            45 => {
                tran(gfx, COMMON, [395.0, 101.0, 95.0, 54.0], [270.0, b - 27.0, 95.0, 54.0], 0xA2);
                tran(gfx, COMMON, [350.0, 6.0, 164.0, 83.0], [238.0, b, 164.0, 83.0], 0xA2);
                tran(gfx, COMMON, [393.0, 164.0, 105.0, 58.0], [205.0, b + 13.0, -105.0, 58.0], 0xA2);
                tran(gfx, COMMON, [393.0, 164.0, 105.0, 58.0], [435.0, b + 13.0, 105.0, 58.0], 0xA2);
            }
            _ => {}
        }
    }
}

/// Mini_RunAtariEffect : l'objet qui a touché Dokuro s'envole en
/// rapetissant et en tournant (12° par image).
fn bump_effect(sys: &mut Sys, x: i32, y: i32, src: [f32; 4]) {
    let mut d = EffectDef::ready();
    d.texture = Some(CH);
    d.src = src;
    d.size = (src[2], src[3]);
    d.prio = 0xA0;
    d.group = 1;
    d.pos = (x, y);
    d.keys[0] = Key { dur: 0x14, alpha: 100, rgb: [0x7F; 3], scale: 0.3, x: x + 200, y: y - 0x28, brake: 0.0, spin: 12.0 };
    d.keys[1] = Key { dur: 10, alpha: 100, rgb: [0x7F; 3], scale: 0.1, spin: 12.0, ..Key::NONE };
    sys.effects.make(d);
}

/// Mini_RunAtariEffect2 : l'étoile du choc.
fn star_effect(sys: &mut Sys, x: i32, y: i32) {
    let mut d = EffectDef::ready();
    d.texture = Some(CH);
    d.src = [238.0, 60.0, 71.0, 68.0];
    d.size = (71.0, 68.0);
    d.prio = 0xA0;
    d.group = 1;
    d.pos = (x, y);
    d.keys[0] = Key { dur: 5, alpha: 100, rgb: [0x7F; 3], scale: 1.0, ..Key::NONE };
    sys.effects.make(d);
}

/// Mini_RunItemEffectMake : la tête de mort ramassée monte au-dessus de
/// Sakura.
fn pickup_effect(sys: &mut Sys, x: i32, y: i32) {
    let mut d = EffectDef::ready();
    d.texture = Some(CH);
    d.src = [5.0, 51.0, 74.0, 72.0];
    d.size = (40.0, 40.0);
    d.prio = 0xA3;
    d.group = 1;
    d.alpha = 0x80;
    d.pos = (x + 0x40, y + 0x14);
    d.keys[0] = Key { dur: 0x14, alpha: 0x80, rgb: [0x80; 3], scale: 1.0, x: x + 0x40, y: y - 0x14, brake: 0.0, spin: 0.0 };
    sys.effects.make(d);
}

/// Mini_RunGoalEffectMake : « ゴール ! » grandit au milieu, reste, pâlit.
fn goal_effect(sys: &mut Sys) {
    let mut d = EffectDef::ready();
    d.texture = Some(COMMON);
    d.src = [16.0, 396.0, 237.0, 78.0];
    d.size = (237.0, 78.0);
    d.prio = 0xA2;
    d.group = 1;
    d.scale = 0.5;
    d.pos = (0x140, 0xE0);
    d.keys[0] = Key { dur: 10, alpha: 0x7F, rgb: [0x7F; 3], scale: 1.0, ..Key::NONE };
    d.keys[1] = Key { dur: 0x28, alpha: 0x7F, ..Key::NONE };
    d.keys[2] = Key { dur: 10, alpha: 0x40, ..Key::NONE };
    sys.effects.make(d);
}
