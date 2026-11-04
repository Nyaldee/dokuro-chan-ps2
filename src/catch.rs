//! 桜くん私物キャッチ (mini_catch.cpp) : Dokuro jette par la fenêtre les
//! affaires de Sakura, qui les rattrape en bas ; les bombes explosent, au sol
//! comme dans les bras. Trois vies, 50 secondes à tenir. En challenge, des
//! têtes de mort sur les objets et des points par objet rattrapé.
//!
//! États (valeurs du jeu) : 0 voix, 5 PUSH BUTTON, 10 « よーい スタート »,
//! 20 partie, 30 perdu, 40 gagné.

use crate::engine::{EffectDef, Key};
use crate::sys::Sys;
use crate::gfx::{Blend, Gfx, GREY};
use crate::mini::{self, frame, Anim, Frame, Minigame};

/// Les images (CATCH.UNI, dans l'ordre de Mini_CatchInit).
pub const TEXTURES: [(&str, u32); 6] = [(BG, 0), (CHARA, 1), (PARTS, 2), (COMMON, 3), (CH, 4), (PARTS_CH, 5)];

const BG: &str = "catch_bg";
const CHARA: &str = "catch_chara";
const PARTS: &str = "catch_pa-tu";
const COMMON: &str = "catch_kyoutu";
const CH: &str = "catch_ch";
const PARTS_CH: &str = "catch_pa-tu_ch";

/// Sakura (0x35cc70) : marche (0), blessé (5), perdu (7), content (9),
/// danse de victoire en boucle (11), noirci par une bombe (14).
const SAKURA: [Frame; 16] = [
    frame(0.0, 0.0, 128.0, 128.0, 12, 0),
    frame(128.0, 0.0, 128.0, 128.0, 12, 1),
    frame(256.0, 0.0, 128.0, 128.0, 12, 2),
    frame(128.0, 0.0, 128.0, 128.0, 12, 3),
    frame(-1.0, -1.0, -1.0, -1.0, -1, 4),
    frame(0.0, 128.0, 128.0, 128.0, 20, 5),
    frame(-1.0, -1.0, -1.0, -1.0, -1, 6),
    frame(128.0, 128.0, 128.0, 128.0, 32767, 7),
    frame(-1.0, -1.0, -1.0, -1.0, -1, 8),
    frame(256.0, 128.0, 128.0, 128.0, 20, 9),
    frame(-1.0, -1.0, -1.0, -1.0, -1, 10),
    frame(256.0, 128.0, 128.0, 128.0, 10, 11),
    frame(384.0, 128.0, 128.0, 128.0, 10, 12),
    frame(-1.0, -1.0, -1.0, -1.0, -3, 13),
    frame(384.0, 0.0, 128.0, 128.0, 30, 14),
    frame(-1.0, -1.0, -1.0, -1.0, -1, 15),
];
const SAKURA_HURT: usize = 5;
const SAKURA_LOST: usize = 7;
const SAKURA_HAPPY: usize = 9;
const SAKURA_DANCE: usize = 11;
const SAKURA_BLACK: usize = 14;

/// Dokuro à la fenêtre (0x35cd30) : le lancer (0), sa victoire (5), sa
/// défaite (8), en boucle.
const DOKURO: [Frame; 11] = [
    frame(0.0, 384.0, 128.0, 128.0, 10, 0),
    frame(0.0, 256.0, 128.0, 128.0, 8, 1),
    frame(128.0, 256.0, 128.0, 128.0, 8, 2),
    frame(256.0, 256.0, 128.0, 128.0, 8, 3),
    frame(-1.0, -1.0, -1.0, -1.0, -1, 4),
    frame(0.0, 384.0, 128.0, 128.0, 20, 5),
    frame(128.0, 384.0, 128.0, 128.0, 20, 6),
    frame(-1.0, -1.0, -1.0, -1.0, -3, 7),
    frame(256.0, 384.0, 128.0, 128.0, 20, 8),
    frame(384.0, 384.0, 128.0, 128.0, 20, 9),
    frame(-1.0, -1.0, -1.0, -1.0, -4, 10),
];
const DOKURO_WINS: usize = 5;
const DOKURO_LOSES: usize = 8;

