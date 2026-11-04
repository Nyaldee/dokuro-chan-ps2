//! L'écran titre (adv_title) : le décor, le logo, la mention du bas et
//! l'invite qui pulse ; START mène à la chambre de Sakura. Une minute sans
//! rien faire lance la vidéo d'ouverture, puis le titre revient.

use crate::engine::Button;
use crate::gfx::{Blend, Gfx, GREY};
use crate::sys::Sys;

/// Les images (SYSTEM.UNI 10 et 11).
pub const BACK: &str = "title";
pub const LOGO: &str = "t_rogo";
/// La musique du titre.
const MUSIC: u32 = 1;
/// Les images sans rien faire avant la vidéo d'ouverture.
const IDLE: i32 = 0xE10;

/// Où en est le titre (états et sous-états d'adv_title).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    /// État 1 : le noir s'efface (0x14 images), le logo attend 0x28 images.
    Appear,
    Wait,
    /// État 2 : l'invite, START.
    Ready,
    /// État 3 : le noir, puis la chambre.
    Leave,
    /// État 6 : le noir, puis la vidéo d'ouverture.
    Idle,
}

/// Ce que demande le titre au jeu.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Leave {
    Room,
    /// La vidéo d'ouverture (« op »).
    Opening,
}

pub struct Title {
    stage: Stage,
    timer: i32,
    /// Les images passées sans rien faire (sRam003661ac).
    idle: i32,
    /// L'invite (AD_TitleOseTran) : son opacité (0 à 0x80) et son angle.
    prompt: (i32, i32),
    leave: Option<Leave>,
}

impl Title {
    /// Le titre depuis le noir (état 1, sous-états 0 à 0x14).
    pub fn new(sys: &mut Sys) -> Title {
        sys.blackout.end(&mut sys.effects, 0x14);
        Title { stage: Stage::Appear, timer: 0x14, idle: 0, prompt: (0, 0), leave: None }
    }

    /// Le titre revenu après la vidéo d'ouverture (état 6, sous-état 0x14).
    pub fn back(&mut self, sys: &mut Sys) {
        sys.blackout.end(&mut sys.effects, 0xF);
        sys.play_music(MUSIC);
        self.stage = Stage::Ready;
        self.idle = 0;
    }

    pub fn take_leave(&mut self) -> Option<Leave> {
        self.leave.take()
    }

    /// Le compteur des images sans rien faire, au moment de START : la
    /// chambre en tire son personnage (AD_MainmenuCharaSet).
    pub fn idle(&self) -> i32 {
        self.idle
    }

    pub fn step(&mut self, sys: &mut Sys) {
        let countdown = |t: &mut i32| {
            *t -= 1;
            *t < 0
        };
        match self.stage {
            Stage::Appear => {
                if countdown(&mut self.timer) {
                    self.timer = 0x28;
                    self.stage = Stage::Wait;
                }
            }
            Stage::Wait => {
                if countdown(&mut self.timer) {
                    sys.play_music(MUSIC);
                    self.idle = 0;
                    self.stage = Stage::Ready;
                }
            }
            Stage::Ready => {
                self.pulse();
                self.idle += 1;
                if self.idle > IDLE {
                    sys.blackout.start(&mut sys.effects, 0x1E, 0x7F);
                    sys.stop_music();
                    self.timer = 0x1E;
                    self.stage = Stage::Idle;
                } else if sys.input.key(Button::Start as i32) || sys.click.is_some() {
                    // START (ou un clic) : la chambre.
                    sys.menu_sound(2);
                    sys.blackout.start(&mut sys.effects, 10, 0x7F);
                    self.timer = 10;
                    self.stage = Stage::Leave;
                }
            }
            Stage::Leave => {
                if countdown(&mut self.timer) {
                    sys.stop_music();
                    self.leave = Some(Leave::Room);
                }
            }
            Stage::Idle => {
                if countdown(&mut self.timer) {
                    self.leave = Some(Leave::Opening);
                }
            }
        }
    }

    /// AD_TitleOseTran : l'invite paraît (10 par image jusqu'à 0x80) et
    /// pulse en sinus. (Le jeu sait aussi l'effacer, dans un sous-état du
    /// titre — une question oui / non — qu'il n'atteint jamais.)
    fn pulse(&mut self) {
        let (alpha, angle) = &mut self.prompt;
        *alpha = (*alpha + 10).min(0x80);
        *angle = (*angle + 1) % 0x5A;
    }

    /// Le décor, le logo (AD_TitleLogoTran), la mention du bas
    /// (AD_TitleCTran), l'invite.
    pub fn draw(&self, gfx: &mut Gfx) {
        gfx.tran(BACK, [0.0, 0.0, 512.0, 448.0], [0.0, 0.0, 640.0, 448.0], 0x0A, 0x7F, Blend::Normal, GREY);
        gfx.tran_center(Some(LOGO), 1.0, [0.0, 13.0, 339.0, 246.0], [320.0, 160.0, 339.0, 246.0], 0xDE, 0x7F, GREY, 0.0, None);
        gfx.tran_center(Some(LOGO), 1.0, [3.0, 280.0, 378.0, 37.0], [320.0, 410.0, 378.0, 37.0], 0xDD, 0x7F, GREY, 0.0, None);
        let (alpha, angle) = self.prompt;
        let k = ((angle * 2) as f32).to_radians().sin() * 0.5 + 0.5;
        gfx.tran_center(Some(LOGO), 1.0, [0.0, 325.0, 316.0, 28.0], [320.0, 340.0, 316.0, 28.0], 0xDD, (alpha as f32 * k) as i32, GREY, 0.0, None);
    }
}
