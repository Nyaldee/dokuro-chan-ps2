//! Ce que partagent les cinq mini-jeux : leurs voix (AD_MiniGameVoicePlay),
//! PUSH BUTTON, les bandeaux de fin, les chiffres de leur planche
//! « challenge ».

use crate::engine::{EffectDef, Key};
use crate::sys::Sys;
use crate::gfx::{Blend, Gfx, GREY};

/// Un mini-jeu en cours : une image de logique (vrai quand il est fini),
/// une image de dessin.
pub trait Minigame {
    fn step(&mut self, sys: &mut Sys) -> bool;
    fn draw(&self, gfx: &mut Gfx, sys: &Sys);
}

/// Les mini-jeux, dans l'ordre du jeu (AD_MiniGameTypeSet) : le titre de
/// leur entrée de menu (0x35d560…, dans « parts »), leur musique (flux) et
/// leur paquet d'images.
pub const GAMES: [(&str, [f32; 4], u32); 5] = [
    ("UNION/MOGURA.UNI", [0.0, 116.0, 283.0, 33.0], 9),
    ("UNION/RUN.UNI", [0.0, 157.0, 255.0, 32.0], 12),
    ("UNION/CARD.UNI", [268.0, 156.0, 201.0, 32.0], 6),
    ("UNION/CATCH.UNI", [0.0, 194.0, 256.0, 32.0], 3),
    ("UNION/BAT.UNI", [0.0, 234.0, 372.0, 32.0], 15),
];

/// Les voix des mini-jeux par personnage (0x35c8b0) : réplique, puis
/// colonne (0 Sakura, 1 Dokuro, 2 à 5 les autres).
const CAST_VOICES: [[u32; 6]; 5] = [
    [66001, 66006, 66011, 66016, 66021, 66026],
    [66002, 66007, 66012, 66017, 66022, 66027],
    [66003, 66008, 66013, 66018, 66023, 66028],
    [66004, 66009, 66014, 66019, 66024, 66029],
    [66005, 66010, 66015, 66020, 66025, 66030],
];

/// Les voix propres à chaque mini-jeu (0x35c930 et 0x35c970) : trois par
/// jeu et par table.
const TYPE_VOICES: [[[u32; 3]; 5]; 2] = [
    [[66034, 66035, 66036], [66040, 66041, 66042], [66046, 66047, 66048], [66052, 66053, 66054], [66058, 66059, 66060]],
    [[66031, 66032, 66033], [66037, 66038, 66039], [66043, 66044, 66045], [66049, 66050, 66051], [66056, 66057, 66055]],
];

/// AD_MiniGameVoicePlay : `who` 2 est la personne choisie ; Dokuro, en
/// ouverture, a la voix propre au mini-jeu.
pub fn cast_voice(who: usize, line: usize, sys: &Sys) -> u32 {
    if who == 2 {
        let col = sys.chara as usize + 1;
        if line == 0 && col == 1 {
            return type_voice(0, 0, sys);
        }
        return CAST_VOICES[line][col];
    }
    CAST_VOICES[line][who]
}

/// AD_MiniGameVoicePlay2 : une voix propre au mini-jeu en cours.
pub fn type_voice(table: usize, line: usize, sys: &Sys) -> u32 {
    TYPE_VOICES[table][sys.kind as usize][line]
}

/// La voix d'ouverture des mini-jeux (état 0) ; avec Dokuro, une seconde
/// (`second`, de la seconde table). Rend vrai quand c'est fini (○ coupe).
pub fn intro(sys: &mut Sys, timer: &mut i32, second: usize) -> bool {
    match *timer {
        0 => {
            sys.voice(cast_voice(2, 0, sys));
            *timer = 10;
        }
        10 => {
            if !sys.voice_playing() {
                *timer = 0x14;
            } else if sys.input.key(3) {
                sys.stop_voice();
                *timer = 0x28;
            }
        }
        0x14 => {
            if sys.chara == 0 {
                sys.voice(type_voice(1, second, sys));
                *timer = 0x1E;
            } else {
                *timer = 0;
                return true;
            }
        }
        0x1E => {
            if !sys.voice_playing() || sys.input.key(3) {
                sys.stop_voice();
                *timer = 0x28;
            }
        }
        0x28 if !sys.voice_playing() => {
            *timer = 0;
            return true;
        }
        _ => {}
    }
    false
}

/// Une des quatre touches de face vient d'être appuyée.
pub fn any_key(sys: &Sys) -> bool {
    (0..4).any(|k| sys.input.key(k))
}

/// La fin : voix (gagné : ligne 1 ; perdu : ligne 2), puis un bouton ou
/// 300 images ; `second` ajoute la voix de la seconde table (et le compte
/// commence alors à 0x28 au lieu de 0x1e). Rend vrai quand c'est fini.
pub fn ending(sys: &mut Sys, timer: &mut i32, won: bool, second: bool) -> bool {
    let line = if won { 1 } else { 2 };
    if *timer < if second { 0x28 } else { 0x1E } {
        match *timer {
            0 => {
                sys.voice(type_voice(0, line, sys));
                *timer = 10;
            }
            10 => {
                if !sys.voice_playing() {
                    *timer = 0x14;
                    if second {
                        sys.voice(type_voice(1, line, sys));
                    }
                } else if sys.input.key(3) {
                    sys.stop_voice();
                    *timer = if second { 0x1E } else { 0x14 };
                }
            }
            0x14 if !second => {
                if !sys.voice_playing() {
                    *timer = 0x1E;
                }
            }
            0x14 => {
                if !sys.voice_playing() {
                    *timer = 0x1E;
                } else if second && sys.input.key(3) {
                    sys.stop_voice();
                    *timer = 0x1E;
                }
            }
            0x1E if !sys.voice_playing() => *timer = 0x28,
            _ => {}
        }
        false
    } else {
        *timer += 1;
        any_key(sys) || *timer > 300
    }
}

