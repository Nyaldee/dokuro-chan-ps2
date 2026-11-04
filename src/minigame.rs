//! Une partie de mini-jeu (AD_MiniGame*) : la musique, le choix du
//! personnage qui encourage Sakura (AD_MiniGameStProc), le tutoriel, le jeu,
//! puis le noir. Lancée par le menu des mini-jeux (adv_mainmenu, cas 5 et
//! 6), le challenge, ou l'histoire (adv_main, cas 0x0e).

use crate::engine::{EffectDef, Key};
use crate::gfx::{Blend, Gfx, GREY};
use crate::mini::{Minigame, GAMES};
use crate::pointer::{self, Zone};
use crate::sys::Sys;
use crate::{bat, card, catch, mogura, run};

/// Qui lance la partie.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Le menu des mini-jeux : avec tutoriel.
    Menu,
    /// Le challenge : plus dur, sans tutoriel.
    Challenge,
    /// L'histoire : avec tutoriel ; le jeu est noté joué
    /// (AD_MiniGamePlayRecord) après le tutoriel.
    Story,
}

/// Où en est la partie.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Stage {
    /// La musique du jeu.
    Load,
    /// La musique a commencé : le choix du personnage apparaît.
    Music,
    Select,
    SelectOut,
    TutorialIn,
    Tutorial,
    TutorialOut,
    PlayIn,
    Play,
    Leave,
    Restart,
}

/// Ce qui est dessiné (AD_GPTranModeSet2).
#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Nothing,
    Select,
    Tutorial,
    Game,
}

pub struct Session {
    mode: Mode,
    stage: Stage,
    view: View,
    game: Option<Box<dyn Minigame>>,
    /// Le dernier personnage montré au choix.
    shown: i32,
    /// Le jeu vient d'être noté joué (histoire).
    played: bool,
}

impl Session {
    /// Une partie du mini-jeu `sys.kind` ; Sakura est d'abord encouragée par
    /// Dokuro. Dans l'histoire, le choix du personnage est déjà affiché (son
    /// fond est arrivé par-dessus la scène : `arrival`) ; ailleurs, il
    /// paraît depuis le noir.
    pub fn start(mode: Mode, sys: &mut Sys) -> Session {
        sys.chara = 0;
        let view = if mode == Mode::Story { View::Select } else { View::Nothing };
        Session { mode, stage: Stage::Load, view, game: None, shown: 0, played: false }
    }

    /// Un clic au choix du personnage : sur le portrait choisi, il vaut ○ ;
    /// ailleurs, il ne fait rien (le survol choisit). Vrai si le clic ne
    /// vaut pas ○.
    pub fn pick(&self, sys: &mut Sys) -> bool {
        self.stage == Stage::Select && pointer::pick(sys, &portraits()).clicked != Some(sys.chara)
    }

    /// La course est-elle en cours ? (Espace y saute, Ctrl y glisse.)
    pub fn running(&self, sys: &Sys) -> bool {
        self.stage == Stage::Play && sys.kind == 1
    }

    /// Vrai une fois, quand le jeu vient d'être noté joué (histoire).
    pub fn take_played(&mut self) -> bool {
        std::mem::take(&mut self.played)
    }

