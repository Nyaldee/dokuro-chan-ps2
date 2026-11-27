//! La vibration de la manette (AD_PadVib, AD_InputProc, PadVibration) :
//! le tremblement de la caméra en donne la force à chaque image ; la
//! manette vibre tant qu'il dure (le petit moteur au-delà de 4, le gros à
//! 3 fois la force), et se coupe une image toutes les 30 secondes.
//! L'option « 振動 » la coupe ; l'écran des options la verrouille.

/// Une vibration ininterrompue est coupée après 0x708 images (30 s).
const LONGEST: i32 = 0x708;

pub struct Rumble {
    /// La force donnée depuis la dernière image (iRam01c27c40 ; −1 : rien),
    /// l'état (sRam01c27c48 : les images de vibration, −1 coupée, −3 au
    /// repos), le verrou (cRam01c27c3c).
    force: i32,
    state: i32,
    locked: bool,
    /// Les moteurs : le petit (en marche ou non), le gros (0 à 255), les
    /// images qu'il leur reste (−1 : sans fin).
    small: bool,
    big: u8,
    left: i32,
}

impl Default for Rumble {
    fn default() -> Rumble {
        Rumble { force: -1, state: -3, locked: false, small: false, big: 0, left: 0 }
    }
}

impl Rumble {
    /// AD_PadVib : une force de plus pour cette image (sans effet sous
    /// verrou).
    pub fn add(&mut self, v: i32) {
        if !self.locked {
            self.force = self.force.max(0) + v;
        }
    }

    /// AD_PadVibLock : l'écran des options verrouille (et arrête) la
    /// vibration.
    pub fn lock(&mut self, on: bool) {
        self.locked = on;
        if on {
            self.set(0, 0, true);
        }
    }

    /// PadVibration(0, 2, durée, force) : les deux moteurs ; une force
    /// positive ne passe pas si l'option est coupée.
    fn set(&mut self, dur: i32, force: i32, enabled: bool) {
        if force > 0 && !enabled {
            return;
        }
        let force = force.clamp(0, 0xFF);
        self.small = force > 4;
        self.big = (force * 3).min(0xFF) as u8;
        self.left = dur.clamp(-4, 4);
    }

    /// AD_PadVib2(2, 4, 0x96) : l'essai des options, un instant.
    pub fn test(&mut self, enabled: bool) {
        self.set(4, 0x96, enabled);
    }

    /// AD_InputProc, au début de chaque image : la force reçue part à la
    /// manette, ou l'arrête. Puis la durée de la vibration s'écoule.
    pub fn frame(&mut self, enabled: bool) {
        if self.left > 0 {
            self.left -= 1;
            if self.left == 0 {
                (self.small, self.big) = (false, 0);
            }
        }
        if !(0..LONGEST).contains(&self.state) {
            if self.state == -1 {
                if self.force < 1 {
                    self.state = -3;
                    self.set(0, 0, enabled);
                } else {
                    self.set(-1, self.force, enabled);
                }
                self.force = -1;
            } else if self.state == -3 && self.force > 0 {
                self.state = 0;
            }
            return;
        }
        self.state += 1;
        if self.state == LONGEST || self.force == 0 {
            self.state = -1;
            self.force = -1;
            self.set(0, 0, enabled);
        } else if self.force > 0 {
            self.set(-1, self.force, enabled);
            self.force = -1;
        }
    }

    /// Les moteurs : le petit, le gros.
    pub fn motors(&self) -> (bool, u8) {
        (self.small, self.big)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Une force fait vibrer à l'image qui suit le repos (les deux forces
    /// reçues s'ajoutent), et s'arrête quand le tremblement cesse ; l'option
    /// coupée l'empêche.
    #[test]
    fn follows_the_shake() {
        let mut r = Rumble::default();
        r.add(10);
        r.frame(true);
        assert_eq!(r.motors(), (false, 0));
        r.add(10);
        r.frame(true);
        assert_eq!(r.motors(), (true, 60));
        r.add(0);
        r.frame(true);
        assert_eq!(r.motors(), (false, 0));
        let mut off = Rumble::default();
        for _ in 0..3 {
            off.add(10);
            off.frame(false);
        }
        assert_eq!(off.motors(), (false, 0));
    }
}