/// PUSH BUTTON, qui clignote (20 images sur 40).
pub fn draw_push(gfx: &mut Gfx, common: &'static str, timer: i32) {
    if timer % 0x28 < 0x14 {
        gfx.tran_center(Some(common), 1.0, [3.0, 480.0, 219.0, 32.0], [320.0, 224.0, 219.0, 32.0], 0xA0, 0x7F, GREY, 0.0, None);
    }
}

/// ゲームクリア ! (avec ailes et auréole) ou 失敗 ! sur l'écran assombri,
/// le bandeau à `y`.
pub fn draw_result(gfx: &mut Gfx, common: &'static str, won: bool, y: f32) {
    if won {
        gfx.tran(common, [37.0, 303.0, 443.0, 78.0], [102.0, y, 443.0, 78.0], 0xA0, 0x7F, Blend::Normal, GREY);
        gfx.tran(common, [393.0, 164.0, 105.0, 58.0], [105.0, y + 7.0, -105.0, 58.0], 0xA0, 0x7F, Blend::Normal, GREY);
        gfx.tran(common, [393.0, 164.0, 105.0, 58.0], [535.0, y + 7.0, 105.0, 58.0], 0xA0, 0x7F, Blend::Normal, GREY);
        gfx.tran(common, [395.0, 101.0, 95.0, 54.0], [80.0, y - 27.0, 95.0, 54.0], 0x9F, 0x7F, Blend::Normal, GREY);
    } else {
        gfx.tran(common, [0.0, 194.0, 200.0, 103.0], [220.0, y, 200.0, 103.0], 0xA0, 0x7F, Blend::Normal, GREY);
        gfx.paint([0.0, 0.0, 640.0, 448.0], 0x8C, 0x40, [0x14; 3]);
    }
}

/// « よーい » dans l'image commune de l'attrape et du filet ; la course a le
/// sien.
pub const READY: [f32; 4] = [276.0, 397.0, 237.0, 78.0];

/// Mini_*StEffectMake : « よーい » (`ready`) grandit puis se pose, puis
/// « スタート ! ».
pub fn start_effect(sys: &mut Sys, common: &'static str, ready: [f32; 4]) {
    let mut d = EffectDef::ready();
    d.texture = Some(common);
    d.mode = 2;
    d.delay = 0x1E;
    d.src = ready;
    d.size = (ready[2], ready[3]);
    d.prio = 0xA2;
    d.group = 1;
    d.pos = (0x140, 0xE0);
    d.scale = 0.0;
    d.keys[0] = Key { dur: 0x3C, alpha: 0x7F, rgb: [0x7F; 3], scale: 2.0, ..Key::NONE };
    d.keys[1] = Key { dur: 0x1E, alpha: 0x7F, rgb: [0x7F; 3], scale: 1.0, ..Key::NONE };
    d.keys[2] = Key { dur: 0x3C, alpha: 0x7F, rgb: [0x7F; 3], scale: 1.0, ..Key::NONE };
    sys.effects.make(d);
    let mut d = EffectDef::ready();
    d.texture = Some(common);
    d.delay = 0xB9;
    d.src = [3.0, 105.0, 380.0, 83.0];
    d.size = (380.0, 83.0);
    d.prio = 0xA2;
    d.group = 1;
    d.pos = (0x140, 0xE0);
    d.keys[0] = Key { dur: 0x1E, alpha: 100, rgb: [0x7F; 3], scale: 1.0, ..Key::NONE };
    sys.effects.make(d);
}

/// Un chiffre de la planche « challenge » (31 pixels l'un).
pub fn digit(gfx: &mut Gfx, digits: &'static str, n: i32, dst: [f32; 4], prio: i32) {
    gfx.tran(digits, [(n * 0x1F) as f32, 0.0, 30.0, 43.0], dst, prio, 0x7F, Blend::Normal, GREY);
}

/// Une animation du jeu : rectangles, durées et numéros (tables de
/// l'exécutable), une image en cours et son compte (-1 : arrêtée).
pub struct Anim {
    pub frames: &'static [Frame],
    pub at: usize,
    pub count: i32,
}

/// Une image d'animation : rectangle source, durée (images ; -1, -3, -4 :
/// fin de séquence), numéro.
#[derive(Clone, Copy)]
pub struct Frame {
    pub src: [f32; 4],
    pub dur: i32,
    pub id: i32,
}

pub const fn frame(u: f32, v: f32, w: f32, h: f32, dur: i32, id: i32) -> Frame {
    Frame { src: [u, v, w, h], dur, id }
}

impl Anim {
    pub fn new(frames: &'static [Frame], at: usize) -> Anim {
        Anim { frames, at, count: -1 }
    }

    pub fn now(&self) -> Frame {
        self.frames[self.at]
    }

    /// Lance la séquence qui commence à `at`, pour sa première durée.
    pub fn play(&mut self, at: usize) {
        self.at = at;
        self.count = self.frames[at].dur;
    }

    /// Une image : le compte baisse ; à 0, l'image suivante (son compte est
    /// sa durée) ; à -1, rien ne bouge. Rend la marque de fin atteinte
    /// (durée négative), le jeu décidant où reprendre.
    pub fn step(&mut self) -> Option<i32> {
        if self.count < 1 {
            if self.count == 0 {
                self.at += 1;
                self.count = self.frames[self.at].dur;
            }
        } else {
            self.count -= 1;
        }
        let d = self.frames[self.at].dur;
        (d < 0).then_some(d)
    }
}
