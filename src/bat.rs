//! Le mini-jeu de la batte (Mini_BatStart, Mini_BatProc, Mini_BatTran,
//! mini_bat.cpp) : Dokuro frappe Sakura avec Excalibolg. Trois essais :
//! puissance, timing, angle, puis le vol ; gagné à 600 m. En mode
//! challenge, des têtes de mort à attraper en vol.
//!
//! Les états gardent les valeurs du jeu (des caractères en C) : 0 voix
//! d'ouverture, 3 PUSH BUTTON, 5 nouvel essai, 10 puissance, 15 MAX,
//! 20 timing, 25 timing parfait, 30 angle, 35 angle parfait, 40 élan,
//! 50 frappe, 60 vol, 63 et 64 Dokuro revient, 70 essai fini, 80 gagné,
//! 90 perdu.

use crate::engine::{EffectDef, Key};
use crate::sys::Sys;
use crate::gfx::{Blend, Gfx, GREY};
use crate::mini::{self, Minigame};

/// Les images du mini-jeu (BAT.UNI, dans l'ordre de Mini_BatInit).
pub const TEXTURES: [(&str, u32); 7] = [("bat_bg1", 0), ("bat_bg2", 1), ("bat_sora", 2), ("bat_pa-tu", 3), ("bat_eff", 4), ("bat_kyoutu", 5), ("bat_ch", 6)];

const BG1: &str = "bat_bg1";
const BG2: &str = "bat_bg2";
const SKY: &str = "bat_sora";
const PARTS: &str = "bat_pa-tu";
const EFF: &str = "bat_eff";
const COMMON: &str = "bat_kyoutu";
const CH: &str = "bat_ch";

/// L'élan de Dokuro (0x35c9b0) : rectangle dans bat_pa-tu, durée (images),
/// numéro. Après la dernière, l'animation revient au début.
const SWING: [([f32; 4], i32, i32); 4] =
    [([0.0, 256.0, 128.0, 128.0], 5, 0), ([128.0, 256.0, 128.0, 128.0], 5, 1), ([256.0, 256.0, 128.0, 128.0], 5, 2), ([384.0, 256.0, 128.0, 128.0], 32767, 3)];

/// Les têtes de mort du mode challenge (0x35c9f0) : x sur le parcours
/// (pixels, 10 par mètre), y. Après la liste, le jeu lit des zéros.
const MARKS: [(i32, i32); 66] = [
    (6000, 370), (6100, 370), (6200, 370), (6300, 370), (6400, 370), (6500, 320), (6600, 370), (6700, 320), (6800, 370), (6900, 320), (7000, 370),
    (7050, 320), (7100, 370), (7150, 320), (7200, 370), (7250, 320), (7300, 370), (7350, 320), (7400, 370), (7450, 320), (7500, 370), (7550, 320),
    (7600, 370), (7650, 320), (7700, 370), (7750, 320), (7800, 370), (7850, 320), (7900, 370), (7950, 320), (8000, 370), (8050, 320), (8100, 370),
    (8150, 320), (8200, 370), (8250, 320), (8300, 370), (8350, 320), (8400, 370), (8450, 320), (8500, 370), (8500, 280), (8550, 320), (8600, 370),
    (8600, 280), (8650, 320), (8700, 370), (8700, 280), (8750, 320), (8800, 370), (8800, 280), (8850, 320), (8900, 370), (8900, 280), (8950, 320),
    (9000, 370), (9000, 280), (9050, 320), (9050, 370), (9100, 370), (9100, 280), (9150, 320), (9150, 370), (9200, 370), (9200, 280), (9250, 320),
];

/// Ce qui reste de la liste après les 66 premières (le jeu va jusqu'à
/// 10 000).
const MARKS_END: [(i32, i32); 15] =
    [(9250, 370), (9300, 370), (9300, 280), (9350, 320), (9350, 370), (9400, 370), (9400, 280), (9450, 320), (9450, 370), (9500, 370), (9550, 370), (9600, 370), (9650, 370), (9700, 370), (9750, 370)];
const MARKS_LAST: [(i32, i32); 6] = [(9800, 370), (9850, 370), (9900, 370), (9950, 370), (10000, 370), (0, 0)];

