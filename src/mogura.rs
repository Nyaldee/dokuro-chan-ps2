//! ドクロちゃん捕縛アミ (mini_mogura.cpp) : Dokuro sort d'un des neuf
//! buissons, Sakura l'attrape au filet. Chaque prise fait baisser sa jauge
//! de 2, chaque coup dans le vide la remonte de 1 ; à zéro, c'est gagné.
//! 40 secondes. En challenge, une jauge plus longue, Dokuro plus rapide et
//! des points par prise.
//!
//! États (valeurs du jeu) : 0 voix, 5 PUSH BUTTON, 10 « よーい スタート »,
//! 20 partie, 30 gagné, 40 perdu.

use crate::engine::Button;
use crate::sys::Sys;
use crate::gfx::{Blend, Gfx, GREY};
use crate::mini::{self, frame, Frame, Minigame};
use crate::pointer::{self, Zone};

/// Les images (MOGURA.UNI, dans l'ordre de Mini_MoguraInit).
pub const TEXTURES: [(&str, u32); 6] = [(CHARA, 0), (BG, 1), (PARTS, 2), (COMMON, 3), (CH, 4), (CHARA_CH, 5)];

const CHARA: &str = "mogura_chara";
const BG: &str = "mogura_bg";
const PARTS: &str = "mogura_pa-tu";
const COMMON: &str = "mogura_kyoutu";
const CH: &str = "mogura_ch";
const CHARA_CH: &str = "mogura_chara_ch";

/// Les neuf buissons (0x35ceb0) : où se place le filet, de gauche à droite
/// puis de haut en bas.
const HOLES: [(i32, i32); 9] = [(100, 112), (256, 112), (412, 112), (100, 212), (256, 212), (412, 212), (100, 312), (256, 312), (412, 312)];
/// Le buisson visé sans direction : celui du milieu.
const CENTER: usize = 4;

/// Ce que vise la souris pour un buisson : sa largeur (celle de Dokuro), et
/// de la tête de Dokuro au pied du buisson (une rangée de 100 de haut).
fn hole_rect(k: usize) -> [f32; 4] {
    let (x, y) = HOLES[k];
    [x as f32, (y - 20) as f32, 128.0, 100.0]
}

/// Le filet (0x35cef0) : angle (degrés), durée, numéro. Il s'abat (3 : le
/// coup) puis se relève ; après une prise, il reste baissé (8).
const NET: [(i32, i32, i32); 13] = [(30, 0, 0), (20, 1, 1), (10, 1, 2), (0, 12, 3), (10, 1, 4), (20, 1, 5), (30, 0, 6), (-1, -1, 7), (0, 10, 8), (10, 1, 9), (20, 1, 10), (30, 6, 11), (-1, -1, 12)];
const NET_CAUGHT: usize = 8;

/// Dokuro dans un buisson (0x35cf40) : cachée (0), elle sort et rentre (1
/// à 5 ; on l'attrape de 2 à 4), touchée (7), prise, en boucle (12).
const DOKURO: [Frame; 17] = [
    frame(0.0, 0.0, 0.0, 0.0, 20, 0),
    frame(0.0, 0.0, 128.0, 128.0, 5, 1),
    frame(128.0, 0.0, 128.0, 128.0, 10, 2),
    frame(256.0, 0.0, 128.0, 128.0, 40, 3),
    frame(128.0, 0.0, 128.0, 128.0, 10, 4),
    frame(0.0, 0.0, 128.0, 128.0, 5, 5),
    frame(-1.0, -1.0, -1.0, -1.0, -1, 6),
    frame(0.0, 128.0, 128.0, 128.0, 5, 7),
    frame(128.0, 128.0, 128.0, 128.0, 5, 8),
    frame(256.0, 128.0, 128.0, 128.0, 5, 9),
    frame(384.0, 128.0, 128.0, 128.0, 15, 10),
    frame(-1.0, -1.0, -1.0, -1.0, -1, 11),
    frame(0.0, 256.0, 128.0, 128.0, 10, 12),
    frame(128.0, 256.0, 128.0, 128.0, 10, 13),
    frame(256.0, 256.0, 128.0, 128.0, 10, 14),
    frame(128.0, 256.0, 128.0, 128.0, 10, 15),
    frame(-1.0, -1.0, -1.0, -1.0, -3, 16),
];
const DOKURO_HIT: usize = 7;
const DOKURO_CAUGHT: usize = 12;