    /// Une image ; vrai quand la partie est finie (le noir revenu, la
    /// musique arrêtée). Le résultat est dans `sys.won` et `sys.points`.
    pub fn step(&mut self, sys: &mut Sys) -> bool {
        let countdown = |sys: &mut Sys| {
            if sys.lock < 0 {
                return true;
            }
            sys.lock -= 1;
            false
        };
        match self.stage {
            Stage::Load => {
                let n = GAMES[sys.kind as usize].2;
                sys.play_music(n);
                self.stage = Stage::Music;
            }
            Stage::Music => {
                let mode = i32::from(self.mode == Mode::Challenge);
                self.game = Some(match sys.kind {
                    0 => Box::new(mogura::Mogura::start(mode)),
                    1 => Box::new(run::Run::start(mode)),
                    2 => Box::new(card::Cards::start(mode, sys)),
                    3 => Box::new(catch::Catch::start(mode)),
                    _ => Box::new(bat::Bat::start(mode)),
                });
                self.view = View::Select;
                if self.mode != Mode::Story {
                    sys.blackout.end(&mut sys.effects, 10);
                }
                sys.lock = 10;
                self.stage = Stage::Select;
            }
            Stage::Select => {
                if self.select(sys) {
                    sys.blackout.start(&mut sys.effects, 0x14, 0x7F);
                    sys.lock = 0x14;
                    self.stage = Stage::SelectOut;
                }
            }
            Stage::SelectOut => {
                if countdown(sys) {
                    self.stage = if self.mode == Mode::Challenge { Stage::PlayIn } else { Stage::TutorialIn };
                }
            }
            Stage::TutorialIn => {
                self.view = View::Tutorial;
                sys.lock = 10;
                sys.blackout.end(&mut sys.effects, 10);
                self.stage = Stage::Tutorial;
            }
            Stage::Tutorial => {
                if sys.menu_key(0x1F) {
                    sys.lock = 0x14;
                    sys.blackout.start(&mut sys.effects, 0x14, 0x7F);
                    self.stage = Stage::TutorialOut;
                }
            }
            Stage::TutorialOut => {
                if countdown(sys) {
                    self.played = self.mode == Mode::Story;
                    self.stage = Stage::PlayIn;
                }
            }
            Stage::PlayIn => {
                sys.blackout.end(&mut sys.effects, 10);
                self.view = View::Game;
                self.stage = Stage::Play;
            }
            Stage::Play => {
                if self.game.as_mut().is_some_and(|g| g.step(sys)) {
                    sys.lock = 10;
                    sys.blackout.start(&mut sys.effects, 10, 0x7F);
                    self.stage = Stage::Leave;
                }
            }
            Stage::Leave => {
                if countdown(sys) {
                    // Mini_*End, puis la musique s'arrête.
                    sys.se.stop(0);
                    sys.stop_music();
                    sys.lock = 10;
                    self.view = View::Nothing;
                    self.stage = Stage::Restart;
                }
            }
            Stage::Restart => return countdown(sys),
        }
        false
    }

    /// AD_MiniGameStProc : ○ valide ; la croix change de personnage (à
    /// l'appui, puis toutes les 10 images), la souris aussi (le portrait
    /// survolé).
    fn select(&mut self, sys: &mut Sys) -> bool {
        if sys.menu_key(0x1F) {
            select_effect(sys, sys.chara, true);
            sys.menu_sound(2);
            return true;
        }
        if let Some(s) = pointer::pick(sys, &portraits()).hovered {
            sys.chara = s;
        } else if sys.input.direct(2, 10, 10, true) {
            sys.chara = (sys.chara + 4) % 5;
        } else if sys.input.direct(3, 10, 10, true) {
            sys.chara = (sys.chara + 1) % 5;
        }
        if self.shown != sys.chara {
            select_effect(sys, sys.chara, false);
            self.shown = sys.chara;
            sys.lock = 5;
            sys.menu_sound(1);
        }
        false
    }

    /// Le choix (AD_MiniGameStTran), le tutoriel, ou le jeu.
    pub fn draw(&self, gfx: &mut Gfx, sys: &Sys) {
        match self.view {
            View::Nothing => {}
            View::Select => {
                for s in 0..5 {
                    let chosen = s == sys.chara;
                    gfx.tran_center(Some(SEL), if chosen { 1.0 } else { 0.9 }, portrait(s), [portrait_x(s), 214.0, PORTRAIT.0, PORTRAIT.1], 0xAF, if chosen { 0x7F } else { 0x50 }, GREY, 0.0, None);
                }
                gfx.tran(SEL_BG, [0.0, 0.0, 512.0, 448.0], [0.0, 0.0, 640.0, 448.0], 0xAA, 0x7F, Blend::Normal, GREY);
            }
            View::Tutorial => {
                let name = tutorial(sys.kind, sys.chara.clamp(0, 4));
                gfx.tran(name, [0.0, 0.0, 512.0, 448.0], [0.0, 0.0, 640.0, 448.0], 0xAA, 0x7F, Blend::Normal, GREY);
            }
            View::Game => {
                if let Some(g) = &self.game {
                    g.draw(gfx, sys);
                }
            }
        }
    }
}

