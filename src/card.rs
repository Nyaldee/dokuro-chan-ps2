//! 水着で神経衰弱 (mini_card.cpp) : un memory à deux contre l'adversaire,
//! qui retient les dernières cartes vues. Quinze cartes : sept paires (2
//! points) et un joker. Le joker donne 1 point et montre toutes les cartes
//! un instant ; en challenge, il ne rapporte rien et rebat les cartes
//! cachées. Plus de 7 points gagne (en challenge, la majorité des 14).
//!
//! États (valeurs du jeu) : 0 voix, 1 choix d'une des deux cartes de
//! départ, 2 l'adversaire retourne l'autre, 3 qui commence, 4 la donne, 5
//! annonce du tour, 6 tour du joueur, 7 tour de l'adversaire, 8 les deux
//! cartes retournées, 9 paire prise, 15 joker en challenge, 20 gagné, 30
//! perdu, 40 joker.

use crate::engine::{EffectDef, Key};
use crate::sys::Sys;
use crate::gfx::{Blend, Gfx, GREY};
use crate::mini::{self, Minigame};
use crate::pointer::{self, Zone};

/// Les images (CARD.UNI, dans l'ordre de Mini_CardInit).
pub const TEXTURES: [(&str, u32); 7] = [(BG, 0), (CARD, 1), (CHARA, 2), (PARTS, 3), (COMMON, 4), (CH, 5), (CARD_CH, 6)];

const BG: &str = "card_bg";
const CARD: &str = "card_card";
const CHARA: &str = "card_chara";
const PARTS: &str = "card_pa-tu";
const COMMON: &str = "card_kyoutu";
const CH: &str = "card_ch";
const CARD_CH: &str = "card_card_ch";

/// Les faces (0x35cb50) : sept paires (0 à 6) et le joker (7) dans
/// card_card ; les deux cartes de départ (9 et 10) dans card_pa-tu.
const FACES: [[f32; 4]; 11] = [
    [0.0, 0.0, 112.0, 170.0],
    [226.0, 0.0, 112.0, 170.0],
    [339.0, 0.0, 112.0, 170.0],
    [0.0, 171.0, 112.0, 170.0],
    [113.0, 171.0, 112.0, 170.0],
    [226.0, 171.0, 112.0, 170.0],
    [339.0, 171.0, 112.0, 170.0],
    [113.0, 0.0, 112.0, 170.0],
    [0.0, 341.0, 112.0, 170.0],
    [0.0, 143.0, 88.0, 136.0],
    [89.0, 143.0, 88.0, 136.0],
];
const JOKER: i32 = 7;
/// Le dos d'une carte et le curseur (card_pa-tu).
const BACK: [f32; 4] = [180.0, 140.0, 88.0, 136.0];
const CURSOR: [f32; 4] = [0.0, 0.0, 94.0, 142.0];
/// La carte qui se retourne (0x35cc00) : trois images de 2.
const FLIP: [([f32; 4], i32); 4] = [([96.0, 0.0, 88.0, 136.0], 2), ([185.0, 0.0, 88.0, 136.0], 2), ([274.0, 0.0, 88.0, 136.0], 2), ([-1.0; 4], -1)];

/// Une des deux cartes de départ, entre les deux visages.
fn pre_rect(k: i32) -> [f32; 4] {
    [(k * 0xE2 + 0xA3) as f32, 156.0, 88.0, 136.0]
}

/// Une carte posée sur la table (80 × 120).
fn card_rect(k: usize) -> [f32; 4] {
    let (x, y) = POS[k];
    [x as f32, y as f32, 80.0, 120.0]
}

/// Les places des quinze cartes (0x35cc30), cinq par rangée.
const POS: [(i32, i32); 15] = [
    (100, 22),
    (190, 22),
    (280, 22),
    (370, 22),
    (460, 22),
    (100, 164),
    (190, 164),
    (280, 164),
    (370, 164),
    (460, 164),
    (100, 306),
    (190, 306),
    (280, 306),
    (370, 306),
    (460, 306),
];

/// Les visages (0x35cbb0, card_chara) : le joueur en haut de l'image,
/// l'adversaire dessous ; normal, content, déçu (40 images).
const FACE_NORMAL: f32 = 0.0;
const FACE_HAPPY: f32 = 128.0;
const FACE_SAD: f32 = 256.0;
const FACE_TIME: i32 = 40;

/// Les mots (card_pa-tu) des cartes de départ et les annonces de tour
/// (« あなたの番 », « あいての番 »).
const YOU_TURN: [f32; 4] = [7.0, 340.0, 157.0, 58.0];
const RIVAL_TURN: [f32; 4] = [8.0, 398.0, 307.0, 54.0];
const WORD_A: [f32; 4] = [6.0, 280.0, 109.0, 57.0];
const WORD_B: [f32; 4] = [119.0, 280.0, 109.0, 57.0];

