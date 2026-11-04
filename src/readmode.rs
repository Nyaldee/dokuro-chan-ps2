//! L'avance rapide et la lecture automatique de l'histoire
//! (AD_ReadModeProc, AD_ReadModeCheck, AD_ReadModeStop, AD_ReadModeTran).
//! △ lance l'avance rapide (ou l'arrête), START la lecture automatique,
//! R1 passe de l'avance des pages lues (既読) à l'avance forcée (強制) ;
//! ✕, □ ou L1 arrêtent. En haut à droite, un signe les montre. Voir
//! docs/moteur.md.

use crate::engine::{Button, Input};
use crate::gfx::{Gfx, GREY};
use crate::options::Settings;

/// L'image du signe.
const SIGN: &str = "win2";

/// Le mode (sRam01da7534).
pub const NONE: i16 = 0;
pub const SKIP: i16 = 1;
pub const SKIP_ENDING: i16 = 2;
pub const AUTO: i16 = 3;
pub const AUTO_ENDING: i16 = 4;
/// R1 vient de changer d'avance : le signe la montre un instant.
pub const SHOWN: i16 = 5;

#[derive(Default)]
pub struct ReadMode {
    pub mode: i16,
    /// L'opacité du signe (sRam01da7536), son clignotement (gpffff8a90), la
    /// course des huit points autour (gpffff8a94) et leur écart
    /// (gpffff8a98).
    alpha: i16,
    blink: i16,
    spin: i16,
    radius: i16,
    /// Ctrl était tenu à l'image précédente.
    held: bool,
}

impl ReadMode {
    /// AD_ReadModeProc, à chaque image de l'histoire. `active` : l'histoire
    /// est sans sous-état et hors des mini-jeux et du résultat ; `reading` :
    /// une page attend. Au clavier, comme dans les visual novels sur PC,
    /// Ctrl tenu avance (le temps qu'il est tenu) et A vaut START. Vrai
    /// quand START arrête la lecture automatique sur une page : ○ attend
    /// alors 10 images.
    pub fn step(&mut self, input: &Input, settings: &mut Settings, active: bool, reading: bool) -> bool {
        let stop = input.key(2) || input.key(1) || input.key(4);
        let ctrl = input.held(Button::Slide as usize, 0);
        let released = std::mem::replace(&mut self.held, ctrl) && !ctrl;
        if !active {
            return false;
        }
        let mut wait = false;
        let idle = matches!(self.mode, NONE | SHOWN | SKIP_ENDING | AUTO_ENDING);
        if input.key(0) || ctrl && idle {
            if idle {
                self.mode = SKIP;
            } else if self.mode == SKIP && !ctrl {
                self.mode = SKIP_ENDING;
            }
        } else if self.mode == SKIP && (stop || released) {
            self.mode = SKIP_ENDING;
        }
        if input.key(8) || input.tapped(Button::Auto) {
            if matches!(self.mode, NONE | SHOWN | SKIP_ENDING | AUTO_ENDING) {
                self.mode = AUTO;
            } else if self.mode == AUTO {
                self.mode = AUTO_ENDING;
                wait = reading;
            }
        } else if self.mode == AUTO && stop {
            self.mode = AUTO_ENDING;
        }
        if input.key(5) {
            settings.skip_mode = (settings.skip_mode + 1) & 1;
            if matches!(self.mode, SHOWN | NONE | AUTO_ENDING) {
                self.alpha = 0x7F;
                self.mode = SHOWN;
            }
        }
        if matches!(self.mode, SKIP | AUTO) {
            self.alpha = (self.alpha + 0x14).min(0x7F);
        } else {
            if self.alpha > 0 {
                self.alpha -= if self.mode == SHOWN { 10 } else { 0x14 };
            }
            if self.alpha < 1 {
                self.alpha = 0;
                self.mode = NONE;
            }
        }
        wait
    }

    /// AD_ReadModeCheck : le mode, sauf qu'une page pas lue ne s'avance pas
    /// dans l'avance des pages lues (0).
    pub fn check(&self, settings: &Settings, read: bool) -> i16 {
        if self.mode == SKIP && settings.skip_mode == 0 { i16::from(read) } else { self.mode }
    }

    /// AD_ReadModeStop : l'avance (ou la lecture automatique) s'arrête à ce
    /// cas si les options le disent ; −1 : toujours.
    pub fn stop(&mut self, settings: &Settings, case: i32) {
        if case < 0 || settings.skip_stops(self.mode != SKIP, case) {
            self.mode = NONE;
            self.alpha = 0;
        }
    }