/// Les objets (0x35cdc0) dans catch_pa-tu : 0 et 1 des bombes, 2 à 15 les
/// affaires de Sakura (14 et 15 avec leur bulle), 16 éclat (objet cassé),
/// 17 fumée (bombe rattrapée), 18 explosion (bombe au sol).
const ITEMS: [[f32; 4]; 19] = [
    [160.0, 80.0, 80.0, 80.0],
    [160.0, 80.0, 80.0, 80.0],
    [334.0, 183.0, 83.0, 60.0],
    [417.0, 183.0, 83.0, 60.0],
    [95.0, 165.0, 69.0, 89.0],
    [184.0, 166.0, 73.0, 84.0],
    [261.0, 170.0, 65.0, 78.0],
    [89.0, 277.0, 99.0, 50.0],
    [205.0, 256.0, 66.0, 86.0],
    [288.0, 254.0, 66.0, 87.0],
    [160.0, 0.0, 80.0, 80.0],
    [240.0, 0.0, 80.0, 80.0],
    [320.0, 0.0, 80.0, 80.0],
    [400.0, 0.0, 80.0, 80.0],
    [13.0, 263.0, 59.0, 80.0],
    [380.0, 267.0, 103.0, 83.0],
    [240.0, 80.0, 80.0, 80.0],
    [320.0, 80.0, 80.0, 80.0],
    [400.0, 80.0, 80.0, 80.0],
];
const BROKEN: i32 = 0x10;
const SMOKE: i32 = 0x11;
const BLAST: i32 = 0x12;

/// Les bulles « なぜハラ?! » et « なぜカツオ?! » (0x365750, 0x365758).
const WHY_HARA: [f32; 4] = [44.0, 373.0, 74.0, 83.0];
const WHY_KATSUO: [f32; 4] = [151.0, 373.0, 74.0, 83.0];

/// Un objet lancé : état (0 libre, 1 lancé, 2 en l'air, 4 et plus : réglé,
/// compté jusqu'à 30), sorte, position, vitesse.
#[derive(Clone, Copy, Default)]
struct Item {
    state: i32,
    kind: i32,
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
}

pub struct Catch {
    mode: i32,
    x: f32,
    vx: f32,
    sakura: Anim,
    dokuro: Anim,
    seconds: i32,
    timer: i32,
    lives: i32,
    banner: i32,
    state: i32,
    items: [Item; 5],
    throw_timer: i32,
    points: i32,
    voice_timer: i32,
}

impl Catch {
    /// Mini_CatchStart.
    pub fn start(mode: i32) -> Catch {
        Catch {
            mode,
            x: 256.0,
            vx: 0.0,
            sakura: Anim::new(&SAKURA, 0),
            dokuro: Anim::new(&DOKURO, 0),
            seconds: 0x32,
            timer: 0,
            lives: 3,
            banner: 0,
            state: 0,
            items: [Item::default(); 5],
            throw_timer: 0,
            points: 0,
            voice_timer: 0x96,
        }
    }