/// Le buisson (0x35d010) : au repos ou qui bruisse (0), qui s'agite sans
/// fin autour de Dokuro prise (4).
const BUSH: [Frame; 9] = [
    frame(0.0, 342.0, 123.0, 87.0, 3, 0),
    frame(0.0, 0.0, 123.0, 87.0, 3, 1),
    frame(123.0, 342.0, 123.0, 87.0, 3, 2),
    frame(-1.0, -1.0, -1.0, -1.0, -1, 3),
    frame(0.0, 342.0, 123.0, 87.0, 8, 4),
    frame(0.0, 0.0, 123.0, 87.0, 8, 5),
    frame(123.0, 342.0, 123.0, 87.0, 8, 6),
    frame(0.0, 0.0, 123.0, 87.0, 8, 7),
    frame(-1.0, -1.0, -1.0, -1.0, -2, 8),
];
const BUSH_SHAKE: usize = 4;

/// Une animation de buisson ou de Dokuro : image en cours et compte (-1 :
/// arrêtée ; -2, -3 : marques que pose le jeu).
#[derive(Clone, Copy)]
struct Anim {
    at: usize,
    count: i32,
}

const STILL: Anim = Anim { at: 0, count: -1 };

pub struct Mogura {
    mode: i32,
    /// Le buisson visé.
    hole: usize,
    net: usize,
    net_count: i32,
    dokuro: [Anim; 9],
    bush: [Anim; 9],
    seconds: i32,
    timer: i32,
    /// La jauge de Dokuro (30, 40 en challenge) : à zéro, elle est prise.
    gauge: i32,
    banner: i32,
    state: i32,
    /// Images depuis la dernière sortie de Dokuro.
    since: i32,
    points: i32,
    voice_timer: i32,
}

impl Mogura {
    /// Mini_MoguraStart.
    pub fn start(mode: i32) -> Mogura {
        Mogura {
            mode,
            hole: CENTER,
            net: 0,
            net_count: -1,
            dokuro: [STILL; 9],
            bush: [STILL; 9],
            seconds: 0x28,
            timer: 0,
            gauge: if mode == 1 { 0x28 } else { 0x1E },
            banner: 0,
            state: 0,
            since: 0,
            points: 0,
            voice_timer: 0x96,
        }
    }

    /// Mini_MoguraAmiAnim : 2 au coup, 0 revenu au repos.
    fn net_step(&mut self) -> i32 {
        if self.net_count < 1 {
            if self.net_count == 0 {
                self.net += 1;
                self.net_count = NET[self.net].1;
            }
        } else {
            self.net_count -= 1;
        }
        if NET[self.net].2 == 3 {
            2
        } else if NET[self.net].1 < 0 {
            self.net = 0;
            0
        } else {
            1
        }
    }

    /// Mini_MoguraDokuroAnim : en challenge, Dokuro ne reste dehors que 15
    /// images.
    fn dokuro_step(&mut self, k: usize) {
        let a = &mut self.dokuro[k];
        if a.count < 1 {
            if a.count == 0 {
                a.at += 1;
                a.count = DOKURO[a.at].dur;
                if self.mode == 1 && DOKURO[a.at].id == 3 {
                    a.count = 0xF;
                }
            }
        } else {
            a.count -= 1;
        }
        if DOKURO[a.at].dur == -1 {
            a.at = 0;
        } else if a.count == -2 {
            a.at = DOKURO_HIT;
            a.count = DOKURO[a.at].dur;
        } else if a.count == -3 {
            a.at = DOKURO_CAUGHT;
            a.count = DOKURO[a.at].dur;
        }
    }

    /// Mini_MoguraKusaiAnim.
    fn bush_step(&mut self, k: usize) {
        let a = &mut self.bush[k];
        if a.count < 1 {
            if a.count == 0 {
                a.at += 1;
                a.count = BUSH[a.at].dur;
            }
        } else {
            a.count -= 1;
        }
        match BUSH[a.at].dur {
            -1 => a.at = 0,
            -2 => {
                a.at = BUSH_SHAKE;
                a.count = BUSH[a.at].dur;
            }
            _ => {}
        }
    }

    /// Dokuro sort du buisson k, qui bruisse.
    fn pop(&mut self, k: usize) {
        self.since = 0;
        self.dokuro[k].count = DOKURO[0].dur;
        self.bush[k].count = BUSH[0].dur;
    }