    /// AD_ReadModeTranCut : un sous-état s'ouvre, le signe disparaît.
    pub fn cut(&mut self) {
        self.alpha = 0;
    }

    /// AD_ReadModeTran, la part qui avance à chaque image : le signe
    /// clignote, les points tournent, plus vite et plus loin avec la
    /// vitesse de l'avance (ou de la lecture automatique).
    pub fn tick(&mut self, settings: &Settings) {
        if self.skipping() {
            let speed = settings.skip_speed(false) as i16;
            self.blink = (self.blink + 2) % 0xB4;
            self.spin = (self.spin + speed + 1) % 0x168;
            chase(&mut self.radius, speed << 2);
        } else {
            let speed = settings.skip_speed(true) as i16;
            self.blink = (self.blink + 1) % 0xB4;
            self.spin = (self.spin + (speed >> 1) + 1) % 0x168;
            chase(&mut self.radius, speed << 2);
        }
    }

    fn skipping(&self) -> bool {
        matches!(self.mode, SKIP | SKIP_ENDING | SHOWN)
    }

    /// Le signe : « SKIP » (les pages lues, ou forcée) ou « AUTO », qui
    /// clignote, en 0x221, 0x28 ; huit points de plus en plus grands et
    /// opaques tournent autour.
    pub fn draw(&self, gfx: &mut Gfx, settings: &Settings) {
        if self.alpha <= 0 {
            return;
        }
        // Les étiquettes de « win2 », de 119 × 21 : « オート » en ligne 2,
        // « 強制スキップ » en 0x1a, « 既読スキップ » en 0x32. Le jeu découpe
        // « オート » depuis la ligne 0 : deux lignes vides en haut, et le bord
        // du bas coupé. Bug corrigé, voir le README.
        let v = if !self.skipping() {
            2.0
        } else if settings.skip_mode == 0 {
            50.0
        } else {
            26.0
        };
        let sin = |deg: i16| (f32::from(deg)).to_radians().sin();
        let cos = |deg: i16| (f32::from(deg)).to_radians().cos();
        let alpha = f32::from(self.alpha);
        gfx.tran_center(Some(SIGN), 1.0, [4.0, v, 119.0, 21.0], [545.0, 40.0, 119.0, 21.0], 0xA0, (alpha * sin(self.blink)) as i32, GREY, 0.0, None);
        let r = f32::from(self.radius + 10);
        for k in 0..8i16 {
            let deg = self.spin + k * 20;
            let rest = f32::from(8 - k);
            let at = [(r * sin(deg) + 545.0).trunc(), (r * cos(deg) + 40.0).trunc(), 23.0, 31.0];
            gfx.tran_center(Some(SIGN), 1.0 - rest / 10.0, [4.0, 76.0, 23.0, 31.0], at, 0x9F, (alpha * (0.6 - rest / 20.0)) as i32, GREY, 0.0, None);
        }
    }
}

/// EtNumChase(n, but, 1) : un pas vers le but.
fn chase(n: &mut i16, goal: i16) {
    *n += (goal - *n).signum();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn press(b: Button) -> Input {
        let mut input = Input::default();
        let mut down = [false; crate::engine::BUTTONS];
        down[b as usize] = true;
        input.update(down);
        input
    }

    /// △ lance l'avance, △ l'arrête : le signe s'efface puis le mode
    /// revient à 0 ; l'avance des pages lues passe les seules pages lues ;
    /// un choix l'arrête (réglages du départ).
    #[test]
    fn skip_then_stop() {
        let mut s = Settings::default();
        let mut m = ReadMode::default();
        m.step(&press(Button::Triangle), &mut s, true, true);
        assert_eq!(m.mode, SKIP);
        assert_eq!(m.check(&s, false), 0);
        assert_eq!(m.check(&s, true), 1);
        for _ in 0..10 {
            m.step(&Input::default(), &mut s, true, true);
        }
        m.step(&press(Button::Triangle), &mut s, true, true);
        assert_eq!(m.mode, SKIP_ENDING);
        for _ in 0..10 {
            m.step(&Input::default(), &mut s, true, true);
        }
        assert_eq!(m.mode, NONE);
        m.step(&press(Button::Start), &mut s, true, true);
        assert_eq!(m.mode, AUTO);
        m.stop(&s, 0);
        assert_eq!(m.mode, NONE);
    }
}