fn mark(i: usize) -> (i32, i32) {
    MARKS.iter().chain(&MARKS_END).chain(&MARKS_LAST).nth(i).copied().unwrap_or((0, 0))
}

/// Gagné à partir de 600 m.
const GOAL: i32 = 600;

/// Les variables du mini-jeu (0x365f08 à 0x365f84).
pub struct Bat {
    mode: i32,
    marks_got: i32,
    mark_next: usize,
    swing: usize,
    swing_count: i32,
    frames: i32,
    angle: i32,
    angle_step: i32,
    power: i32,
    power_step: i32,
    timing: i32,
    timing_step: i32,
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    mult: f32,
    spin: i32,
    spin_step: i32,
    ground: f32,
    clouds: f32,
    sky: f32,
    banner: i32,
    camera_y: i32,
    best: i32,
    distance: i32,
    state: i32,
    tries: i32,
    timer: i32,
    max_flash: i32,
    dokuro_x: i32,
    /// Têtes de mort à l'écran : x, y, présente.
    marks: [(i32, i32, bool); 25],
}

impl Bat {
    /// Mini_BatStart.
    pub fn start(mode: i32) -> Bat {
        Bat {
            mode,
            marks_got: 0,
            mark_next: 0,
            swing: 0,
            swing_count: -1,
            frames: 0,
            angle: 0,
            angle_step: 3,
            power: 0,
            power_step: 2,
            timing: 0,
            timing_step: 2,
            x: 260.0,
            y: 370.0,
            vx: 0.0,
            vy: 0.0,
            mult: 0.0,
            spin: 0,
            spin_step: 0,
            ground: 320.0,
            clouds: 640.0,
            sky: 0.0,
            banner: 0,
            camera_y: 0,
            best: 0,
            distance: 0,
            state: 0,
            tries: 0,
            timer: 0,
            max_flash: 0,
            dokuro_x: 0x2C4,
            marks: [(0, 0, false); 25],
        }
    }

    /// Un nombre de 0 à n - 1, comme le jeu (le reste de rand(), qui peut
    /// être négatif en C mais ne l'est jamais ici).
    fn rand(sys: &mut Sys, n: i32) -> i32 {
        sys.rng.next() % n
    }

    /// Mini_BatProc : une image ; vrai quand le mini-jeu est fini.
    fn frame(&mut self, sys: &mut Sys) -> bool {
        match self.state {
            // Gagné ou perdu : le bandeau descend, deux voix, puis un
            // bouton (ou 300 images) termine.
            80 | 90 => {
                let won = self.state == 80;
                if self.banner < 0xB6 {
                    self.banner += 2;
                }
                if mini::ending(sys, &mut self.timer, won, true) {
                    sys.points = if won { self.marks_got } else { 0 };
                    sys.won = won;
                    return true;
                }
            }
            // L'essai est fini : la caméra revient ; après trois essais,
            // gagné ou perdu ; sinon ○ relance.
            70 => {
                sys.camera.prepare();
                sys.camera.set(10, 0, 0, 100);
                self.best = self.best.max(self.distance);
                if self.tries > 2 {
                    self.state = if self.best < GOAL { 90 } else { 80 };
                }
                if sys.input.key(3) {
                    self.state = 5;
                }
            }
            // Dokuro revient en courant, de gauche (réussi) ou de droite.
            64 => {
                self.frames += 1;
                if self.dokuro_x < 0x140 {
                    self.dokuro_x += 8;
                } else {
                    self.state = 70;
                }
            }
            63 => {
                self.frames += 1;
                if self.dokuro_x < 0x141 {
                    self.state = 70;
                } else {
                    self.dokuro_x -= 8;
                }
            }
            60 => self.fly(sys),
            // La frappe, à la 3e image de l'élan.
            50 => {
                if SWING[self.swing].2 == 2 && self.swing_count == 5 {
                    hit_effect(sys);
                    let mut t = self.timing as f32;
                    if t > 50.0 {
                        t = (50 - (t as i32) % 51) as f32;
                    }
                    let force = self.mult * self.power as f32 * (t * 0.01 + 0.5);
                    let a = (self.angle as f32 * std::f32::consts::PI) / 180.0;
                    self.vx = force * 0.4 * a.cos();
                    self.vy = force * 0.2 * a.sin();
                    self.spin_step = 10;
                    self.max_flash = 0;
                    self.state = 60;
                }
            }
            // L'élan : la voix de Dokuro, puis la batte.
            40 => {
                self.timer += 1;
                if self.timer == 2 {
                    sys.voice(mini::cast_voice(1, 4, sys));
                } else if self.timer > 0x95 {
                    self.timer = 0;
                    self.swing_count = SWING[self.swing].1;
                    swing_effect(sys);
                    sys.se.play(0x1A, 5);
                    self.state = 50;
                }
            }
            // Parfait : zoom, éclat, +0,04 ; 70 images.
            15 | 25 | 35 => {
                self.timer += 1;
                if self.timer == 1 {
                    sys.camera.prepare();
                    sys.camera.set(0x14, -0x78, 0x80, 200);
                    let (u, v) = match self.state {
                        15 => (0, 0),
                        25 => (0x100, 0),
                        _ => (0, 0x100),
                    };
                    clear_effect(sys, u, v);
                    sys.se.play(0x1D, 10);
                } else if self.timer == 0x46 {
                    self.timer = 0;
                    sys.camera.prepare();
                    sys.camera.set(10, 0, 0, 100);
                    sys.se.stop(0);
                    self.mult += 0.04;
                    self.state = match self.state {
                        15 => 20,
                        25 => 30,
                        _ => 40,
                    };
                }
            }
            30 => self.gauge_angle(sys),
            20 => self.gauge_timing(sys),
            10 => self.gauge_power(sys),
            5 => self.new_try(sys),
            3 => {
                self.timer = (self.timer + 1) % 0x28;
                if mini::any_key(sys) {
                    self.timer = 0;
                    self.state = 5;
                }
            }
            // La voix d'ouverture (○ la coupe) ; avec Dokuro, une seconde.
            0 if mini::intro(sys, &mut self.timer, 0) => self.state = 3,
            _ => {}
        }
        self.clouds -= 0.4;
        if self.clouds <= 0.0 {
            self.clouds = 640.0;
        }
        if self.max_flash != 0 {
            self.max_flash += 1;
        }
        self.animate();
        false
    }