/// Les images du choix (SYSTEM.UNI 40 et 41).
pub const SEL: &str = "mini_sel";
pub const SEL_BG: &str = "mini_selbg";

/// La durée de l'arrivée du choix dans l'histoire.
pub const ARRIVAL: i32 = 10;

/// AD_MiniGameStartStEffectMake : dans l'histoire, le fond du choix arrive
/// par-dessus la scène, de l'échelle 2 et transparent à l'échelle 1 et
/// opaque en 10 images (centré en 0x140, 0xcc, prio 0xab), puis s'efface ;
/// le choix se dessine alors. Rend la durée de l'arrivée.
pub fn arrival(effects: &mut crate::engine::Effects) -> i32 {
    let mut d = EffectDef::ready();
    d.texture = Some(SEL_BG);
    d.src = [0.0, 0.0, 512.0, 448.0];
    d.size = (640.0, 448.0);
    d.prio = 0xAB;
    d.pos = (0x140, 0xCC);
    d.alpha = 0;
    d.scale = 2.0;
    d.keys[0] = Key { dur: ARRIVAL, alpha: 0x7F, scale: 1.0, ..Key::NONE };
    effects.make(d);
    ARRIVAL
}

/// Le tutoriel d'un mini-jeu pour un personnage (« tutorial%d%d »,
/// MINI.UNI n° jeu × 10 + personnage).
pub fn tutorial(kind: i32, chara: i32) -> &'static str {
    const NAMES: [[&str; 5]; 5] = [
        ["tutorial00", "tutorial01", "tutorial02", "tutorial03", "tutorial04"],
        ["tutorial10", "tutorial11", "tutorial12", "tutorial13", "tutorial14"],
        ["tutorial20", "tutorial21", "tutorial22", "tutorial23", "tutorial24"],
        ["tutorial30", "tutorial31", "tutorial32", "tutorial33", "tutorial34"],
        ["tutorial40", "tutorial41", "tutorial42", "tutorial43", "tutorial44"],
    ];
    NAMES[kind.clamp(0, 4) as usize][chara.clamp(0, 4) as usize]
}

/// La taille d'un portrait, et le centre de sa place à l'écran (y = 0xd6).
const PORTRAIT: (f32, f32) = (125.0, 235.0);
fn portrait_x(s: i32) -> f32 {
    (s * 0x78 + 0x50) as f32
}

/// Les portraits à l'écran, à leur taille réduite (0,9), pour la souris.
fn portraits() -> [Zone<i32>; 5] {
    std::array::from_fn(|s| Zone::centered(portrait_x(s as i32), 214.0, PORTRAIT.0 * 0.9, PORTRAIT.1 * 0.9, s as i32))
}

/// Le portrait d'un personnage dans mini_sel (trois par rangée).
fn portrait(s: i32) -> [f32; 4] {
    [((s % 3) * 0x8C + 2) as f32, ((s / 3) * 0xF8) as f32, PORTRAIT.0, PORTRAIT.1]
}

/// AD_MiniGameSelEffMake : le portrait s'illumine (changement) ou grandit en
/// s'effaçant (choix).
fn select_effect(sys: &mut Sys, s: i32, chosen: bool) {
    let mut d = EffectDef::ready();
    d.texture = Some(SEL);
    d.src = portrait(s);
    d.size = PORTRAIT;
    d.prio = 0xB0;
    d.pos = (portrait_x(s) as i32, 0xD6);
    d.alpha = 0x50;
    let dur = if chosen { 10 } else { 7 };
    d.scale = if chosen { 0.9 } else { 1.0 };
    d.keys[0] = Key { dur, alpha: 0x7F, rgb: [0xDC; 3], scale: if chosen { 1.2 } else { 1.0 }, ..Key::NONE };
    d.keys[1] = Key { dur, alpha: 0, rgb: [0x7F; 3], scale: if chosen { 1.5 } else { 1.0 }, ..Key::NONE };
    sys.effects.make(d);
}