/// À qui le tour (valeurs du jeu).
const PLAYER: i32 = 6;
const RIVAL: i32 = 7;

/// Une carte : face, état (1 cachée, 2 montrée, 3 qui se retourne, -1 prise
/// par le joueur, -2 par l'adversaire), numéro d'origine.
#[derive(Clone, Copy)]
struct Card {
    face: i32,
    state: i32,
    id: i32,
}

pub struct Cards {
    mode: i32,
    cards: [Card; 15],
    /// Les deux cartes de départ : face (9 ou 10), état (1 cachée, 3 qui se
    /// retourne, 0 montrée).
    pre: [(i32, i32); 2],
    cursor: i32,
    /// Les points du joueur (au départ : la carte choisie) et de
    /// l'adversaire.
    score: i32,
    rival_score: i32,
    /// Les cartes retournées ce tour (au départ : les faces tirées), et leur
    /// nombre.
    picked: [i32; 2],
    flips: i32,
    turn: i32,
    /// L'adversaire : 0 pas encore choisi, 1 au hasard, 2 une paire en
    /// mémoire ; ce qu'il a vu, du plus récent au plus ancien.
    chosen: i32,
    memory: [i32; 15],
    /// La carte qui se retourne : image et compte.
    flip_at: usize,
    flip_count: i32,
    /// Les visages du joueur et de l'adversaire : image et compte.
    faces: [(f32, i32); 2],
    timer: i32,
    state: i32,
    banner: i32,
    /// Cartes posées.
    dealt: usize,
    voice_timer: i32,
}

impl Cards {
    /// Mini_CardStart.
    pub fn start(mode: i32, sys: &mut Sys) -> Cards {
        let mut cards: [Card; 15] = std::array::from_fn(|i| Card { face: i as i32 >> 1, state: 1, id: i as i32 });
        for _ in 0..15 {
            let a = (sys.rng.next() % 15) as usize;
            let b = (sys.rng.next() % 15) as usize;
            let f = cards[a].face;
            cards[a].face = cards[b].face;
            cards[b].face = f;
        }
        let pre = if (sys.rng.next() % 10) & 1 == 0 { [(10, 1), (9, 1)] } else { [(9, 1), (10, 1)] };
        Cards {
            mode,
            cards,
            pre,
            cursor: 0,
            score: 0,
            rival_score: 0,
            picked: [0; 2],
            flips: 0,
            turn: 0,
            chosen: 0,
            memory: [-1; 15],
            flip_at: 0,
            flip_count: -1,
            faces: [(FACE_NORMAL, 0); 2],
            timer: 0,
            state: 0,
            banner: 0,
            dealt: 0,
            voice_timer: 0x96,
        }
    }

    fn card(&mut self, k: i32) -> &mut Card {
        &mut self.cards[k as usize]
    }

    /// Les cartes cachées (Mini_CardStateCheck).
    fn hidden(&self) -> Vec<i32> {
        (0..15).filter(|&k| self.cards[k as usize].state == 1).collect()
    }

    /// Mini_CardMemory : l'adversaire retient une carte vue.
    fn remember(&mut self, k: i32) {
        for i in (1..15).rev() {
            if self.memory[i - 1] != -1 {
                self.memory[i] = self.memory[i - 1];
            }
        }
        self.memory[0] = k;
    }