    /// Mini_BatAnim : l'élan avance quand son compte arrive à 0 (à -1, il
    /// reste sur la première image).
    fn animate(&mut self) {
        if self.swing_count < 1 {
            if self.swing_count == 0 {
                self.swing += 1;
                self.swing_count = SWING.get(self.swing).map_or(-1, |s| s.1);
            }
        } else {
            self.swing_count -= 1;
        }
        if self.swing >= SWING.len() {
            self.swing = 0;
        }
    }

    /// État 5 : un nouvel essai.
    fn new_try(&mut self, sys: &mut Sys) {
        self.power = 0;
        self.angle = 0;
        if self.mode == 1 {
            self.power_step = (sys.rng.next() & 3) + 2;
            self.angle_step = if sys.rng.next() & 1 == 0 { 3 } else { 5 };
            self.marks = [(0, 0, false); 25];
            self.mark_next = 0;
        } else {
            self.power_step = 2;
            self.angle_step = 3;
        }
        self.timing = 0;
        self.timing_step = Bat::rand(sys, 5) + 1;
        self.distance = 0;
        self.x = 260.0;
        self.vx = 0.0;
        self.y = 370.0;
        self.vy = 0.0;
        self.ground = 320.0;
        self.swing = 0;
        self.swing_count = -1;
        self.tries += 1;
        self.frames = 0;
        self.mult = 1.0;
        self.state = 10;
    }

    /// État 10 : la puissance monte de 0 à 120 (dents de scie) ; ○ arrête.
    /// MAX à 115 et plus.
    fn gauge_power(&mut self, sys: &mut Sys) {
        if self.timer == 0 {
            if self.power == 0 {
                sys.se.play(0x1B, 0);
            }
            self.power += self.power_step;
            if self.power > 0x79 {
                self.power = 0;
                if self.mode == 1 {
                    self.power_step = (sys.rng.next() & 3) + 2;
                }
            }
            if sys.input.key(3) {
                self.power_step = 0;
                sys.se.stop(0);
                if self.power < 0x73 {
                    sys.se.play(0x1C, 0);
                    self.timer = 1;
                } else {
                    if self.power == 0x78 {
                        self.max_flash = 1;
                    }
                    self.state = 15;
                }
            }
        }
        self.wait_miss(sys, 20);
    }