    /// Le filet, au repos : la croix (tenue plus de 3 images), L1 R1 L2 R2
    /// ou le stick en diagonale visent un buisson ; ○ l'abat.
    fn aim(&mut self, sys: &mut Sys) {
        let zones: [Zone<usize>; 9] = std::array::from_fn(|k| Zone::new(hole_rect(k), k));
        let hovered = pointer::pick(sys, &zones).hovered;
        let input = &sys.input;
        self.hole = if input.direct(0, 3, 1, false) {
            1
        } else if input.direct(1, 3, 1, false) {
            7
        } else if input.direct(2, 3, 1, false) {
            3
        } else if input.direct(3, 3, 1, false) {
            5
        } else if input.held(Button::L1 as usize, 3) {
            0
        } else if input.held(Button::L2 as usize, 3) {
            6
        } else if input.held(Button::R1 as usize, 3) {
            2
        } else if input.held(Button::R2 as usize, 3) {
            8
        } else if let Some(k) = hovered {
            // La souris vise le buisson qu'elle survole (le clic, qui vaut
            // ○, l'abat).
            k
        } else {
            CENTER
        };
        if sys.input.key(3) {
            self.net_count = NET[0].1;
            self.bush[self.hole].count = BUSH[0].dur;
            sys.se.play(0xB, 0);
            if self.voice_timer > 400 && self.seconds > 5 && self.gauge > 3 && sys.rng.next() % 3 == 0 {
                self.voice_timer = 0;
                // Le jeu tire de 0 à 5 mais n'a de voix que pour 3 et 4,
                // jamais atteints : de 0 à 1, rien.
                if sys.rng.next() % 6 >= 2 {
                    let line = (sys.rng.next() % 3) as usize;
                    sys.voice(mini::type_voice(1, line, sys));
                }
            }
        }
    }

    /// Le coup du filet : Dokuro dehors (de 2 à 4) est touchée, sinon la
    /// jauge remonte (une fois, à la fin du coup).
    fn strike(&mut self, sys: &mut Sys) {
        let k = self.hole;
        let d = self.dokuro[k];
        let id = DOKURO[d.at].id;
        if d.count < 0 || !(2..=4).contains(&id) {
            if self.net_count == 0 {
                let max = if self.mode == 1 { 0x28 } else { 0x1E };
                if self.gauge < max {
                    self.gauge += 1;
                }
            }
            return;
        }
        self.gauge -= 2;
        if self.mode == 1 {
            self.points += 1;
        }
        self.dokuro[k].count = -2;
        if self.gauge < 1 {
            // Prise : Dokuro se débat dans le buisson.
            self.dokuro[k].count = -3;
            self.bush[k] = Anim { at: BUSH_SHAKE, count: BUSH[BUSH_SHAKE].dur };
            sys.se.play(0xC, 0);
            sys.se.play(7, 0x1E);
            self.timer = 0;
            self.state = 30;
        } else {
            self.net = NET_CAUGHT;
            self.net_count = NET[NET_CAUGHT].1;
            sys.se.play(9, 5);
            if self.voice_timer > 400 && self.seconds > 5 && self.gauge > 3 && sys.rng.next() % 3 == 0 {
                self.voice_timer = 0;
                let id = if sys.rng.next() & 1 == 0 { mini::cast_voice(1, 1, sys) } else { mini::cast_voice(0, 3, sys) };
                sys.voice(id);
            }
        }
    }

    fn play(&mut self, sys: &mut Sys) {
        self.timer += 1;
        self.voice_timer += 1;
        if self.timer == 0x3C {
            self.timer = 0;
            self.seconds -= 1;
            if self.seconds == 0 {
                sys.se.play(8, 10);
                self.state = 40;
            }
        }
        if self.net_count < 0 {
            self.aim(sys);
        }
        if self.net_step() == 2 {
            self.strike(sys);
        }
        self.since += 1;
        for k in 0..9 {
            if self.gauge < 1 && k != self.hole {
                self.dokuro[k].count = -1;
            }
            if self.mode == 0 {
                if sys.rng.next() % 800 == 100 && self.dokuro[k].count == -1 {
                    self.pop(k);
                    sys.se.play(0xB, 0);
                    sys.se.play(10, 0x14);
                }
            } else if self.dokuro[k].count == -1 {
                // Plus le temps passe, plus elle sort souvent ; les tirages
                // suivants ne regardent plus si elle est déjà dehors.
                if self.seconds > 0x22 && sys.rng.next() % 500 == 0x14D {
                    self.pop(k);
                    sys.se.play(10, 5);
                }
                if self.seconds > 0xE && self.seconds < 0x23 && sys.rng.next() % 300 == 0xDE {
                    self.pop(k);
                    sys.se.play(10, 5);
                }
                if self.seconds < 0xF && sys.rng.next() % 200 == 0x6F {
                    self.pop(k);
                    sys.se.play(10, 5);
                }
            }
            // Un buisson qui bruisse pour rien.
            if sys.rng.next() & 0x7FF == 800 && self.dokuro[k].count == -1 {
                let other = (sys.rng.next() % 9) as usize;
                self.bush[other].count = BUSH[0].dur;
                sys.se.play(0xB, 0);
            }
            self.dokuro_step(k);
            self.bush_step(k);
        }
        if self.since > 0x5A {
            // Une sortie forcée, même si elle est déjà dehors là.
            let k = (sys.rng.next() % 9) as usize;
            self.pop(k);
            sys.se.play(0xB, 0);
            sys.se.play(10, 0x14);
        }
        self.voice_timer += 1;
        if sys.chara != 0 && self.voice_timer > 600 && sys.rng.next() % 10 == 0 {
            self.voice_timer = 0;
            let line = (sys.rng.next() % 3 + 1) as usize;
            sys.voice(mini::cast_voice(2, line, sys));
        }
    }
}

