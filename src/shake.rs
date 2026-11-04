//! Le tremblement de la caméra de l'histoire (AD_BGShakeX/Y/Z, et sa part
//! d'AD_CameraProc). Une sorte de tremblement se lit axes × 4 + période :
//! les axes 0 X, 1 Y, 2 Z (le zoom), 3 XY, 4 XZ, 5 YZ, 6 XYZ ; la période
//! 1 en 40 images, 2 en 20, 3 en 12, 4 au hasard. 0 : immobile. Chaque
//! étape de la caméra peut en donner une, et un niveau (l'amplitude, ×
//! niveau + 1) : pendant l'étape, le tremblement passe de l'ancien au
//! nouveau, qui s'installe à sa fin. Voir docs/moteur.md.

use crate::engine::Rng;

/// Pas de tremblement pour cette étape (−1 dans le jeu).
pub const NONE: u8 = 0xFF;

pub struct Shake {
    /// La sorte et le niveau en cours (0x1be9284, 0x1be9285).
    kind: u8,
    level: u8,
    /// Ceux de chaque étape de la caméra (0x1be92ba, 0x1be92bb).
    steps: [(u8, u8); 5],
    /// Le compte des images, de 0 à 3999 (0x1be926e).
    counter: i32,
    rng: Rng,
    /// Le décalage de la caméra : x et y (entiers, comme dans le jeu), z
    /// (en centièmes du zoom).
    pub offset: (i32, i32, f32),
}

impl Default for Shake {
    fn default() -> Shake {
        Shake { kind: 0, level: 0, steps: [(NONE, 0); 5], counter: 0, rng: Rng::new(1), offset: (0, 0, 0.0) }
    }
}

/// Les axes et la période (en images ; 0 : au hasard) d'une sorte.
fn decode(kind: u8) -> (u8, i32) {
    let k = kind.saturating_sub(1);
    (k / 4, [40, 20, 12, 0][usize::from(k % 4)])
}

/// |(c + phase) % période − période / 2|, la dent de scie du jeu.
fn tooth(c: i32, phase: i32, period: i32) -> f32 {
    (((c + phase) % period) - period / 2).abs() as f32
}

impl Shake {
    /// AD_BGTargetPrepare : les étapes sont effacées (le tremblement en
    /// cours continue).
    pub fn prepare(&mut self) {
        self.steps = [(NONE, 0); 5];
    }

    /// AD_BGPrintChange : le nouveau décor ne tremble pas (sa sorte à 0).
    pub fn reset(&mut self) {
        self.kind = 0;
    }

    /// target_shake_set : la sorte et le niveau d'une étape.
    pub fn set_step(&mut self, k: usize, step: (u8, u8)) {
        if let Some(s) = self.steps.get_mut(k) {
            *s = step;
        }
    }

    /// Une étape de la caméra finie : son tremblement s'installe.
    pub fn settle(&mut self, k: usize) {
        if let Some(&(kind, level)) = self.steps.get(k)
            && kind != NONE
        {
            (self.kind, self.level) = (kind, level);
        }
    }

    /// Une image (AD_CameraProc) : le compte avance, le décalage se calcule
    /// d'après l'étape en cours de la caméra (numéro, compte, durée). Rend
    /// la force de la vibration (AD_PadVib) : 0 sans tremblement.
    pub fn step(&mut self, running: Option<(usize, i32, i32)>) -> i32 {
        self.counter = (self.counter + 1) % 4000;
        // Le passage vers le tremblement de l'étape, s'il diffère.
        let blend = running.and_then(|(k, count, dur)| {
            let (kind, level) = *self.steps.get(k)?;
            (kind != NONE && (kind, level) != (self.kind, self.level)).then_some((kind, level, count as f32 / dur.max(1) as f32))
        });
        let x = self.axis(0, blend);
        let y = self.axis(1, blend);
        let z = self.zoom(blend);
        self.offset = (x as i32, y as i32, z / 100.0);
        if self.kind == 0 {
            return 0;
        }
        // La force : un seul axe qui bouge, ou plusieurs.
        let (sx, sy) = (self.offset.0, self.offset.1);
        let moving = [sx != 0, sy != 0, z != 0.0].iter().filter(|&&m| m).count();
        let plane = f64::from(sx.abs() + sy.abs());
        if moving <= 1 {
            ((plane + f64::from((z * 2.5).abs())) * 2.0 + 1.0) as i32
        } else {
            ((plane + f64::from((z * 2.0).abs())) * 3.0 / 2.0) as i32
        }
    }