    /// Raté : 50 images, puis la jauge suivante.
    fn wait_miss(&mut self, sys: &mut Sys, next: i32) {
        if self.timer != 0 {
            self.timer += 1;
        }
        if self.timer == 0x32 {
            sys.se.stop(0);
            self.timer = 0;
            self.state = next;
        }
    }

    /// État 20 : le repère va et vient de 0 à 100, à une vitesse tirée au
    /// hasard à chaque bord ; parfait sur 50.
    fn gauge_timing(&mut self, sys: &mut Sys) {
        if self.timer == 0 {
            self.timing += self.timing_step;
            if self.timing < 0x65 {
                if self.timing < 1 {
                    self.timing = 0;
                    self.timing_step = Bat::rand(sys, 5) + 1;
                    sys.se.stop(0);
                    sys.se.play(0x1B, 0);
                }
            } else {
                self.timing = 100;
                self.timing_step = -(Bat::rand(sys, 5) + 1);
                sys.se.stop(0);
                sys.se.play(0x1B, 0);
            }
            if sys.input.key(3) {
                self.timer = 0;
                self.timing_step = 0;
                sys.se.stop(0);
                if self.timing == 0x32 {
                    self.state = 25;
                } else {
                    sys.se.play(0x1C, 0);
                    self.timer = 1;
                }
            }
        }
        self.wait_miss(sys, 30);
    }

    /// État 30 : l'aiguille va de 0° à 90° et revient ; parfait à 45°.
    fn gauge_angle(&mut self, sys: &mut Sys) {
        if self.timer == 0 {
            self.angle += self.angle_step;
            if self.mode == 0 {
                if self.angle == 90 || self.angle == 0 {
                    sys.se.stop(0);
                    sys.se.play(0x1B, 0);
                    self.angle_step = -self.angle_step;
                }
            } else if self.angle == 0 {
                sys.se.stop(0);
                sys.se.play(0x1B, 0);
                self.angle_step = if sys.rng.next() & 1 == 0 { 3 } else { 5 };
            } else if self.angle == 90 {
                sys.se.stop(0);
                sys.se.play(0x1B, 0);
                self.angle_step = if sys.rng.next() & 1 == 0 { -3 } else { -5 };
            }
            if sys.input.key(3) {
                self.angle_step = 0;
                sys.se.stop(0);
                if self.angle == 45 {
                    self.state = 35;
                } else {
                    sys.se.play(0x1C, 0);
                    self.timer = 1;
                }
            }
        }
        self.wait_miss(sys, 40);
    }

    /// État 60 : le vol de Sakura, en pixels par image (10 par mètre).
    fn fly(&mut self, sys: &mut Sys) {
        self.x += self.vx;
        self.distance = ((self.x - 260.0) as i32) / 10;
        if self.y >= 390.0 {
            // Rebond au sol.
            if self.vx > 3.0 || self.vy < -1.5 {
                down_effect(sys, self.vx as i32);
            }
            self.vx *= 0.6;
            self.y = 390.0;
            self.vy *= if self.vy < -10.0 { 0.3 } else { 0.5 };
            self.vy = -self.vy;
            self.spin_step = self.vx as i32;
            sys.se.play(0x0B, 0);
        }
        self.y -= self.vy;
        let drag = (self.vy * self.vy + self.vx * self.vx).sqrt() * 0.0018;
        self.vx = (self.vx - drag).max(0.0);
        self.vy = self.vy - drag - 0.08;
        self.ground -= self.vx;
        if self.ground <= -320.0 {
            self.ground = 320.0;
        }
        self.clouds -= self.vx * 0.2;
        if self.y - 224.0 < 0.0 {
            self.sky += self.vy * 0.1;
        }
        if self.sky >= 448.0 {
            self.sky = 0.0;
        } else if self.sky < 0.0 {
            self.sky = 448.0;
        }
        self.spin += self.spin_step;
        if self.mode == 1 {
            self.marks_step(sys);
        }
        self.camera_y = if self.y - 224.0 < 0.0 { self.y as i32 - 0xE0 } else { 0 };
        sys.camera.prepare();
        sys.camera.set(0, 0, self.camera_y, 100);
        if self.vx < 1.0 && self.vy < 1.0 && self.vy > -1.0 && self.y >= 390.0 {
            if self.distance < GOAL {
                self.dokuro_x = 0x2C4;
                self.state = 63;
            } else {
                self.dokuro_x = -0x40;
                self.state = 64;
            }
        }
    }