    fn parts(&self) -> &'static str {
        if self.mode == 1 { PARTS_CH } else { PARTS }
    }

    /// Mini_CatchItemCheck : le premier objet libre.
    fn free_item(&self) -> Option<usize> {
        self.items.iter().position(|i| i.state == 0)
    }

    /// Mini_CatchSakuraMove : la croix pousse Sakura (après 3 images
    /// tenues), qui glisse ; une explosion proche la repousse.
    fn sakura_move(&mut self, sys: &Sys) {
        if self.vx != 0.0 {
            self.vx *= 0.96;
        }
        // La souris pousse Sakura vers elle, comme une direction tenue,
        // tant qu'elle est à plus de 16 pixels de son milieu.
        let toward = sys.mouse.map_or(0.0, |(x, _)| x - (self.x + 64.0));
        if sys.input.direct(2, 3, 1, false) || toward < -16.0 {
            self.vx -= 0.5;
        }
        if sys.input.direct(3, 3, 1, false) || toward > 16.0 {
            self.vx += 0.5;
        }
        for i in &self.items[..2] {
            if i.state > 0 && i.kind == BLAST {
                let d = (i.x + 40.0) - (self.x + 64.0);
                if d < 0.0 {
                    self.vx += (d + 512.0) * 0.005;
                } else if d > 0.0 {
                    self.vx += (d - 512.0) * 0.005;
                }
            }
        }
        self.x += self.vx;
        if self.x < 0.0 {
            self.x = 0.0;
            self.vx = 0.0;
        } else if self.x > 512.0 {
            self.x = 512.0;
            self.vx = 0.0;
        }
    }

    /// Mini_CatchItemProc : les objets lancés, qui tombent, se cassent,
    /// explosent ou sont rattrapés.
    fn items_step(&mut self, sys: &mut Sys) {
        for k in 0..self.items.len() {
            let free_second = self.items[1].state < 1;
            let it = &mut self.items[k];
            match it.state {
                1 => {
                    if self.mode == 0 {
                        it.vx = (sys.rng.next() % 0x28 - 0x10) as f32 * 0.1;
                        it.vy = if free_second { ((sys.rng.next() & 3) - 2) as f32 } else { ((sys.rng.next() & 1) - 5) as f32 };
                    } else {
                        it.vx = (sys.rng.next() % 0x30 - 0x14) as f32 * 0.1;
                        if k == 1 {
                            it.vy = (sys.rng.next() % 3 - 4) as f32;
                        } else {
                            sys.rng.next();
                            it.vy = -1.0;
                        }
                    }
                    it.x = if it.vx >= 0.0 { 270.0 } else { 200.0 };
                    it.y = 100.0;
                    it.state = 2;
                    sys.se.play(0x10, 0);
                    sys.se.play(0x0D, 10);
                }
                2 => {
                    it.x += it.vx;
                    it.y += it.vy;
                    it.vy += 0.12;
                    if it.y > 370.0 {
                        if it.kind < 2 {
                            // Une bombe au sol explose.
                            it.state = 4;
                            it.kind = BLAST;
                            let (x, y, kind) = (it.x as i32, it.y as i32, it.kind);
                            blast_effect(sys, self.mode, x, y, kind);
                            sys.se.play(0x0E, 0);
                        } else if it.kind == 0x15 {
                            it.state = 0;
                        } else {
                            // Un objet se casse : une vie de moins.
                            self.lives -= 1;
                            it.state = 4;
                            it.kind = BROKEN;
                            self.sakura.play(SAKURA_HURT);
                            sys.se.play(0x0F, 0);
                        }
                    }
                }
                s if s > 3 => {
                    it.state += 1;
                    if it.kind < 0x10 {
                        it.x = self.x + 20.0;
                        it.y = 340.0;
                    }
                    if it.state == 12 {
                        let kind = it.kind;
                        if kind < 2 {
                            // Une bombe rattrapée : Sakura noirci.
                            it.state = 0;
                            it.x = self.x + 20.0;
                            it.kind = SMOKE;
                            let (x, y) = (it.x as i32, it.y as i32);
                            blast_effect(sys, self.mode, x, y, SMOKE);
                            self.lives -= 1;
                            self.sakura.play(SAKURA_BLACK);
                            sys.se.play(0x0E, 0);
                        } else if kind > 0xD {
                            if kind == 0xE || kind == 0xF {
                                if self.mode == 1 {
                                    self.points += 1;
                                }
                                bubble_effect(sys, self.mode, if kind == 0xE { WHY_HARA } else { WHY_KATSUO }, self.x, self.vx);
                                sys.se.play(0x11, 0);
                            } else if kind == BLAST {
                                it.state = 0;
                            }
                        } else {
                            if self.mode == 1 {
                                self.points += 1;
                            }
                            it.state = 0;
                            it.x = self.x + 50.0;
                            let (x, y) = (it.x as i32, it.y as i32);
                            caught_effect(sys, self.mode, ITEMS[kind as usize], x, y);
                            self.sakura.play(SAKURA_HAPPY);
                            sys.se.play(0x11, 0);
                        }
                    } else if it.state == 0x1E {
                        it.state = 0;
                    }
                }
                _ => {}
            }
        }
    }

    /// Mini_CatchItemHit : un objet qui tombe dans les bras de Sakura.
    fn items_hit(&mut self) {
        for it in &mut self.items {
            let (x, y) = (it.x + 40.0, it.y + 40.0);
            if it.state == 2 && self.x < x && y > 340.0 && x < self.x + 128.0 && y < 468.0 {
                it.state = 4;
            }
        }
    }

    /// Une voix de Dokuro au lancer, parfois (après 500 images, s'il reste
    /// plus de 5 secondes).
    fn throw_voice(&mut self, sys: &mut Sys) {
        if self.voice_timer > 500 && self.seconds > 5 && sys.rng.next() & 7 == 0 {
            self.voice_timer = 0;
            let line = [0, 1, 3, 4][(sys.rng.next() & 3) as usize];
            sys.voice(mini::cast_voice(1, line, sys));
        }
    }

    fn play(&mut self, sys: &mut Sys) {
        self.timer += 1;
        if self.timer == 0x3C {
            self.timer = 0;
            self.seconds -= 1;
            if self.seconds == 0 {
                sys.se.play(7, 0x1E);
                self.state = 40;
            }
        }
        self.sakura_move(sys);
        if self.vx < -0.3 || self.vx > 0.3 {
            if self.sakura.count < 0 {
                self.sakura.count = self.sakura.now().dur;
            }
        } else {
            self.vx = 0.0;
        }
        self.throw_timer += 1;
        let chance = if self.mode == 0 { 0x5A } else { 0x28 };
        if sys.rng.next() % chance == 1 && self.dokuro.count == -1 {
            self.throw_timer = 0;
            if self.free_item().is_some() {
                self.dokuro.count = self.dokuro.now().dur;
                self.throw_voice(sys);
            }
        }
        if self.throw_timer > 0xF0 {
            self.throw_timer = 0;
            self.dokuro.count = self.dokuro.now().dur;
        }
        if self.dokuro.now().id == 3
            && self.dokuro.count == 8
            && let Some(k) = self.free_item()
        {
            let kinds = if self.mode == 0 { 0x10 } else { 0x13 };
            let mut kind = sys.rng.next() % kinds;
            if self.mode == 1 && kind > 0xF {
                kind = 0;
            }
            self.items[k] = Item { state: 1, kind, ..self.items[k] };
        }
        self.items_step(sys);
        self.items_hit();
        if self.lives == 0 {
            self.timer = 0;
            sys.se.play(8, 10);
            self.state = 30;
        }
        self.voice_timer += 1;
        if self.voice_timer > 500 && self.seconds > 5 && sys.rng.next() & 7 == 0 {
            self.voice_timer = 0;
            if sys.rng.next() & 1 == 0 {
                let line = (sys.rng.next() % 3 + 1) as usize;
                sys.voice(mini::cast_voice(2, line, sys));
            } else {
                let line = (sys.rng.next() % 3) as usize;
                sys.voice(mini::type_voice(1, line, sys));
            }
        }
    }
}