    /// Mini_CardMemCheck : une paire parmi les quatre cartes les plus
    /// récentes en mémoire. Au second retournement, il prend la seconde de
    /// la paire, même si la première n'est pas celle déjà retournée.
    fn recall(&mut self) -> bool {
        const DEPTH: usize = 3;
        for i in 0..DEPTH - 1 {
            for j in i + 1..=DEPTH {
                let (a, b) = (self.memory[i], self.memory[j]);
                if a == -1 || b == -1 {
                    continue;
                }
                let (ca, cb) = (self.cards[a as usize], self.cards[b as usize]);
                if ca.state > 0 && cb.state > 0 && ca.face == cb.face && ca.id != cb.id {
                    if self.flips == 0 {
                        self.picked = [a, b];
                        self.chosen = 2;
                        return true;
                    }
                    if self.flips == 1 {
                        self.picked[1] = b;
                        self.chosen = 2;
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Mini_CardShuffle : les faces des cartes cachées échangées au hasard.
    fn shuffle(&mut self, sys: &mut Sys) {
        let list = self.hidden();
        let n = list.len() as i32;
        for _ in 0..n {
            let a = list[(sys.rng.next() % n) as usize] as usize;
            let b = list[(sys.rng.next() % n) as usize] as usize;
            let f = self.cards[a].face;
            self.cards[a].face = self.cards[b].face;
            self.cards[b].face = f;
        }
    }

    /// Toutes les cartes cachées se montrent.
    fn reveal(&mut self) {
        for c in &mut self.cards {
            if c.state == 1 {
                c.state = 2;
            }
        }
    }

    /// La carte k se retourne.
    fn flip(&mut self, k: i32, sys: &mut Sys) {
        self.card(k).state = 3;
        self.flip_count = FLIP[0].1;
        sys.se.play(0x18, 0);
    }

    /// Le joker retourné par celui dont c'est le tour : un point (le jeu
    /// montre tout), ou en challenge les cartes rebattues.
    fn joker(&mut self) {
        if self.mode == 0 {
            if self.turn == PLAYER {
                self.score += 1;
            } else {
                self.rival_score += 1;
            }
            self.state = 40;
        } else {
            self.flips = 0;
            self.state = 15;
        }
    }

    /// Le tour passe à l'autre, annoncé.
    fn pass(&mut self, sys: &mut Sys) {
        if self.turn == PLAYER {
            word_effect(sys, RIVAL_TURN);
            self.turn = RIVAL;
        } else {
            word_effect(sys, YOU_TURN);
            self.turn = PLAYER;
        }
    }

    /// Les visages réagissent : `player` content ou déçu.
    fn react(&mut self, player_happy: bool) {
        self.faces = if player_happy { [(FACE_HAPPY, FACE_TIME), (FACE_SAD, FACE_TIME)] } else { [(FACE_SAD, FACE_TIME), (FACE_HAPPY, FACE_TIME)] };
    }

    /// État 1 : le joueur prend une des deux cartes de départ (la croix,
    /// puis ○).
    fn choose(&mut self, sys: &mut Sys) {
        if self.timer == 1 {
            intro_effect(sys);
        }
        if self.timer < 0x8C {
            self.timer += 1;
        } else if self.pre[self.cursor as usize].1 == 1 {
            // La souris choisit la carte qu'elle survole.
            let zones: [Zone<i32>; 2] = std::array::from_fn(|k| Zone::new(pre_rect(k as i32), k as i32));
            if let Some(k) = pointer::pick(sys, &zones).hovered.filter(|&k| k != self.cursor) {
                self.cursor = k;
                sys.se.play(0x13, 0);
            } else if sys.input.direct(2, 7, 7, true) {
                if self.cursor != 0 {
                    self.cursor = 0;
                    sys.se.play(0x13, 0);
                }
            } else if sys.input.direct(3, 7, 7, true) && self.cursor != 1 {
                self.cursor = 1;
                sys.se.play(0x13, 0);
            }
            self.score = self.cursor;
            if sys.input.key(3) {
                let c = self.cursor as usize;
                self.picked[0] = self.pre[c].0;
                self.pre[c].1 = 3;
                self.flip_count = FLIP[0].1;
                sys.se.play(0x18, 0);
            }
        }
        if self.pre[self.score as usize].1 == 0 {
            self.timer = 0;
            self.cursor ^= 1;
            self.state = 2;
        }
    }

    /// État 3 : la plus forte des deux cartes commence ; les mots volent à
    /// leur place, puis la table change.
    fn order(&mut self, sys: &mut Sys) {
        self.timer += 1;
        if self.timer == 0x28 {
            // Le joueur (en bas) ou l'adversaire (en haut) commence.
            let (first, second) = if self.picked[1] < self.picked[0] { (75, 355) } else { (355, 75) };
            fly_effect(sys, WORD_A, 0xAE, first, 0, 0xC1);
            sys.se.play(0x19, 0xF);
            fly_effect(sys, if first == 75 { RIVAL_TURN } else { YOU_TURN }, if first == 75 { 0x1A2 } else { 0x158 }, first, 0x1E, 0xA3);
            sys.se.play(0x19, 0x2D);
            fly_effect(sys, WORD_B, 0xAE, second, 0x3C, 0x85);
            sys.se.play(0x19, 0x4B);
            fly_effect(sys, if first == 75 { YOU_TURN } else { RIVAL_TURN }, if first == 75 { 0x158 } else { 0x1A2 }, second, 0x5A, 0x67);
            sys.se.play(0x19, 0x69);
        } else if self.timer == 0xDC {
            change_effect(sys);
        } else if self.timer > 0xFA {
            self.timer = 0;
            self.turn = if self.picked[0] < self.picked[1] { PLAYER } else { RIVAL };
            self.cursor = 0;
            self.score = 0;
            self.state = 4;
        }
    }

    /// État 6 : le joueur déplace le curseur (la croix, en boucle) et
    /// retourne une carte cachée (○).
    fn player(&mut self, sys: &mut Sys) {
        // La souris choisit la carte posée qu'elle survole (le clic, qui
        // vaut ○, la retourne).
        let zones: Vec<_> = (0..self.dealt).map(|k| Zone::new(card_rect(k), k as i32)).collect();
        if self.cards[self.cursor as usize].state != 3
            && let Some(k) = pointer::pick(sys, &zones).hovered.filter(|&k| k != self.cursor)
        {
            self.cursor = k;
            sys.se.play(0x13, 0);
        }
        let k = self.cursor;
        if sys.input.key(3) && self.cards[k as usize].state == 1 {
            self.picked[self.flips as usize] = k;
            if self.cards[k as usize].face == JOKER {
                self.joker();
            } else {
                self.remember(k);
                self.flips += 1;
            }
            self.flip(k, sys);
        }
        if self.cards[self.cursor as usize].state != 3 {
            let moved = if sys.input.direct(0, 7, 7, true) {
                self.cursor -= 5;
                if self.cursor < 0 {
                    self.cursor += 15;
                }
                true
            } else if sys.input.direct(1, 7, 7, true) {
                self.cursor += 5;
                if self.cursor > 14 {
                    self.cursor -= 15;
                }
                true
            } else if sys.input.direct(2, 7, 7, true) {
                self.cursor -= 1;
                if self.cursor % 5 == 4 || self.cursor < 0 {
                    self.cursor += 5;
                }
                true
            } else if sys.input.direct(3, 7, 7, true) {
                self.cursor += 1;
                if self.cursor % 5 == 0 {
                    self.cursor -= 5;
                }
                true
            } else {
                false
            };
            if moved {
                sys.se.play(0x13, 0);
            }
        }
        if self.flips == 2 {
            self.turn = PLAYER;
            self.state = 8;
        }
        if self.voice_timer > 500 && sys.rng.next() % 5 == 0 {
            self.voice_timer = 0;
            let line = if sys.rng.next() & 1 == 0 { 0 } else { 2 };
            sys.voice(mini::type_voice(1, line, sys));
        }
    }

    /// État 7 : l'adversaire choisit (une paire qu'il a en mémoire, sinon
    /// au hasard en retenant la carte), puis son curseur y va pas à pas
    /// toutes les 21 images. Plus de carte cachée : perdu.
    fn rival(&mut self, sys: &mut Sys) {
        if self.flip_count < 0 {
            if self.chosen == 0 && !self.recall() {
                let list = self.hidden();
                let target = match list.len() {
                    0 => {
                        self.state = 30;
                        return;
                    }
                    1 => list[0],
                    n => {
                        let k = list[(sys.rng.next() % n as i32) as usize];
                        self.remember(k);
                        k
                    }
                };
                self.picked[self.flips as usize] = target;
                self.chosen = 1;
            }
            self.timer += 1;
            if self.timer > 0x14 {
                self.timer = 0;
                let d = self.picked[self.flips as usize] - self.cursor;
                self.cursor += match d {
                    5.. => 5,
                    ..=-5 => -5,
                    1..=4 => 1,
                    -4..=-1 => -1,
                    _ => 0,
                };
                sys.se.play(0x13, 0);
                if d == 0 {
                    let k = self.picked[self.flips as usize];
                    self.card(k).state = 3;
                    self.flip_count = FLIP[0].1;
                    if self.cards[k as usize].face == JOKER {
                        self.joker();
                    } else {
                        self.flips += 1;
                    }
                    self.chosen = 0;
                    sys.se.play(0x18, 0);
                }
            }
            if self.flips == 2 {
                self.turn = RIVAL;
                self.state = 8;
            }
        }
        if self.voice_timer > 500 && sys.rng.next() % 5 == 0 {
            self.voice_timer = 0;
            if sys.rng.next() % 3 == 0 {
                sys.voice(mini::cast_voice(0, 2, sys));
            } else {
                let line = (sys.rng.next() & 1) as usize;
                sys.voice(mini::cast_voice(1, line, sys));
            }
        }
    }

    /// État 8 : les deux cartes retournées. Une paire est prise par celui
    /// dont c'est le tour ; sinon elles se cachent et le tour passe.
    fn resolve(&mut self, sys: &mut Sys) {
        self.timer += 1;
        if self.timer == 0x32 {
            self.flips = 0;
            let [a, b] = self.picked;
            if self.cards[a as usize].face == self.cards[b as usize].face {
                let mine = self.turn == PLAYER;
                self.react(mine);
                let taken = if mine { -1 } else { -2 };
                self.card(a).state = taken;
                self.card(b).state = taken;
                if self.voice_timer > 400 {
                    self.voice_timer = 0;
                    let r = sys.rng.next() & 3;
                    let id = match (mine, r) {
                        (true, 0) => mini::cast_voice(0, 0, sys),
                        (true, 1) => mini::cast_voice(0, 3, sys),
                        (true, 2) => mini::cast_voice(1, 2, sys),
                        (true, _) => mini::type_voice(1, 1, sys),
                        (false, 0) => mini::cast_voice(0, 2, sys),
                        (false, 1) => mini::cast_voice(1, 0, sys),
                        (false, 2) => mini::cast_voice(1, 1, sys),
                        (false, _) => mini::cast_voice(1, 3, sys),
                    };
                    sys.voice(id);
                }
                self.timer = 0;
                self.state = 9;
            } else {
                self.card(a).state = 1;
                self.card(b).state = 1;
                sys.se.play(0x14, 0);
                self.react(self.turn != PLAYER);
                self.pass(sys);
                self.cursor = 0;
            }
        } else if self.timer == 0x7D {
            self.timer = 0;
            self.state = self.turn;
        }
    }

    /// État 9 : la paire prise vole vers les points ; le même rejoue.
    fn taken(&mut self, sys: &mut Sys) {
        self.timer += 1;
        if self.timer == 5 {
            let [a, b] = self.picked;
            let (left, right) = if POS[b as usize].0 < POS[a as usize].0 { (POS[b as usize], POS[a as usize]) } else { (POS[a as usize], POS[b as usize]) };
            let face = FACES[self.cards[a as usize].face as usize];
            let face_b = FACES[self.cards[b as usize].face as usize];
            pair_effect(sys, self.mode, left, face, 0xE0, self.turn);
            pair_effect(sys, self.mode, right, face_b, 0x1A0, self.turn);
            sys.se.play(0x16, 0);
            sys.se.play(0x15, 0x4B);
            sys.se.play(0x17, 0xA5);
        } else if self.timer == 200 {
            self.timer = 0;
            if self.turn == PLAYER {
                if self.voice_timer > 500 {
                    self.voice_timer = 0;
                    sys.voice(mini::type_voice(1, 1, sys));
                }
                self.score += 2;
            } else {
                self.rival_score += 2;
            }
            self.state = self.turn;
            let won = if self.mode == 0 { self.score > 7 } else { self.score + self.rival_score > 13 && self.rival_score < self.score };
            if won {
                self.reveal();
                self.state = 20;
            }
            if self.rival_score > 7 {
                self.reveal();
                self.state = 30;
            }
        }
    }

    /// État 40 : le joker, pris par celui qui l'a retourné, montre toutes
    /// les cartes ; seule reste visible la première du tour.
    fn joker_show(&mut self, sys: &mut Sys) {
        self.timer += 1;
        match self.timer {
            2 => sys.se.play(0x1D, 0),
            0x32 => {
                let k = self.cursor;
                self.card(k).state = if self.turn == RIVAL { -2 } else { -1 };
            }
            0x37 => self.reveal(),
            0x9B => {
                self.timer = 0;
                let keep = self.cards[self.picked[0] as usize].id;
                for c in &mut self.cards {
                    if c.state == 2 && c.id != keep {
                        c.state = 1;
                    }
                }
                self.state = self.turn;
            }
            _ => {}
        }
    }

    /// État 15 : le joker en challenge rebat les cartes cachées ; le tour
    /// passe.
    fn joker_shuffle(&mut self, sys: &mut Sys) {
        self.timer += 1;
        match self.timer {
            2 => {
                sys.se.play(0x1D, 0);
                text_effect(sys, CARD_CH, [128.0, 354.0, 338.0, 97.0]);
            }
            0x46 => {
                for k in self.picked {
                    if self.cards[k as usize].state == 2 {
                        self.card(k).state = 1;
                    }
                }
                self.shuffle(sys);
                self.memory = [-1; 15];
            }
            0x96 => self.pass(sys),
            0xE1 => {
                self.timer = 0;
                self.cursor = 0;
                self.state = self.turn;
            }
            _ => {}
        }
    }

    /// Mini_CardSpAnim et Mini_CardAnim : les visages reviennent à la
    /// normale ; la carte qui se retourne finit montrée.
    fn animate(&mut self) {
        if self.state < 10 {
            for f in &mut self.faces {
                if f.1 >= 1 {
                    f.1 -= 1;
                } else if f.1 == 0 {
                    f.0 = FACE_NORMAL;
                }
            }
        }
        if self.flip_count < 1 {
            if self.flip_count == 0 {
                self.flip_at += 1;
                self.flip_count = FLIP[self.flip_at].1;
            }
        } else {
            self.flip_count -= 1;
        }
        if FLIP[self.flip_at].1 < 0 {
            self.flip_at = 0;
            if self.state < 3 {
                self.pre[self.cursor as usize].1 = 0;
            } else {
                let k = self.cursor;
                self.card(k).state = 2;
            }
        }
    }
}

impl Minigame for Cards {
    /// Mini_CardProc.
    fn step(&mut self, sys: &mut Sys) -> bool {
        match self.state {
            40 => self.joker_show(sys),
            20 | 30 => {
                let won = self.state == 20;
                if self.banner < 0xB6 {
                    self.banner += 2;
                }
                if self.timer == 0 {
                    self.react(won);
                }
                if mini::ending(sys, &mut self.timer, won, false) {
                    sys.points = if won { self.score * 2 } else { 0 };
                    sys.won = won;
                    return true;
                }
            }
            15 => self.joker_shuffle(sys),
            9 => self.taken(sys),
            8 => self.resolve(sys),
            7 => self.rival(sys),
            6 => self.player(sys),
            5 => {
                self.timer += 1;
                if self.timer == 1 {
                    word_effect(sys, if self.turn == RIVAL { RIVAL_TURN } else { YOU_TURN });
                } else if self.timer == 0x6E {
                    self.timer = 0;
                    self.state = self.turn;
                }
            }
            4 => {
                self.timer += 1;
                if self.timer == 0x1E {
                    self.timer = 0x14;
                    self.dealt += 1;
                    sys.se.play(0x24, 0);
                    if self.dealt == 15 {
                        self.timer = 0x1F;
                    }
                } else if self.timer == 0x32 {
                    self.timer = 0;
                    self.state = 5;
                }
            }
            3 => self.order(sys),
            2 => {
                let c = self.cursor as usize;
                if self.pre[c].1 != 3 {
                    self.timer += 1;
                    if self.timer > 0x1D {
                        self.timer = 0;
                        self.picked[1] = self.pre[c].0;
                        self.pre[c].1 = 3;
                        self.flip_count = FLIP[0].1;
                        sys.se.play(0x18, 0);
                    }
                }
                if self.score != self.cursor && self.pre[c].1 == 0 {
                    sys.effects.delete(0, Some(1));
                    self.state = 3;
                }
            }
            1 => self.choose(sys),
            0 if mini::intro(sys, &mut self.timer, 1) => self.state = 1,
            _ => {}
        }
        self.animate();
        if self.state > 5 && self.state < 8 {
            self.voice_timer += 1;
            if sys.chara != 0 && self.voice_timer > 500 && sys.rng.next() & 7 == 0 {
                self.voice_timer = 0;
                let line = (sys.rng.next() % 3 + 1) as usize;
                sys.voice(mini::cast_voice(2, line, sys));
            }
        }
        false
    }

    /// Mini_CardTran.
    fn draw(&self, gfx: &mut Gfx, _sys: &Sys) {
        let tran = |gfx: &mut Gfx, tex: &'static str, src: [f32; 4], dst: [f32; 4], prio: i32| gfx.tran(tex, src, dst, prio, 0x7F, Blend::Normal, GREY);
        // Le curseur du tour de l'adversaire est rouge.
        let red = [0xFF, 0x20, 0];
        tran(gfx, BG, [0.0, 0.0, 512.0, 448.0], [0.0, 0.0, 640.0, 448.0], 0x8C);
        if self.state < 4 {
            // Les deux cartes de départ, entre les deux visages.
            for (k, &(face, state)) in self.pre.iter().enumerate() {
                let dst = pre_rect(k as i32);
                match state {
                    1 => tran(gfx, PARTS, BACK, dst, 0x8D),
                    0 => tran(gfx, PARTS, FACES[face as usize], dst, 0x8D),
                    3 => tran(gfx, PARTS, FLIP[self.flip_at].0, dst, 0x8D),
                    _ => {}
                }
            }
            let x = (self.cursor * 0xE2 + 0xA0) as f32;
            let rgb = if self.state < 2 { GREY } else { red };
            gfx.tran(PARTS, CURSOR, [x, 153.0, 93.0, 141.0], 0x8D, 0x7F, Blend::Normal, rgb);
            gfx.paint([0.0, 0.0, 640.0, 100.0], 0x8C, 0x19, [0x14; 3]);
            gfx.paint([0.0, 348.0, 640.0, 100.0], 0x8C, 0x19, [0x14; 3]);
            let rival = if self.state < 2 { [270.0, 21.0, 100.0, 100.0] } else { [(self.cursor * 0xE2 + 0x9F) as f32, 49.0, 100.0, 100.0] };
            tran(gfx, CHARA, [0.0, 128.0, 128.0, 128.0], rival, 0xA0);
            tran(gfx, CHARA, [0.0, 0.0, 128.0, 128.0], [(self.score * 0xE2 + 0x9F) as f32, 300.0, 100.0, 100.0], 0xA0);
            return;
        }
        if self.state > 4 {
            // Les bords de la table, les visages, les points.
            tran(gfx, PARTS, [414.0, 0.0, 98.0, 448.0], [0.0, 0.0, 98.0, 448.0], 0x8C);
            tran(gfx, PARTS, [414.0, 0.0, 98.0, 448.0], [542.0, 448.0, 98.0, -448.0], 0x8C);
            tran(gfx, CHARA, [self.faces[1].0, 128.0, 128.0, 128.0], [0.0, 21.0, 100.0, 100.0], 0xA0);
            tran(gfx, CHARA, [self.faces[0].0, 0.0, 128.0, 128.0], [540.0, 318.0, 100.0, 100.0], 0xA0);
            tran(gfx, PARTS, [272.0, 247.0, 82.0, 97.0], [10.0, 121.0, 81.0, 93.0], 0xA0);
            mini::digit(gfx, CH, self.rival_score / 10, [15.0, 153.0, 30.0, 43.0], 0xA0);
            mini::digit(gfx, CH, self.rival_score % 10, [55.0, 153.0, 30.0, 43.0], 0xA0);
            tran(gfx, PARTS, [273.0, 140.0, 81.0, 93.0], [548.0, 232.0, 82.0, 97.0], 0xA0);
            mini::digit(gfx, CH, self.score / 10, [553.0, 238.0, 30.0, 43.0], 0xA0);
            mini::digit(gfx, CH, self.score % 10, [593.0, 238.0, 30.0, 43.0], 0xA0);
            if self.mode == 1 {
                gfx.tran(CH, [5.0, 51.0, 72.0, 74.0], [568.0, 22.0, 40.0, 40.0], 0xA3, 0x7F, Blend::Normal, GREY);
                mini::digit(gfx, CH, self.score * 2 / 10, [553.0, 72.0, 30.0, 43.0], 0xA3);
                mini::digit(gfx, CH, self.score * 2 % 10, [593.0, 72.0, 30.0, 43.0], 0xA3);
            }
        }
        let faces = if self.mode == 1 { CARD_CH } else { CARD };
        for (k, c) in self.cards.iter().enumerate().take(self.dealt) {
            let dst = card_rect(k);
            let face = FACES[c.face as usize];
            match c.state {
                1 => tran(gfx, PARTS, BACK, dst, 0x8D),
                // Prises : pâlies, bleues pour le joueur, rouges pour
                // l'adversaire.
                -1 => gfx.tran(faces, face, dst, 0x8D, 0x46, Blend::Normal, [0x1B, 0x1B, 0x7F]),
                -2 => gfx.tran(faces, face, dst, 0x8D, 0x46, Blend::Normal, [0x7F, 0x1B, 0x1B]),
                2 => tran(gfx, faces, face, dst, 0x8D),
                3 => tran(gfx, PARTS, FLIP[self.flip_at].0, dst, 0x8D),
                _ => {}
            }
        }
        let (x, y) = POS[self.cursor as usize];
        let at = [(x - 2) as f32, (y - 2) as f32, 84.0, 124.0];
        match self.state {
            6 => tran(gfx, PARTS, CURSOR, at, 0x8D),
            7 => gfx.tran(PARTS, CURSOR, at, 0x8D, 0x7F, Blend::Normal, red),
            _ => {}
        }
        let b = self.banner as f32;
        match self.state {
            20 => {
                tran(gfx, COMMON, [350.0, 6.0, 162.0, 83.0], [238.0, b, 164.0, 83.0], 0xA0);
                tran(gfx, COMMON, [393.0, 164.0, 105.0, 58.0], [205.0, b + 13.0, -105.0, 58.0], 0xA0);
                tran(gfx, COMMON, [393.0, 164.0, 105.0, 58.0], [435.0, b + 13.0, 105.0, 58.0], 0xA0);
                tran(gfx, COMMON, [395.0, 101.0, 95.0, 54.0], [280.0, b - 27.0, 95.0, 54.0], 0x9F);
            }
            30 => {
                tran(gfx, COMMON, [203.0, 194.0, 168.0, 103.0], [236.0, b, 168.0, 103.0], 0xA0);
                gfx.paint([0.0, 0.0, 640.0, 448.0], 0x8D, 0x40, [0x14; 3]);
            }
            _ => {}
        }
    }
}

/// Mini_CardMojiEffectMake (sans argument) : la consigne du départ apparaît
/// au milieu, reste, s'efface.
fn intro_effect(sys: &mut Sys) {
    let mut d = EffectDef::ready();
    d.texture = Some(PARTS);
    d.delay = 0x1E;
    d.src = [6.0, 454.0, 263.0, 57.0];
    d.size = (263.0, 57.0);
    d.prio = 0xA1;
    d.group = 1;
    d.alpha = 0;
    d.pos = (0x140, 0xE0);
    d.keys[0] = Key { dur: 0x14, alpha: 0x7F, rgb: [0x7F; 3], scale: 1.0, x: 0x140, y: 0xE0, brake: 0.0, spin: 0.0 };
    d.keys[1] = Key { dur: 0x50, alpha: 0x7F, ..Key::NONE };
    d.keys[2] = Key { dur: 10, alpha: 0, ..Key::NONE };
    sys.effects.make(d);
}

/// Mini_CardMojiEffectMake (image, rectangle) : un texte au milieu,
/// annoncé après 40 images, qui reste puis s'efface.
fn text_effect(sys: &mut Sys, texture: &'static str, src: [f32; 4]) {
    let mut d = EffectDef::ready();
    d.texture = Some(texture);
    d.delay = 0x28;
    d.src = src;
    d.size = (src[2], src[3]);
    d.prio = 0xA1;
    d.group = 1;
    d.alpha = 0;
    d.pos = (0x140, 0xE0);
    d.keys[0] = Key { dur: 10, alpha: 0x7F, rgb: [0x7F; 3], scale: 1.0, x: 0x140, y: 0xE0, brake: 0.0, spin: 0.0 };
    d.keys[1] = Key { dur: 0x14, alpha: 0x7F, rgb: [0x7F; 3], scale: 1.0, ..Key::NONE };
    d.keys[2] = Key { dur: 10, alpha: 0, rgb: [0x7F; 3], scale: 1.0, ..Key::NONE };
    sys.effects.make(d);
}

/// L'annonce d'un tour.
fn word_effect(sys: &mut Sys, src: [f32; 4]) {
    text_effect(sys, PARTS, src);
}

/// Mini_CardMojiEffectMake (complet) : un mot part du milieu, cinq fois
/// plus grand, et se pose en (x, y) pour `stay` images.
fn fly_effect(sys: &mut Sys, src: [f32; 4], x: i32, y: i32, delay: i32, stay: i32) {
    let mut d = EffectDef::ready();
    d.texture = Some(PARTS);
    d.delay = delay;
    d.src = src;
    d.size = (src[2], src[3]);
    d.prio = 0xA1;
    d.group = 1;
    d.alpha = 0x80;
    d.scale = 5.0;
    d.pos = (0x140, 0xE0);
    d.keys[0] = Key { dur: 0xF, alpha: 0x7F, rgb: [0x7F; 3], scale: 1.0, x, y, brake: 0.0, spin: 0.0 };
    d.keys[1] = Key { dur: stay, alpha: 0x7F, rgb: [0x7F; 3], scale: 1.0, ..Key::NONE };
    sys.effects.make(d);
}

/// L'image du décor se répète au-delà de ses 512 pixels de large (voir
/// change_effect) : le rendu la prend en mode répété.
pub const REPEATED: &str = BG;

/// Mini_CardChangeEffect : le décor grandit depuis le centre (de 0,1 à 1 en
/// 30 images) en faisant un tour complet (12° par image, champ +0x14 de la
/// première image-clé), puis reste 15 images.
///
/// Le jeu y lit 640 pixels de large (src 0, 0, 640, 448) dans card_bg, qui
/// n'en a que 512, à raison d'un pixel par pixel d'écran, alors que le
/// décor fixe étire ses 512 pixels sur 640. Au-delà de 512, le GS répète
/// l'image (mode REPEAT de son registre CLAMP) : les 128 pixels de droite
/// reprennent le début du damier, qui se raccorde sans couture. Une image
/// bloquée à son bord (CLAMP) étirerait au contraire sa dernière colonne
/// en bandes visibles ; une capture de la console tournant le mini-jeu ne
/// montre aucune bande pendant ce zoom.
fn change_effect(sys: &mut Sys) {
    let mut d = EffectDef::ready();
    d.texture = Some(BG);
    d.src = [0.0, 0.0, 640.0, 448.0];
    d.size = (640.0, 448.0);
    d.prio = 0xA2;
    d.group = 1;
    d.alpha = 0x80;
    d.scale = 0.1;
    d.pos = (0x140, 0xE0);
    d.keys[0] = Key { dur: 0x1E, alpha: 0x7F, rgb: [0x7F; 3], scale: 1.0, spin: 12.0, ..Key::NONE };
    d.keys[1] = Key { dur: 0xF, alpha: 0x7F, ..Key::NONE };
    sys.effects.make(d);
}

/// Mini_CardAtariEffect : une carte de la paire grandit au milieu (à
/// gauche ou à droite), puis file vers les points de celui qui l'a prise.
fn pair_effect(sys: &mut Sys, mode: i32, (x, y): (i32, i32), src: [f32; 4], to_x: i32, turn: i32) {
    let mut d = EffectDef::ready();
    d.texture = Some(if mode == 1 { CARD_CH } else { CARD });
    d.src = src;
    d.size = (177.0, 273.0);
    d.prio = 0xA2;
    d.group = 1;
    d.alpha = 0x80;
    d.scale = 0.6;
    d.pos = (x + 0x30, y + 0x48);
    let (px, py) = if turn == PLAYER { (0x238, 0x12A) } else { (0x37, 0x95) };
    d.keys[0] = Key { dur: 0x28, alpha: 0x7F, rgb: [0x7F; 3], scale: 1.0, x: to_x, y: 0xDF, brake: 0.0, spin: 0.0 };
    d.keys[1] = Key { dur: 100, alpha: 0x7F, rgb: [0x7F; 3], scale: 1.0, ..Key::NONE };
    d.keys[2] = Key { dur: 0x14, alpha: 0x40, rgb: [0x7F; 3], scale: 0.1, x: px, y: py, brake: 0.0, spin: 0.0 };
    sys.effects.make(d);
}