    /// Mini_BatDkMkMain : les têtes de mort arrivent, défilent, et Sakura
    /// les attrape en passant.
    fn marks_step(&mut self, sys: &mut Sys) {
        let travelled = self.x - 260.0;
        for i in 0..self.marks.len() {
            let (mx, my) = mark(self.mark_next);
            if (mx as f32) < travelled + 320.0 && !self.marks[i].2 {
                self.marks[i] = (((mx as f32 - travelled) + 320.0) as i32, my, true);
                self.mark_next += 1;
            }
            let m = &mut self.marks[i];
            if m.2 {
                m.0 = (m.0 as f32 - self.vx) as i32;
                if m.0 < -0x28 {
                    m.2 = false;
                }
                if m.0 < 0x154 && m.0 > 300 && (m.1 as f32) < self.y + 25.0 && self.y - 25.0 < m.1 as f32 {
                    self.marks_got += 1;
                    mark_effect(sys, m.0, m.1);
                    m.2 = false;
                }
            }
        }
    }

    /// Mini_BatTran : l'image.
    fn paint(&self, gfx: &mut Gfx, sys: &Sys) {
        let cam = &sys.camera;
        if self.state == 3 {
            mini::draw_push(gfx, COMMON, self.timer);
        }
        if !matches!(self.state, 15 | 25 | 35) {
            self.draw_scenery(gfx, sys);
            if self.state < 60 {
                self.draw_gauges(gfx);
            }
        }
        // Dokuro et son élan, qui s'éloignent quand Sakura vole.
        let dx = 180.0 - (self.x - 260.0);
        if dx > -128.0 && self.y > 0.0 {
            cam.draw(gfx, PARTS, SWING[self.swing].0, [dx, 355.0, 128.0, 128.0], 0xA0, 0x7F);
        }
        if matches!(self.state, 63 | 64) {
            let u = match self.frames % 15 {
                0..5 => 0.0,
                5..10 => 128.0,
                _ => 256.0,
            };
            cam.draw(gfx, PARTS, [u, 384.0, 128.0, 128.0], [self.dokuro_x as f32, 210.0, 128.0, 128.0], 0xA0, 0x7F);
        } else if self.state > 69 {
            let d = if self.tries < 3 { self.distance } else { self.best };
            let u = if d < GOAL { 256.0 } else { 384.0 };
            cam.draw(gfx, EFF, [u, 384.0, 128.0, 128.0], [self.dokuro_x as f32, 210.0, 128.0, 128.0], 0xA0, 0x7F);
        }
        if self.mode == 1 {
            gfx.tran(CH, [5.0, 51.0, 72.0, 74.0], [60.0, 25.0, 40.0, 40.0], 0xA3, 0x7F, Blend::Normal, GREY);
            mini::digit(gfx, CH, self.marks_got / 10, [120.0, 25.0, 30.0, 43.0], 0xA3);
            mini::digit(gfx, CH, self.marks_got % 10, [170.0, 25.0, 30.0, 43.0], 0xA3);
        }
        // Sakura.
        let sx = self.x.min(320.0);
        let sy = if self.y < 224.0 { 224.0 } else { self.y };
        let sakura = [316.0, 124.0, 42.0, 49.0];
        if self.state < 50 {
            cam.draw(gfx, PARTS, sakura, [sx, sy, 41.0, 47.0], 0xA0, 0x7F);
        } else if self.state > 50 {
            gfx.tran_center(Some(PARTS), 1.0, sakura, [sx, sy, 41.0, 47.0], 0xA0, 0x7F, GREY, self.spin as f32, None);
        }
        if self.state > 59 {
            // 飛距離 : la distance.
            gfx.tran(PARTS, [316.0, 174.0, 81.0, 30.0], [300.0, 38.0, 81.0, 30.0], 0xA0, 0x7F, Blend::Normal, GREY);
            for (k, n) in [self.distance / 100, (self.distance / 10) % 10, self.distance % 10].into_iter().enumerate() {
                mini::digit(gfx, CH, n, [400.0 + 50.0 * k as f32, 25.0, 30.0, 43.0], 0xA0);
            }
            gfx.tran(PARTS, [292.0, 55.0, 30.0, 27.0], [550.0, 41.0, 30.0, 27.0], 0xA0, 0x7F, Blend::Normal, GREY);
            if self.mode == 1 {
                for &(x, y, on) in &self.marks {
                    if on {
                        cam.draw(gfx, CH, [5.0, 51.0, 72.0, 74.0], [x as f32, y as f32, 40.0, 40.0], 0xA0, 0x7F);
                    }
                }
            }
        }
        // 最高記録 : le meilleur essai.
        gfx.tran(PARTS, [316.0, 205.0, 106.0, 31.0], [380.0, 389.0, 106.0, 31.0], 0xA0, 0x7F, Blend::Normal, GREY);
        for (k, n) in [self.best / 100, (self.best / 10) % 10, self.best % 10].into_iter().enumerate() {
            mini::digit(gfx, CH, n, [500.0 + 35.0 * k as f32, 389.0, 21.0, 30.0], 0xA0);
        }
        gfx.tran(PARTS, [292.0, 55.0, 30.0, 27.0], [605.0, 400.0, 21.0, 19.0], 0xA0, 0x7F, Blend::Normal, GREY);
        if matches!(self.state, 80 | 90) {
            mini::draw_result(gfx, COMMON, self.state == 80, self.banner as f32);
        }
    }