impl Minigame for Catch {
    /// Mini_CatchProc.
    fn step(&mut self, sys: &mut Sys) -> bool {
        match self.state {
            40 | 30 => {
                let won = self.state == 40;
                if mini::ending(sys, &mut self.timer, won, false) {
                    sys.points = if won { self.points } else { 0 };
                    sys.won = won;
                    return true;
                }
                if self.banner < 0xB6 {
                    self.banner += 2;
                }
                if won {
                    if self.sakura.count == -1 {
                        self.sakura.play(SAKURA_DANCE);
                    }
                    if self.dokuro.count == -1 {
                        self.dokuro.play(DOKURO_LOSES);
                    }
                } else {
                    if self.sakura.now().id != SAKURA_BLACK as i32 && self.sakura.count == -1 {
                        self.sakura.play(SAKURA_LOST);
                    }
                    if self.sakura.now().id == SAKURA_BLACK as i32 {
                        self.sakura.count = 0x7FFF;
                    }
                    if self.dokuro.count == -1 {
                        self.dokuro.play(DOKURO_WINS);
                    }
                }
            }
            20 => self.play(sys),
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
                    mini::start_effect(sys, COMMON, mini::READY);
                    sys.se.play(6, 0xBE);
                    self.state = 10;
                }
            }
            0 if mini::intro(sys, &mut self.timer, 1) => self.state = 5,
            _ => {}
        }
        // Mini_CatchSakuraAnim et Mini_CatchDokuroAnim.
        match self.sakura.step() {
            Some(-1) => self.sakura.at = 0,
            Some(-3) => self.sakura.play(SAKURA_DANCE),
            _ => {}
        }
        match self.dokuro.step() {
            Some(-1) => self.dokuro.at = 0,
            Some(-3) => self.dokuro.play(DOKURO_WINS),
            Some(-4) => self.dokuro.play(DOKURO_LOSES),
            _ => {}
        }
        false
    }

    /// Mini_CatchTran.
    fn draw(&self, gfx: &mut Gfx, _sys: &Sys) {
        let tran = |gfx: &mut Gfx, tex: &'static str, src: [f32; 4], dst: [f32; 4]| gfx.tran(tex, src, dst, 0xA0, 0x7F, Blend::Normal, GREY);
        gfx.tran(BG, [0.0, 0.0, 512.0, 448.0], [0.0, 0.0, 640.0, 448.0], 0x8C, 0x7F, Blend::Normal, GREY);
        tran(gfx, CHARA, self.sakura.now().src, [self.x as i32 as f32, 320.0, 128.0, 128.0]);
        tran(gfx, CHARA, self.dokuro.now().src, [215.0, 91.0, 80.0, 80.0]);
        let parts = self.parts();
        if self.state == 20 {
            for it in self.items.iter().filter(|i| i.state != 0) {
                let src = ITEMS[it.kind as usize];
                let dst = [it.x as i32 as f32, it.y as i32 as f32, (src[2] * 0.8).trunc(), (src[3] * 0.8).trunc()];
                gfx.tran(parts, src, dst, 0xA1, 0x7F, Blend::Normal, GREY);
            }
        }
        // Le temps qui reste.
        tran(gfx, COMMON, [43.0, 44.0, 111.0, 58.0], [50.0, 5.0, 111.0, 58.0]);
        mini::digit(gfx, CH, self.seconds / 10, [150.0, 20.0, 30.0, 43.0], 0xA0);
        mini::digit(gfx, CH, self.seconds % 10, [190.0, 20.0, 30.0, 43.0], 0xA0);
        tran(gfx, COMMON, [0.0, 76.0, 21.0, 26.0], [230.0, 37.0, 21.0, 26.0]);
        // Les vies : le visage de Sakura et ses cœurs.
        tran(gfx, parts, [241.0, 354.0, 258.0, 66.0], [365.0, 5.0, 258.0, 66.0]);
        let face = match self.lives {
            3 => 246.0,
            2 => 313.0,
            _ => 382.0,
        };
        tran(gfx, parts, [face, 428.0, 58.0, 58.0], [369.0, 9.0, 58.0, 58.0]);
        for k in 0..self.lives {
            tran(gfx, parts, [447.0, 427.0, 52.0, 44.0], [(k * 0x36 + 0x1BC) as f32, 15.0, 52.0, 44.0]);
        }
        if self.mode == 1 {
            tran(gfx, CH, [5.0, 51.0, 72.0, 74.0], [450.0, 400.0, 40.0, 40.0]);
            mini::digit(gfx, CH, self.points / 10, [510.0, 400.0, 30.0, 43.0], 0xA0);
            mini::digit(gfx, CH, self.points % 10, [550.0, 400.0, 30.0, 43.0], 0xA0);
        }
        match self.state {
            5 => mini::draw_push(gfx, COMMON, self.timer),
            30 | 40 => mini::draw_result(gfx, COMMON, self.state == 40, self.banner as f32),
            _ => {}
        }
    }
}