impl Minigame for Mogura {
    /// Mini_MoguraProc.
    fn step(&mut self, sys: &mut Sys) -> bool {
        match self.state {
            30 | 40 => {
                let won = self.state == 30;
                if mini::ending(sys, &mut self.timer, won, false) {
                    sys.points = if won { self.points } else { 0 };
                    sys.won = won;
                    return true;
                }
                if won {
                    self.dokuro_step(self.hole);
                    self.bush_step(self.hole);
                }
                if self.banner < 0xB6 {
                    self.banner += 2;
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
        false
    }

    /// Mini_MoguraTran.
    fn draw(&self, gfx: &mut Gfx, _sys: &Sys) {
        let tran = |gfx: &mut Gfx, tex: &'static str, src: [f32; 4], dst: [f32; 4], prio: i32| gfx.tran(tex, src, dst, prio, 0x7F, Blend::Normal, GREY);
        tran(gfx, BG, [0.0, 0.0, 512.0, 448.0], [0.0, 0.0, 640.0, 448.0], 0x8C);
        for k in 0..9 {
            let row = (k / 3) as i32;
            let src = BUSH[self.bush[k].at].src;
            tran(gfx, PARTS, src, [((k % 3) as i32 * 0x9C + 100) as f32, (row * 100 + 0x92) as f32, 128.0, 88.0], row * 2 + 0x8D);
        }
        // Le temps qui reste.
        tran(gfx, COMMON, [43.0, 44.0, 111.0, 58.0], [100.0, 10.0, 111.0, 58.0], 0xA0);
        mini::digit(gfx, CH, self.seconds / 10, [200.0, 25.0, 30.0, 43.0], 0xA0);
        mini::digit(gfx, CH, self.seconds % 10, [240.0, 25.0, 30.0, 43.0], 0xA0);
        tran(gfx, COMMON, [0.0, 76.0, 21.0, 26.0], [280.0, 42.0, 21.0, 26.0], 0xA0);
        // La jauge de Dokuro.
        tran(gfx, COMMON, [0.0, 44.0, 43.0, 32.0], [350.0, 36.0, 43.0, 32.0], 0xA0);
        let frame_w = if self.mode == 1 { 164.0 } else { 124.0 };
        tran(gfx, COMMON, [154.0, 44.0, 131.0, 22.0], [400.0, 40.0, frame_w, 27.0], 0xA0);
        tran(gfx, COMMON, [154.0, 66.0, 131.0, 22.0], [402.0, 42.0, (self.gauge * 4) as f32, 23.0], 0xA0);
        if self.mode == 1 {
            gfx.tran(CH, [5.0, 51.0, 72.0, 74.0], [230.0, 400.0, 40.0, 40.0], 0xA3, 0x7F, Blend::Normal, GREY);
            mini::digit(gfx, CH, self.points / 10, [290.0, 400.0, 30.0, 43.0], 0xA3);
            mini::digit(gfx, CH, self.points % 10, [340.0, 400.0, 30.0, 43.0], 0xA3);
        }
        let chara = if self.mode == 1 { CHARA_CH } else { CHARA };
        for (k, d) in self.dokuro.iter().enumerate() {
            if d.count >= 0 {
                let f = DOKURO[d.at];
                let (x, y) = HOLES[k];
                tran(gfx, chara, f.src, [x as f32, (y - 0x41) as f32, 128.0, f.src[3]], (k / 3) as i32 * 2 + 0x8C);
            }
        }
        // Le filet, tourné autour de son manche ; Dokuro prise, il clignote
        // avec elle dedans.
        let caught = self.state == 30 && self.timer % 0x1E > 0xE;
        let src = [0.0, if caught { 215.0 } else { 88.0 }, 273.0, 127.0];
        let (x, y) = HOLES[self.hole];
        let pivot = ((x + 0x80) as f32, (y + 0x5A) as f32);
        gfx.tran_center(Some(PARTS), 1.0, src, [(x + 0x88) as f32, y as f32, 273.0, 127.0], 0x9F, 0x7F, GREY, NET[self.net].0 as f32, Some(pivot));
        match self.state {
            5 => mini::draw_push(gfx, COMMON, self.timer),
            30 | 40 => mini::draw_result(gfx, COMMON, self.state == 30, self.banner as f32),
            _ => {}
        }
    }
}