    /// Le ciel (deux rangées qui défilent), le terrain répété et le départ.
    fn draw_scenery(&self, gfx: &mut Gfx, sys: &Sys) {
        let c = (self.clouds * 0.8) as i32 as f32;
        let cl = self.clouds as i32 as f32;
        for y in [self.sky, self.sky - 448.0] {
            gfx.tran(SKY, [512.0 - c, 0.0, c, 448.0], [0.0, y, cl, 448.0], 0x8C, 0x7F, Blend::Normal, GREY);
            gfx.tran(SKY, [0.0, 0.0, 512.0 - c, 448.0], [cl, y, 640.0 - cl, 448.0], 0x8C, 0x7F, Blend::Normal, GREY);
        }
        if self.y > -224.0 {
            let cam = &sys.camera;
            cam.draw(gfx, BG2, [0.0, 0.0, 512.0, 447.0], [self.ground, 224.0, 640.0, 448.0], 0x8C, 0x7F);
            cam.draw(gfx, BG2, [0.0, 0.0, 512.0, 447.0], [self.ground + 639.0, 224.0, 640.0, 448.0], 0x8C, 0x7F);
            if self.x < 900.0 {
                cam.draw(gfx, BG1, [0.0, 0.0, 512.0, 448.0], [self.ground, 224.0, 640.0, 448.0], 0x8C, 0x7F);
            }
        }
    }

    /// Les trois jauges : celle en cours est pleine, les autres pâles.
    fn draw_gauges(&self, gfx: &mut Gfx) {
        let part = |gfx: &mut Gfx, src: [f32; 4], x: f32, y: f32, alpha: i32| gfx.tran(PARTS, src, [x, y, src[2], src[3]], 0x8D, alpha, Blend::Normal, GREY);
        let lit = |state: i32| if self.state == state { 0x7F } else { 0x40 };
        let a = lit(10);
        part(gfx, [0.0, 0.0, 138.0, 216.0], 100.0, 70.0, a);
        let p = self.power as f32;
        gfx.tran(PARTS, [138.0, 121.0, 92.0, -p], [126.0, 260.0, 92.0, -p], 0x8D, a, Blend::Normal, GREY);
        if self.max_flash % 0x1E > 0xF {
            gfx.tran(EFF, [258.0, 262.0, 135.0, 86.0], [100.0, 68.0, 135.0, 86.0], 0x8D, 0x7F, Blend::Normal, GREY);
        }
        let a = lit(20);
        part(gfx, [266.0, 125.0, 50.0, 130.0], 295.0, 150.0, a);
        part(gfx, [211.0, 121.0, 19.0, 8.0], 310.0, (0x104 - self.timing) as f32, a);
        part(gfx, [230.0, 83.0, 119.0, 40.0], 260.0, 105.0, a);
        let a = lit(30);
        part(gfx, [422.0, 145.0, 86.0, 91.0], 479.0, 119.0, a);
        part(gfx, [139.0, 129.0, 127.0, 126.0], 413.0, 150.0, a);
        gfx.tran_center(Some(PARTS), 1.0, [0.0, 216.0, 139.0, 32.0], [490.0, 269.0, 139.0, 32.0], 0x8D, a, GREY, self.angle as f32, Some((420.0, 259.0)));
        part(gfx, [230.0, 47.0, 33.0, 32.0], 414.0, 243.0, a);
        part(gfx, [363.0, 147.0, 53.0, 26.0], 397.0, 120.0, a);
        part(gfx, [322.0, 55.0, 47.0, 23.0], 515.0, 280.0, a);
    }
}