fn parts_of(mode: i32) -> &'static str {
    if mode == 1 { PARTS_CH } else { PARTS }
}

/// Mini_CatchBomEffectMake : l'explosion ou la fumée grandit et s'efface.
fn blast_effect(sys: &mut Sys, mode: i32, x: i32, y: i32, kind: i32) {
    let mut d = EffectDef::ready();
    d.texture = Some(parts_of(mode));
    d.mode = 0;
    d.delay = 5;
    d.src = ITEMS[kind as usize];
    d.size = (180.0, 80.0);
    d.prio = 0xA2;
    d.group = 1;
    d.alpha = 0x80;
    d.pos = (x + 0x28, y + 0x28);
    d.keys[0] = Key { dur: 10, alpha: 0, rgb: [0xFF; 3], scale: 2.5, x: 0, y: 0, brake: 0.0, spin: 0.0 };
    sys.effects.make(d);
}

/// Mini_CatchItemEffectMake : l'objet rattrapé monte et rapetisse.
fn caught_effect(sys: &mut Sys, mode: i32, src: [f32; 4], x: i32, y: i32) {
    let mut d = EffectDef::ready();
    d.texture = Some(parts_of(mode));
    d.src = src;
    d.size = (80.0, 80.0);
    d.prio = 0xA0;
    d.group = 1;
    d.alpha = 0x80;
    d.pos = (x + 0x28, y + 0x14);
    d.keys[0] = Key { dur: 0xF, alpha: 0x80, rgb: [0x80; 3], scale: 0.7, x, y: y - 0x28, brake: 0.0, spin: 0.0 };
    sys.effects.make(d);
}

/// Mini_CatchItemEffectMake (bulle) : « なぜハラ ?! » ou « なぜカツオ ?! » au-dessus
/// de Sakura, qui suit son élan.
fn bubble_effect(sys: &mut Sys, mode: i32, src: [f32; 4], x: f32, vx: f32) {
    let mut d = EffectDef::ready();
    d.texture = Some(parts_of(mode));
    d.src = src;
    d.size = (80.0, 80.0);
    d.prio = 0xA0;
    d.group = 1;
    d.alpha = 0x80;
    d.pos = ((x + 120.0) as i32, 0x122);
    d.keys[0] = Key { dur: 0x14, alpha: 0x80, rgb: [0x80; 3], scale: 1.0, x: (x + 120.0 + vx) as i32, ..Key::NONE };
    sys.effects.make(d);
}