    /// AD_BGShakeX (axe 0) et AD_BGShakeY (axe 1) : une dent de scie de −3
    /// à 2 fois le niveau + 1, ou au hasard de −2 à 2.
    fn axis(&mut self, axis: u8, blend: Option<(u8, u8, f32)>) -> f32 {
        let moves = |kind: u8| kind != 0 && [[0, 3, 4, 6], [1, 3, 5, 6]][usize::from(axis)].contains(&decode(kind).0);
        let wave = |c: i32, period: i32| {
            let phase = if axis == 0 { 0 } else { [5, 4, 3][[40, 20, 12].iter().position(|&p| p == period).unwrap_or(0)] };
            tooth(c, phase, period) / (period as f32 / 10.0) - 3.0
        };
        let c = self.counter;
        let level = f32::from(self.level) + 1.0;
        let mut f = if moves(self.kind) {
            match decode(self.kind).1 {
                0 => level * (self.rng.next() % 5 - 2) as f32,
                period => level * wave(c, period),
            }
        } else {
            0.0
        };
        if let Some((kind, new_level, k)) = blend {
            f *= 1.0 - k;
            if moves(kind) {
                let level = i32::from(new_level) + 1;
                f += k * match decode(kind).1 {
                    // Au hasard, le jeu tire de −3 à (niveau + 1) × 5 − 3.
                    0 => (level * (self.rng.next() % 6) - 3) as f32,
                    period => level as f32 * wave(c, period),
                };
            }
        }
        f
    }

    /// AD_BGShakeZ : le zoom, une dent de scie de −1 à 1 fois le niveau + 1
    /// (en centièmes), ou au hasard. Les sortes XYZ ont leurs propres
    /// périodes (20, 12, 8 images) ; XZ est décalé.
    fn zoom(&mut self, blend: Option<(u8, u8, f32)>) -> f32 {
        let c = self.counter;
        let wave = |kind: u8| -> Option<f32> {
            let (phase, period) = match kind {
                9 | 21 => (0, 40),
                10 | 22 => (0, 20),
                11 | 23 => (0, 12),
                17 => (5, 40),
                18 => (4, 20),
                19 => (3, 12),
                25 => (0, 20),
                26 => (0, 12),
                27 => (0, 8),
                _ => return None,
            };
            Some(tooth(c, phase, period) / (period / 2) as f32 * 2.0 - 1.0)
        };
        let random = |kind: u8| matches!(kind, 12 | 20 | 24 | 28);
        let level = f32::from(self.level) + 1.0;
        let mut f = match wave(self.kind) {
            Some(w) => level * w,
            None if random(self.kind) => level * ((self.rng.next() % 9) as f32 / 3.0 - 1.0),
            None => 0.0,
        };
        if let Some((kind, new_level, k)) = blend {
            f *= 1.0 - k;
            let level = f32::from(new_level) + 1.0;
            if let Some(w) = wave(kind) {
                // Le jeu oublie ce facteur pour la sorte 0x1b (XYZ, 8
                // images), dont le zoom saute alors d'un coup : bug corrigé,
                // voir le README.
                f += k * level * w;
            } else if random(kind) {
                f += k * level * ((self.rng.next() % 9) as f32 / 6.0 - 1.0);
            }
        }
        f
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// La sorte 3 (X, en 12 images) au niveau 0 : de −3 à 2 sur x
    /// seulement, et la force d'un seul axe.
    #[test]
    fn sawtooth() {
        let mut s = Shake::default();
        s.set_step(0, (3, 0));
        s.settle(0);
        let mut xs = Vec::new();
        for _ in 0..12 {
            let force = s.step(None);
            xs.push(s.offset.0);
            assert_eq!(s.offset.1, 0);
            assert_eq!(force, (s.offset.0.abs() * 2 + 1));
        }
        assert_eq!(xs.iter().min(), Some(&-3));
        assert_eq!(xs.iter().max(), Some(&2));
    }
}