impl Minigame for Bat {
    fn step(&mut self, sys: &mut Sys) -> bool {
        self.frame(sys)
    }

    fn draw(&self, gfx: &mut Gfx, sys: &Sys) {
        self.paint(gfx, sys);
    }
}

// --- Effets (Mini_Bat*EffectMake) -------------------------------------------

fn effect(texture: &'static str, src: [f32; 4], size: (f32, f32), prio: i32, pos: (i32, i32)) -> EffectDef {
    let mut d = EffectDef::ready();
    d.texture = Some(texture);
    d.mode = 1;
    d.src = src;
    d.size = size;
    d.prio = prio;
    d.group = 1;
    d.pos = pos;
    d
}

/// La tête de mort attrapée monte et s'efface.
fn mark_effect(sys: &mut Sys, x: i32, y: i32) {
    let mut d = effect(CH, [5.0, 51.0, 74.0, 72.0], (40.0, 40.0), 0xA3, (x, y - 10));
    d.alpha = 0x80;
    d.keys[0] = Key { dur: 0x14, alpha: 100, rgb: [0x80; 3], scale: 1.0, x, y: y - 0x28, brake: 0.0, spin: 0.0 };
    sys.effects.make(d);
}

/// La poussière d'un rebond, qui glisse avec la vitesse.
fn down_effect(sys: &mut Sys, vx: i32) {
    let mut d = effect(PARTS, [379.0, 404.0, 129.0, 104.0], (200.0, 104.0), 0xA0, (0x140, 0x186));
    d.keys[0] = Key { dur: 10, alpha: 0x7F, rgb: [0x7F; 3], scale: 1.0, x: vx * -10 + 0x140, y: 0x168, brake: 0.0, spin: 0.0 };
    d.keys[1] = Key { dur: 10, alpha: 0, scale: 1.0, x: vx * -0x14 + 0x140, y: 0x186, ..Key::NONE };
    sys.effects.make(d);
}

/// L'éclat sur fond assombri d'un « parfait » (un quart de bat_eff).
fn clear_effect(sys: &mut Sys, u: i32, v: i32) {
    let mut d = effect(EFF, [u as f32, v as f32, 256.0, 256.0], (256.0, 256.0), 0x9F, (0x140, 0xE0));
    d.alpha = 0;
    d.scale = 2.0;
    let k = |dur, alpha| Key { dur, alpha, rgb: [0x7F; 3], scale: 2.0, ..Key::NONE };
    d.keys[0] = k(0x14, 100);
    d.keys[1] = k(5, 0x50);
    d.keys[2] = k(5, 100);
    d.keys[3] = k(5, 0x50);
    d.keys[4] = k(5, 0x7F);
    d.keys[5] = k(0xF, 0);
    sys.effects.make(d);
}

/// Le choc de la batte sur Sakura.
fn hit_effect(sys: &mut Sys) {
    let mut d = effect(PARTS, [382.0, 0.0, 130.0, 145.0], (130.0, 145.0), 0xA0, (0x140, 0x154));
    d.keys[0] = Key { dur: 5, alpha: 0x7F, rgb: [0x7F; 3], ..Key::NONE };
    sys.effects.make(d);
}

/// Le souffle de l'élan.
fn swing_effect(sys: &mut Sys) {
    let mut d = effect(PARTS, [278.0, 0.0, 103.0, 55.0], (115.0, 160.0), 0xA0, (0xA0, 0x1AE));
    d.keys[0] = Key { dur: 5, alpha: 0x7F, rgb: [0x7F; 3], ..Key::NONE };
    sys.effects.make(d);
}
