//! Les options (AD_Option*) : les réglages (volumes, avance rapide,
//! vibration, fenêtre de texte, vitesse du texte) et leur écran, ouvert
//! depuis la chambre (adv_mainmenu, état 8) ou depuis le menu du jeu
//! (AD_AdvSubProc, sous-état 5). Voir docs/moteur.md.

use crate::dialog::{Action, Windows};
use crate::font::{self, Atlas, Message, Pen, sprintf};
use crate::gfx::{Blend, Gfx, GREY};
use crate::pointer::{self, Zone};
use crate::sys::Sys;

/// Les réglages (0x1c28360, dans l'ordre de leurs octets).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Settings {
    /// Les volumes de la musique, des bruitages et des voix (0 à 0x7f).
    pub volume: [u8; 3],
    /// L'avance rapide (AD_OptionSkipmodeCheck) : 0 les pages lues, 1 tout.
    pub skip_mode: u8,
    /// Couper les effets (AD_OptionEffCutGet) : l'écran n'en montre pas
    /// l'entrée, le réglage reste à 0.
    pub effect_cut: u8,
    pub vibration: u8,
    /// La vitesse du texte (0 à 10).
    pub text_speed: u8,
    /// Les opacités du cadre et du fond de la fenêtre de texte (0 à 0x7f).
    pub window: [u8; 2],
    /// Les trois avances (forcée, des pages lues, automatique) : vitesse
    /// (0 à 10) et conditions d'arrêt (bits 1, 2, 4 : un choix, une page
    /// pas encore lue, un point d'affection).
    pub skip: [[u8; 2]; 3],
}

impl Default for Settings {
    /// AD_OptionInit.
    fn default() -> Settings {
        Settings { volume: [0x3C, 100, 0x78], skip_mode: 0, effect_cut: 0, vibration: 1, text_speed: 5, window: [100, 100], skip: [[5, 0xF], [5, 0xF], [5, 0xD]] }
    }
}

impl Settings {
    /// Les quinze octets, dans l'ordre du jeu.
    pub fn bytes(&self) -> [u8; 15] {
        let [v0, v1, v2] = self.volume;
        let [[s0, b0], [s1, b1], [s2, b2]] = self.skip;
        [v0, v1, v2, self.skip_mode, self.effect_cut, self.vibration, self.text_speed, self.window[0], self.window[1], s0, b0, s1, b1, s2, b2]
    }

    /// Des octets gardés, chacun ramené dans sa plage.
    pub fn from_bytes(b: [u8; 15]) -> Settings {
        let byte = |k: usize, max: u8| b[k].min(max);
        Settings {
            volume: [byte(0, 0x7F), byte(1, 0x7F), byte(2, 0x7F)],
            skip_mode: byte(3, 1),
            effect_cut: byte(4, 1),
            vibration: byte(5, 1),
            text_speed: byte(6, 10),
            window: [byte(7, 0x7F), byte(8, 0x7F)],
            skip: [[byte(9, 10), b[10]], [byte(11, 10), b[12]], [byte(13, 10), b[14]]],
        }
    }

    /// AD_OptionSkipSpeedGet : la vitesse de l'avance rapide en cours (celle
    /// des pages lues, ou la forcée), ou celle de la lecture automatique.
    pub fn skip_speed(&self, auto: bool) -> i32 {
        i32::from(self.skip[if auto { 2 } else if self.skip_mode == 0 { 1 } else { 0 }][0])
    }

    /// AD_OptionSkipStopCheck : l'avance rapide (ou la lecture automatique)
    /// s'arrête-t-elle à ce cas (0 un choix, 1 une page pas lue, 2 un point
    /// d'affection, 3 une vidéo) ? L'avance forcée ne s'arrête jamais aux
    /// pages pas lues.
    pub fn skip_stops(&self, auto: bool, case: i32) -> bool {
        let bits = if auto {
            self.skip[2][1]
        } else if self.skip_mode == 0 {
            self.skip[1][1]
        } else if case != 1 {
            self.skip[0][1]
        } else {
            0
        };
        i32::from(bits as i8) & (1 << (case & 0x1F)) > 0
    }
}

/// Les fenêtres et les curseurs de l'écran (leurs noms dans le jeu).
const OPTION: &str = "option";
const INFO: &str = "op_info";
const MAIN: &str = "op_main";
const GAME: &str = "op_game";
const SKIP_MODE: &str = "op_skipm";
const VIBRATION: &str = "op_vib";
const WINDOW: &str = "op_win";
const SKIP: &str = "op_skip";
const SKIP_DETAIL: &str = "op_skip2";
const SOUND: &str = "op_sound";
/// Les fenêtres des options (centre, taille) et la hauteur des entrées de
/// celle du jeu.
const OPTION_WIN: [i32; 4] = [0xB9, 0x7D, 0x10E, 200];
const GAME_WIN: [i32; 4] = [0x140, 200, 0x1AE, 0x145];
const SKIP_WIN: [i32; 4] = [0x186, 0xD2, 0x1C2, 0x140];
const SOUND_WIN: [i32; 4] = [0x14A, 0x78, 0x172, 0x96];
const GAME_ROWS: [i32; 7] = [0x43, 0x66, 0x89, 0xAC, 0x10B, 0x12E, 0x150];

/// Les phrases de l'écran, dans l'exécutable (`Tools/exe_text.py`).
mod text {
    /// « ゲーム設定 », « サウンド設定 », « 設定初期化 », « 決定　　キャンセル ».
    pub const MAIN: [usize; 4] = [0x364D80, 0x364D90, 0x364DA0, 0x364DC0];
    /// Ce que fait chaque entrée du menu des options (0, 1, 2 ; 3 : garder
    /// ou annuler).
    pub const MAIN_INFO: [usize; 3] = [0x3645C0, 0x3645E0, 0x364600];
    pub const CANCEL_INFO: usize = 0x364630;
    pub const KEEP_INFO: usize = 0x364660;
    /// La fenêtre de jeu : ses lignes, puis ce que fait chaque entrée (la 1,
    /// « 愛キャッチ », n'est pas affichée).
    pub const GAME: [usize; 8] = [0x364C50, 0x364C70, 0x364CA0, 0x364CC0, 0x364CE0, 0x364D00, 0x364D30, 0x364D60];
    pub const GAME_INFO: [usize; 7] = [0x364680, 0x3646A0, 0x3646D0, 0x3646F0, 0x364720, 0x364750, BACK];
    pub const SKIP_MODE_INFO: [usize; 2] = [0x364790, 0x3647C0];
    pub const VIBRATION_INFO: [usize; 2] = [0x364830, 0x364850];
    pub const WINDOW_INFO: [usize; 2] = [0x364870, 0x3648A0];
    /// La fenêtre de l'avance rapide : ce que fait chaque mode, sa vitesse.
    pub const SKIP_INFO: [usize; 3] = [0x3648D0, 0x3648F0, 0x364910];
    pub const SPEED_INFO: [usize; 3] = [0x364930, 0x364950, 0x364970];
    /// Les conditions d'arrêt : « 選択肢 », « 未読文章 », « 好感度確認 » ;
    /// « で » ou « 後も » ; « オートを » ou « スキップを » ; « 解除します » ou
    /// « 継続します ».
    pub const STOP: [usize; 3] = [0x364990, 0x364998, 0x3649A8];
    pub const AT: usize = 0x3649B8;
    pub const AFTER: usize = 0x3649C0;
    pub const AUTO: usize = 0x3649C8;
    pub const SKIPPING: usize = 0x3649D8;
    pub const STOPS: usize = 0x3649E8;
    pub const GOES_ON: usize = 0x3649F8;
    /// Les lignes de la fenêtre de l'avance rapide (`#Color[%d]` : 0 coché,
    /// 6 non).
    pub const SKIP_TITLE: usize = 0x364B18;
    pub const SKIP_SPEED: usize = 0x364B30;
    pub const SKIP_STOP: usize = 0x364B58;
    pub const FORCED_BOXES: usize = 0x364B60;
    pub const READ_TITLE: usize = 0x364B90;
    pub const BOXES: usize = 0x364BC0;
    pub const AUTO_TITLE: usize = 0x364C00;
    pub const SKIP_BACK: usize = 0x364C30;
    /// Le son : ses lignes, ce que fait chaque entrée.
    pub const SOUND: [usize; 4] = [0x364AA0, 0x364AC0, 0x364AE0, 0x364B00];
    pub const SOUND_INFO: [usize; 4] = [0x364A10, 0x364A30, 0x364A50, BACK];
    /// « 戻ります ».
    pub const BACK: usize = 0x364780;
    /// « 設定を初期化します よろしいですか？ ».
    pub const RESET_ASK: usize = 0x364500;
}

/// D'où viennent les options.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Origin {
    Room,
    Story,
}

/// Avant et après l'écran (adv_mainmenu état 8, AD_AdvSubProc sous-état
/// 5) : l'attente avant que l'écran réponde, l'écran, puis le noir et la
/// sauvegarde si un réglage a changé, la musique qui repart.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Outer {
    Opening,
    Running,
    /// Les fenêtres se ferment (10 images) ; l'écran noircit si un réglage
    /// a changé.
    Closing,
    /// Les volumes, puis 10 images avant que la musique reparte.
    Volumes,
    Done,
}

pub struct Options {
    from: Origin,
    outer: Outer,
    /// Un réglage a changé (AD_OptionEnd).
    changed: bool,
    /// Les réglages sont à garder (dans la sauvegarde du portage).
    save: bool,
    /// Le niveau (0x1c283a0 : 0 le menu, 1 la fenêtre de jeu, 2 le son), le
    /// sous-état (0x1c283a4), la valeur en cours (0x1c283a8), l'entrée
    /// choisie (0x1c283ac), et ces quatre valeurs telles que les a vues
    /// AD_OptionCursorProc.
    level: i32,
    sub: i32,
    value: i32,
    entry: i32,
    seen: [i32; 4],
    /// Les réglages à l'ouverture (0x1c28380), que « キャンセル » remet.
    saved: Settings,
    windows: Windows,
    /// L'essai de la vibration (sRam00366174) : les images avant qu'elle
    /// vibre.
    vibration_test: i32,
    /// Les jauges (AD_GPTranSubModeSet, AD_OptionTran) : le mode dessiné,
    /// le mode voulu, l'attente avant de passer à un mode plus haut (10
    /// images pour les modes 2 à 4, le temps qu'une fenêtre s'ouvre).
    gauges: (i32, i32, i32),
}

impl Options {
    /// AD_OptionStart : les réglages sont gardés, la fenêtre des options
    /// et celle des explications s'ouvrent (AD_OptionWinMake). Depuis
    /// l'histoire, le menu du jeu a déjà assombri l'écran (plan 4 à 0x7f)
    /// et caché la fenêtre de texte ; l'écran répond après 15 images
    /// (10 depuis la chambre).
    pub fn start(from: Origin, sys: &mut Sys, pen: &mut Pen) -> Options {
        let mut o = Options {
            from,
            outer: Outer::Opening,
            changed: false,
            save: false,
            level: 0,
            sub: 0,
            value: 0,
            entry: 0,
            seen: [0; 4],
            saved: sys.settings,
            windows: Windows::default(),
            vibration_test: 0,
            gauges: (0, 0, 0),
        };
        sys.rumble.lock(true);
        let main = pen.glyphs(&pen.join(&text::MAIN));
        o.open(OPTION, 1, 0xE6, (0x23, 0xF), main, OPTION_WIN);
        let info = pen.glyphs(pen.font.string(text::MAIN_INFO[0]));
        o.open(INFO, 2, 0xE6, (0x12, 5), info, [0x140, 400, 600, 0x23]);
        o.windows.cursor_make(MAIN, (0x44, 0x34), 0xE7, 0x7F, 0xF);
        sys.lock = if from == Origin::Story { 0xF } else { 10 };
        o
    }

    /// Une fenêtre s'ouvre en 10 images (style, priorité, place du texte,
    /// centre et taille).
    fn open(&mut self, name: &'static str, style: usize, prio: i32, offset: (i32, i32), glyphs: Vec<font::Glyph>, [x, y, w, h]: [i32; 4]) {
        self.windows.create(name, style, prio, 2, None, offset, glyphs);
        self.windows.act(name, &[Action { dur: 10, kind: 1, x, y, w, h, frame: 0x80, back: 0x80, ..Action::READY }]);
    }

    /// Une fenêtre se ferme et disparaît en 10 images.
    fn close(&mut self, name: &str) {
        self.windows.act(name, &[Action { dur: 10, kind: 3, ..Action::READY }]);
    }

    /// Le portage garde les réglages : vrai une fois, quand ils ont changé
    /// (là où le jeu écrit sur la carte mémoire).
    pub fn take_save(&mut self) -> bool {
        std::mem::take(&mut self.save)
    }

    pub fn origin(&self) -> Origin {
        self.from
    }

    /// Une image ; vrai une fois tout fini (la chambre ou le menu du jeu
    /// reprend).
    pub fn step(&mut self, sys: &mut Sys, pen: &mut Pen) -> bool {
        let countdown = |sys: &mut Sys| {
            if sys.lock < 1 {
                return true;
            }
            sys.lock -= 1;
            false
        };
        match self.outer {
            Outer::Opening => {
                if countdown(sys) {
                    self.outer = Outer::Running;
                }
            }
            Outer::Running => {
                if self.proc(sys, pen) {
                    // AD_OptionEnd : les fenêtres se ferment ; la musique
                    // s'arrête ; un réglage changé noircit l'écran.
                    self.close(OPTION);
                    self.close(INFO);
                    self.windows.cursor_del(MAIN, 0xF);
                    sys.rumble.lock(false);
                    self.changed = sys.settings != self.saved;
                    sys.pause_music();
                    if self.changed {
                        sys.blackout.start(&mut sys.effects, 10, 0x7F);
                    }
                    sys.lock = 10;
                    self.outer = Outer::Closing;
                }
            }
            Outer::Closing => {
                if countdown(sys) {
                    if self.changed {
                        // AD_AdvSystemSaveProc et l'écran de la carte
                        // mémoire (AD_SystemMcProc) : le portage écrit sa
                        // sauvegarde, puis le noir s'efface.
                        self.save = true;
                        sys.blackout.end(&mut sys.effects, 10);
                    }
                    if self.from == Origin::Room && self.changed {
                        // Depuis la chambre, la musique repart aussitôt.
                        sys.restart_music(false);
                        self.outer = Outer::Done;
                    } else {
                        sys.apply_volumes();
                        sys.lock = 10;
                        self.outer = Outer::Volumes;
                    }
                }
            }
            Outer::Volumes => {
                if countdown(sys) {
                    sys.restart_music(false);
                    self.outer = Outer::Done;
                }
            }
            Outer::Done => return true,
        }
        self.windows.update();
        self.step_gauges();
        false
    }

    /// AD_GPTranSubModeSet.
    fn set_gauges(&mut self, mode: i32) {
        let (shown, _, _) = self.gauges;
        if mode > shown {
            self.gauges = (shown, mode, if (2..=4).contains(&mode) { 10 } else { 0 });
        } else {
            self.gauges = (mode, mode, 0);
        }
    }

    fn step_gauges(&mut self) {
        let (shown, wanted, delay) = &mut self.gauges;
        if *shown != *wanted {
            if *delay > 0 {
                *delay -= 1;
            } else {
                *shown = *wanted;
            }
        }
    }

    /// AD_OptionProc : vrai quand le joueur quitte (garder ou annuler).
    fn proc(&mut self, sys: &mut Sys, pen: &mut Pen) -> bool {
        self.hover(sys);
        match self.level {
            2 => self.sound(sys),
            1 if (0x3C..=0x50).contains(&self.sub) => self.skip_detail(sys, pen),
            1 => self.game(sys, pen),
            _ => {
                if self.main(sys, pen) {
                    return true;
                }
            }
        }
        let now = [self.level, self.sub, self.value, self.entry];
        if now != self.seen {
            self.cursor_proc(sys, pen);
            self.seen = now;
        }
        false
    }

    /// Niveau 0, le menu des options ; vrai quand le joueur quitte.
    fn main(&mut self, sys: &mut Sys, pen: &mut Pen) -> bool {
        match self.sub {
            0x28 | 0x1E => {
                if self.sub == 0x1E {
                    sys.apply_volumes();
                    sys.restart_music(false);
                }
                sys.lock = 0xF;
                self.value = 0;
                self.seen[2] = 0;
                self.sub = 0;
                self.windows.cursor_show(MAIN, 0x7F, 10);
            }
            // La musique s'est arrêtée (aussitôt dans le portage).
            0x14 => self.sub = 0x1E,
            10 => {
                match self.windows.answer(sys) {
                    // はい : les réglages de départ (AD_OptionInit(1)).
                    Some(0) => {
                        sys.settings = Settings::default();
                        if sys.music().is_some() {
                            sys.pause_music();
                        }
                        self.sub = 0x14;
                    }
                    Some(_) => self.sub = 0x28,
                    None => {}
                }
            }
            _ => {
                if up(sys, 8, 5) {
                    self.entry -= 1;
                } else if down(sys, 8, 5) {
                    self.entry += 1;
                } else if sys.menu_key(0x20) {
                    // ✕ : le curseur sur « キャンセル ».
                    self.value = 1;
                    self.entry = 3;
                } else if self.entry == 3 {
                    if left(sys, 8, 5) || right(sys, 8, 5) {
                        self.value = (self.value + 1) & 1;
                    }
                } else {
                    self.value = 0;
                }
                if self.entry == self.seen[3] && self.value == self.seen[2] {
                    if sys.menu_key(0x21) {
                        sys.menu_sound(2);
                        match self.entry {
                            3 => {
                                self.set_gauges(0);
                                if self.value == 1 {
                                    sys.settings = self.saved;
                                }
                                return true;
                            }
                            2 => {
                                self.sub = 10;
                                self.windows.cursor_hide(MAIN, 0x3C, 10);
                                let (q, yes, no) = (pen.glyphs(pen.font.string(text::RESET_ASK)), pen.glyphs(pen.font.message(Message::Yes)), pen.glyphs(pen.font.message(Message::No)));
                                self.windows.ask(q, yes, no, (0xD2, 0x37));
                            }
                            1 => {
                                self.windows.cursor_hide(MAIN, 0x3C, 10);
                                self.level = 2;
                                self.sub = 0;
                                self.entry = 0;
                                self.set_gauges(4);
                                self.open_sound(pen);
                            }
                            _ => {
                                self.windows.cursor_hide(MAIN, 0x3C, 10);
                                self.level = 1;
                                self.sub = 0;
                                self.set_gauges(2);
                                self.open_game(sys, pen);
                            }
                        }
                        sys.lock = 0xF;
                    }
                } else {
                    sys.menu_sound(1);
                    self.entry = self.entry.rem_euclid(4);
                }
            }
        }
        false
    }

    /// Niveau 1, la fenêtre de jeu (ses sous-états sauf l'avance rapide
    /// détaillée).
    fn game(&mut self, sys: &mut Sys, pen: &mut Pen) {
        let ok = |sys: &mut Sys| sys.menu_key(0x21) || sys.menu_key(0x20);
        match self.sub {
            // La fenêtre de l'avance rapide : ses trois modes et « 戻る ».
            0x32 => {
                if sys.menu_key(0x21) || sys.menu_key(0x20) {
                    if sys.input.key(0x20) || !(0..=2).contains(&self.entry) {
                        // Le curseur revient sur l'entrée 4 de la fenêtre de
                        // jeu (le jeu la ramène ensuite à 0 avec le compte de
                        // cette fenêtre-ci : bug corrigé, voir le README).
                        self.sub = 0;
                        self.entry = 4;
                        sys.menu_sound(0);
                        self.close(SKIP);
                        self.windows.cursor_del(SKIP, 0xF);
                        self.windows.cursor_show(GAME, 0x7F, 10);
                        self.set_gauges(2);
                        sys.lock = 0xF;
                    } else {
                        self.windows.cursor_hide(SKIP, 0x3C, 10);
                        sys.menu_sound(2);
                        self.sub = self.entry * 10 + 0x3C;
                        self.windows.cursor_make(SKIP_DETAIL, (0xD2, self.entry * 0x5A + 0x67), 0xEC, 0x7F, 0xF);
                        self.value = 0;
                        self.entry = 0;
                    }
                } else if up(sys, 8, 5) {
                    self.entry -= 1;
                } else if down(sys, 8, 5) {
                    self.entry += 1;
                }
                if self.sub == 0x32 && self.entry != self.seen[3] {
                    sys.menu_sound(1);
                    self.entry = self.entry.rem_euclid(4);
                }
            }
            // Les opacités de la fenêtre de texte : le cadre, le fond.
            0x28 => {
                if up(sys, 8, 5) {
                    self.entry -= 1;
                } else if down(sys, 8, 5) {
                    self.entry += 1;
                }
                if self.entry == self.seen[3] {
                    let step = sideways(sys, 8, 1);
                    nudge(&mut sys.settings.window[(self.entry & 1) as usize], step, 0x7F);
                } else {
                    self.entry = self.entry.rem_euclid(2);
                    sys.menu_sound(1);
                }
                if ok(sys) {
                    self.windows.cursor_show(GAME, 0x7F, 10);
                    self.windows.cursor_del(WINDOW, 10);
                    self.entry = 3;
                    self.sub = 0;
                    sys.menu_sound(0);
                    sys.lock = 10;
                }
            }
            // La vibration : 7 images après un changement, et en revenant,
            // la manette vibre un instant si elle est en marche.
            0x1E => {
                if left(sys, 8, 5) || right(sys, 8, 5) {
                    self.value = (self.value + 1) & 1;
                    sys.settings.vibration = (sys.settings.vibration + 1) & 1;
                    self.vibration_test = 7;
                    sys.menu_sound(1);
                }
                if ok(sys) {
                    self.windows.cursor_hide(VIBRATION, 0x3C, 10);
                    self.windows.cursor_show(GAME, 0x7F, 10);
                    self.sub = 0;
                    sys.lock = 0xF;
                    sys.menu_sound(0);
                    self.vibration_test = 1;
                }
                if self.vibration_test > 0 {
                    self.vibration_test -= 1;
                    if self.vibration_test == 0 && sys.settings.vibration == 1 {
                        sys.rumble.test(true);
                    }
                }
            }
            // Le mode d'avance rapide.
            10 => {
                if left(sys, 8, 5) {
                    self.value -= 1;
                } else if right(sys, 8, 5) {
                    self.value += 1;
                }
                if self.value != self.seen[2] {
                    self.value = self.value.rem_euclid(2);
                    sys.settings.skip_mode = self.value as u8;
                    sys.menu_sound(1);
                }
                if ok(sys) {
                    self.windows.cursor_hide(SKIP_MODE, 0x3C, 10);
                    self.windows.cursor_show(GAME, 0x7F, 10);
                    self.sub = 0;
                    sys.lock = 0xF;
                    sys.menu_sound(0);
                }
            }
            _ => {
                if sys.menu_key(0x21) {
                    match self.entry {
                        0 => {
                            self.windows.cursor_hide(GAME, 0x3C, 10);
                            self.windows.cursor_show(SKIP_MODE, 0x7F, 10);
                            self.value = i32::from(sys.settings.skip_mode);
                            self.sub = 10;
                        }
                        2 => {
                            self.windows.cursor_hide(GAME, 0x3C, 10);
                            self.windows.cursor_show(VIBRATION, 0x7F, 10);
                            self.value = (i32::from(sys.settings.vibration) + 1) & 1;
                            self.sub = 0x1E;
                        }
                        3 => {
                            self.windows.cursor_hide(GAME, 0x3C, 10);
                            self.windows.cursor_make(WINDOW, (0x96, 0xC3), 0xE9, 0x7F, 0xF);
                            self.sub = 0x28;
                            self.entry = 0;
                        }
                        // La vitesse du texte se règle sur l'entrée : ○ n'y
                        // fait rien.
                        4 => return,
                        5 => {
                            self.windows.cursor_hide(GAME, 0x3C, 10);
                            self.open_skip(sys, pen);
                            self.sub = 0x32;
                            self.entry = 0;
                            self.set_gauges(3);
                        }
                        _ => self.back_to_main(),
                    }
                    sys.lock = 0xF;
                    sys.menu_sound(2);
                } else if up(sys, 8, 5) {
                    self.entry -= 1;
                    if self.entry == 1 {
                        self.entry = 0;
                    }
                } else if down(sys, 8, 5) {
                    self.entry += 1;
                    if self.entry == 1 {
                        self.entry = 2;
                    }
                } else if sys.menu_key(0x20) {
                    self.back_to_main();
                    sys.menu_sound(0);
                    self.windows.cursor_del(GAME, 10);
                    sys.lock = 0xF;
                    return;
                } else if self.entry == 4 {
                    let step = sideways(sys, 8, 5);
                    nudge(&mut sys.settings.text_speed, step, 10);
                }
                if self.entry != self.seen[3] {
                    sys.menu_sound(1);
                    self.entry = (self.entry + 7) % 7;
                }
            }
        }
    }

    /// La fenêtre de jeu se ferme, retour au menu des options.
    fn back_to_main(&mut self) {
        self.level = 0;
        self.close_game();
        self.entry = 0;
        self.value = 0;
        self.windows.cursor_show(MAIN, 0x7F, 10);
        self.set_gauges(1);
    }

    /// Un mode de l'avance rapide (sous-états 0x3c, 0x46, 0x50) : sa vitesse
    /// (entrée 0) et ses conditions d'arrêt (entrée 1, la valeur choisit la
    /// case).
    fn skip_detail(&mut self, sys: &mut Sys, pen: &mut Pen) {
        let k = ((self.sub - 0x3C) / 10) as usize;
        if up(sys, 8, 5) || down(sys, 8, 5) {
            self.entry = (self.entry + 1) & 1;
        } else {
            let step = sideways(sys, 8, 5);
            if self.entry == 0 {
                nudge(&mut sys.settings.skip[k][0], step, 10);
            } else {
                self.value += step;
            }
        }
        if self.value != self.seen[2] || self.entry != self.seen[3] {
            self.entry = self.entry.rem_euclid(2);
            self.value = if k == 0 { self.value.rem_euclid(2) } else { (self.value + 3) % 3 };
            sys.menu_sound(1);
        }
        if sys.menu_key(0x21) && self.entry == 1 {
            // L'avance forcée n'a que deux cases : un choix (1) et un point
            // d'affection (4).
            let bit = match (k, self.value) {
                (0, 1) | (_, 2) => 4,
                (0, _) | (_, 0) => 1,
                _ => 2,
            };
            sys.settings.skip[k][1] ^= bit;
            sys.menu_sound(2);
            let glyphs = pen.glyphs(&skip_text(pen, &sys.settings));
            self.windows.set_text(SKIP, glyphs);
            self.cursor_proc(sys, pen);
        } else if (sys.menu_key(0x21) && self.entry == 0) || sys.menu_key(0x20) {
            self.value = 0;
            self.entry = k as i32;
            self.windows.cursor_del(SKIP_DETAIL, 10);
            self.windows.cursor_show(SKIP, 0x7F, 10);
            self.sub = 0x32;
            sys.menu_sound(0);
        }
    }

    /// Niveau 2, le son : musique, bruitages, voix, retour. Les bruitages
    /// font entendre leur volume tous les 4 pas ; la musique et les voix
    /// prennent le leur en sortant (la musique s'arrête et repart).
    fn sound(&mut self, sys: &mut Sys) {
        match self.sub {
            0x14 => {
                if sys.lock < 1 {
                    sys.restart_music(false);
                    self.level = 0;
                    self.close(SOUND);
                    self.windows.cursor_del(SOUND, 0xF);
                    self.entry = 1;
                    self.value = 0;
                    self.windows.cursor_del(SOUND, 10);
                    self.windows.cursor_show(MAIN, 0x7F, 10);
                    self.set_gauges(1);
                    self.sub = 0;
                } else {
                    sys.lock -= 1;
                }
            }
            // La musique s'est arrêtée (aussitôt dans le portage).
            10 => {
                sys.apply_volumes();
                self.sub = 0x14;
                sys.lock = 10;
            }
            _ => {
                if up(sys, 8, 5) {
                    self.entry -= 1;
                } else if down(sys, 8, 5) {
                    self.entry += 1;
                } else if (sys.menu_key(0x21) && self.entry == 3) || sys.menu_key(0x20) {
                    if sys.music().is_some() {
                        sys.pause_music();
                        self.sub = 10;
                    } else {
                        self.sub = 0x14;
                    }
                    sys.lock = 0xF;
                    sys.menu_sound(0);
                    return;
                } else if self.entry != 3 {
                    let k = self.entry.clamp(0, 2) as usize;
                    let step = sideways(sys, 8, 1);
                    nudge(&mut sys.settings.volume[k], step, 0x7F);
                    if (left(sys, 8, 4) || right(sys, 8, 4)) && k == 1 {
                        sys.set_effect_volume(sys.settings.volume[1]);
                        sys.menu_sound(1);
                    }
                }
                if self.entry != self.seen[3] {
                    sys.menu_sound(1);
                    self.entry = self.entry.rem_euclid(4);
                }
            }
        }
    }

    /// AD_OptionCursorProc : le curseur va sur l'entrée, l'explication
    /// change.
    fn cursor_proc(&mut self, sys: &Sys, pen: &mut Pen) {
        let s = &sys.settings;
        let info: Vec<u8>;
        match self.level {
            2 => {
                let y = if self.entry == 3 { 0xAA } else { self.entry * 0x1E + 0x46 };
                self.windows.cursor_move(SOUND, (0xA5, y), 0x7F, 5);
                info = pen.font.string(text::SOUND_INFO[self.entry.clamp(0, 3) as usize]).to_vec();
            }
            1 if (0x3C..=0x50).contains(&self.sub) => {
                let k = (self.sub - 0x3C) / 10;
                let at = if self.entry == 0 {
                    info = pen.font.string(text::SPEED_INFO[k as usize]).to_vec();
                    (0xD4, k * 0x5A + 0x68)
                } else {
                    // La case : pour l'avance forcée, la seconde est celle
                    // du point d'affection.
                    let (case, x) = if k == 0 {
                        (if self.value == 1 { 2 } else { 0 }, self.value * 0x53 + 0x124)
                    } else {
                        (self.value, [0x124, 0x177, 0x1CA][self.value.clamp(0, 2) as usize])
                    };
                    let on = s.skip[k as usize][1] & (1 << case) != 0;
                    info = pen.join(&[
                        text::STOP[case.clamp(0, 2) as usize],
                        if on { text::AT } else { text::AFTER },
                        if k == 2 { text::AUTO } else { text::SKIPPING },
                        if on { text::STOPS } else { text::GOES_ON },
                    ]);
                    (x, k * 0x5A + 0x81)
                };
                self.windows.cursor_move(SKIP_DETAIL, at, 0x7F, 5);
            }
            1 => {
                let (name, at, line) = match self.sub {
                    0x32 => (SKIP, (0xBC, self.entry * 0x5A + 0x4C), if self.entry == 3 { text::BACK } else { text::SKIP_INFO[self.entry.clamp(0, 2) as usize] }),
                    0x28 => (WINDOW, (0x96, self.entry * 0x1E + 200), text::WINDOW_INFO[(self.entry & 1) as usize]),
                    0x1E => (VIBRATION, (self.value * 0x46 + 0x159, 0x88), text::VIBRATION_INFO[(self.value & 1) as usize]),
                    10 => (SKIP_MODE, (self.value * 0x5A + 0x164, 0x41), text::SKIP_MODE_INFO[(self.value & 1) as usize]),
                    _ => {
                        let k = self.entry.clamp(0, 6) as usize;
                        (GAME, (0x7D, GAME_ROWS[k]), text::GAME_INFO[k])
                    }
                };
                self.windows.cursor_move(name, at, 0x7F, 5);
                info = pen.font.string(line).to_vec();
            }
            _ => {
                let (at, line) = match self.entry {
                    3 if self.value == 1 => ((0xA8, 0xC3), text::CANCEL_INFO),
                    3 => ((0x46, 0xC3), text::KEEP_INFO),
                    2 => ((0x46, 0x99), text::MAIN_INFO[2]),
                    1 => ((0x46, 0x52), text::MAIN_INFO[1]),
                    _ => ((0x46, 0x34), text::MAIN_INFO[0]),
                };
                self.windows.cursor_move(MAIN, at, 0x7F, 5);
                info = pen.font.string(line).to_vec();
            }
        }
        let glyphs = pen.glyphs(&info);
        self.windows.set_text(INFO, glyphs);
    }

    /// AD_OptionGameWinMake : la fenêtre de jeu, son curseur, et ceux du
    /// mode d'avance rapide et de la vibration, estompés sur leur valeur.
    fn open_game(&mut self, sys: &Sys, pen: &mut Pen) {
        let glyphs = pen.glyphs(&pen.join(&text::GAME));
        self.open(GAME, 1, 0xE8, (0x23, 0xF), glyphs, GAME_WIN);
        let s = &sys.settings;
        self.windows.cursor_make(GAME, (0x7D, 0x41), 0xE9, 0x7F, 0xF);
        self.windows.cursor_make(SKIP_MODE, (i32::from(s.skip_mode) * 0x5A + 0x164, 0x41), 0xE9, 100, 0xF);
        self.windows.cursor_make(VIBRATION, (((i32::from(s.vibration) + 1) & 1) * 0x46 + 0x159, 0x88), 0xE9, 100, 0xF);
        self.windows.cursor_hide(SKIP_MODE, 0x3C, 1);
        self.windows.cursor_hide(VIBRATION, 0x3C, 1);
    }

    /// AD_OptionGameWinDel.
    fn close_game(&mut self) {
        self.close(GAME);
        for name in [GAME, SKIP_MODE, VIBRATION] {
            self.windows.cursor_del(name, 0xF);
        }
    }

    /// AD_OptionSkipWinMake : la fenêtre de l'avance rapide (texte à
    /// l'échelle 0,9).
    fn open_skip(&mut self, sys: &Sys, pen: &mut Pen) {
        let glyphs = pen.glyphs(&skip_text(pen, &sys.settings));
        self.open(SKIP, 1, 0xEA, (0x28, 0xF), glyphs, SKIP_WIN);
        self.windows.cursor_make(SKIP, (0xB9, 0x49), 0xEB, 0x7F, 0xF);
    }

    /// AD_OptionSoundWinMake.
    fn open_sound(&mut self, pen: &mut Pen) {
        let glyphs = pen.glyphs(&pen.join(&text::SOUND));
        self.open(SOUND, 1, 0xE8, (0x23, 0xF), glyphs, SOUND_WIN);
        self.windows.cursor_make(SOUND, (0xA0, 0x46), 0xE9, 0x7F, 0xF);
    }

    /// Les zones de la souris de l'écran ouvert : chaque entrée (une ligne
    /// à la hauteur de son curseur, bornée par sa fenêtre) ou valeur (autour
    /// de la place de son curseur) ; ce qu'elle choisit : l'entrée, et la
    /// valeur s'il y en a une.
    fn zones(&self) -> Vec<Zone<(i32, Option<i32>)>> {
        // Une fenêtre (centre, taille), comme à son ouverture : ses bords.
        let span = |[x, _, w, _]: [i32; 4]| ((x - w / 2) as f32, w as f32);
        let row = |win: [i32; 4], y: i32, h: i32, target| {
            let (left, w) = span(win);
            Zone::new([left, (y - h / 2) as f32, w, h as f32], target)
        };
        let column = |x: i32, y: i32, w: i32, h: i32, target| Zone::new([(x - 10) as f32, (y - h / 2) as f32, w as f32, h as f32], target);
        match (self.level, self.sub) {
            (0, 0) => {
                let (left, w) = span(OPTION_WIN);
                let mut z: Vec<_> = [(0, 0x34), (1, 0x52), (2, 0x99)].into_iter().map(|(k, y)| row(OPTION_WIN, y, 28, (k, None))).collect();
                // 決定 et キャンセル, côte à côte : la ligne coupée entre les deux.
                let (top, cut) = ((0xC3 - 14) as f32, 0x96 as f32);
                z.push(Zone::new([left, top, cut - left, 28.0], (3, Some(0))));
                z.push(Zone::new([cut, top, left + w - cut, 28.0], (3, Some(1))));
                z
            }
            // Le jeu saute l'entrée 1.
            (1, 0) => [0, 2, 3, 4, 5, 6].into_iter().map(|k| row(GAME_WIN, GAME_ROWS[k as usize], 28, (k, None))).collect(),
            (1, 10) => (0..2).map(|v| column(v * 0x5A + 0x164, 0x41, 0x5A, 28, (self.entry, Some(v)))).collect(),
            (1, 0x1E) => (0..2).map(|v| column(v * 0x46 + 0x159, 0x88, 0x46, 28, (self.entry, Some(v)))).collect(),
            (1, 0x28) => (0..2).map(|k| row(GAME_WIN, k * 0x1E + 200, 28, (k, None))).collect(),
            (1, 0x32) => (0..4).map(|k| row(SKIP_WIN, k * 0x5A + 0x4C, 28, (k, None))).collect(),
            (1, sub) if (0x3C..=0x50).contains(&sub) => {
                let k = (sub - 0x3C) / 10;
                let values: &[i32] = if k == 0 { &[0x124, 0x177] } else { &[0x124, 0x177, 0x1CA] };
                let mut z = vec![row(SKIP_WIN, k * 0x5A + 0x68, 24, (0, None))];
                z.extend(values.iter().enumerate().map(|(v, &x)| column(x, k * 0x5A + 0x81, 0x53, 24, (1, Some(v as i32)))));
                z
            }
            (2, 0) => [0x46, 0x64, 0x82, 0xAA].into_iter().enumerate().map(|(k, y)| row(SOUND_WIN, y, 28, (k as i32, None))).collect(),
            _ => Vec::new(),
        }
    }

    /// La souris : l'entrée (ou la valeur) survolée est choisie, comme avec
    /// les flèches.
    fn hover(&mut self, sys: &mut Sys) {
        if let Some((entry, value)) = pointer::pick(sys, &self.zones()).hovered {
            self.entry = entry;
            if let Some(v) = value {
                self.value = v;
            }
        }
    }

    /// Un clic : vrai s'il vaut ○, sur une entrée ou une valeur. Sur une
    /// jauge de l'écran ouvert, il règle sa valeur (et ne vaut pas ○) ;
    /// ailleurs il ne fait rien.
    pub fn click(&mut self, sys: &mut Sys, (x, y): (f32, f32)) -> bool {
        if self.outer != Outer::Running {
            return false;
        }
        if !self.set_gauge(sys, (x, y)) {
            return pointer::pick(sys, &self.zones()).clicked.is_some();
        }
        false
    }

    /// Un clic sur une jauge de l'écran ouvert règle sa valeur ; vrai s'il
    /// en a touché une.
    fn set_gauge(&mut self, sys: &mut Sys, (x, y): (f32, f32)) -> bool {
        let detail = (0x3C..=0x50).contains(&self.sub);
        let bars: Vec<(Bar, Setting)> = match (self.level, self.sub, self.gauges.0) {
            (2, 0, 4) => (0..3).map(|k| (SOUND_BARS[k], Setting::Volume(k))).collect(),
            (1, 0 | 0x28, 2) => vec![(GAME_BARS[0], Setting::Window(0)), (GAME_BARS[1], Setting::Window(1)), (GAME_BARS[2], Setting::TextSpeed)],
            (1, sub, 3) if sub == 0x32 || detail => (0..3).map(|k| (SKIP_BARS[k], Setting::SkipSpeed(k))).collect(),
            _ => Vec::new(),
        };
        let inside = |&&((bx, by, bw, bh), _): &&(Bar, Setting)| x >= bx as f32 && x < (bx + bw) as f32 && y >= by as f32 && y < (by + bh) as f32;
        let Some(&((bx, _, bw, _), setting)) = bars.iter().find(inside) else {
            return false;
        };
        let fill = ((x - (bx + 4) as f32) / (bw - 10) as f32).clamp(0.0, 1.0);
        let set = |max: u8| (fill * f32::from(max)).round() as u8;
        let s = &mut sys.settings;
        match setting {
            Setting::Volume(k) => {
                s.volume[k] = set(0x7F);
                self.entry = k as i32;
                if k == 1 {
                    sys.set_effect_volume(sys.settings.volume[1]);
                    sys.menu_sound(1);
                }
            }
            Setting::Window(k) => {
                s.window[k] = set(0x7F);
                self.entry = if self.sub == 0x28 { k as i32 } else { 3 };
            }
            Setting::TextSpeed => {
                if self.sub == 0 {
                    s.text_speed = set(10);
                    self.entry = 4;
                }
            }
            Setting::SkipSpeed(k) => s.skip[k][0] = set(10),
        }
        true
    }

    /// Les fenêtres et les curseurs, puis AD_OptionTran (les jauges,
    /// AD_AdvWinBar2, à la priorité de leur fenêtre mais par-dessus).
    pub fn draw(&self, gfx: &mut Gfx, sys: &Sys, atlas: Option<&Atlas>) {
        self.windows.draw(gfx, atlas);
        let s = &sys.settings;
        let bars: Vec<(Bar, i32)> = match self.gauges.0 {
            4 => (0..3).map(|k| (SOUND_BARS[k], i32::from(s.volume[k]) * 100 / 0x7F)).collect(),
            3 => (0..3).map(|k| (SKIP_BARS[k], i32::from(s.skip[k][0]) * 100 / 10)).collect(),
            2 => vec![
                (GAME_BARS[0], i32::from(s.window[0]) * 100 / 0x7F),
                (GAME_BARS[1], i32::from(s.window[1]) * 100 / 0x7F),
                (GAME_BARS[2], i32::from(s.text_speed) * 100 / 10),
            ],
            _ => Vec::new(),
        };
        let prio = if self.gauges.0 == 3 { 0xEA } else { 0xE8 };
        for ((x, y, w, h), percent) in bars {
            // Le cadre, puis la barre (sa largeur moins 10, au pourcentage).
            gfx.tran(crate::dialog::CURSOR, [4.0, 34.0, 226.0, 16.0], [x as f32, y as f32, w as f32, h as f32], prio, 0x7F, Blend::Normal, GREY);
            let fill = ((w - 10) * percent / 100) as f32;
            gfx.tran(crate::dialog::CURSOR, [9.0, 55.0, 216.0, 6.0], [(x + 4) as f32, (y + 5) as f32, fill, (h - 11) as f32], prio + 1, 0x7F, Blend::Normal, GREY);
        }
    }
}

/// Une jauge : coin, largeur, hauteur.
type Bar = (i32, i32, i32, i32);
const SOUND_BARS: [Bar; 3] = [(300, 0x3D, 0xA5, 0x16), (300, 0x5B, 0xA5, 0x16), (300, 0x79, 0xA5, 0x16)];
const SKIP_BARS: [Bar; 3] = [(0x170, 0x5C, 0xB4, 0x14), (0x170, 0xB6, 0xB4, 0x14), (0x170, 0x110, 0xB4, 0x14)];
/// Les opacités du cadre et du fond, la vitesse du texte.
const GAME_BARS: [Bar; 3] = [(0x136, 0xBB, 0xA4, 0x18), (0x136, 0xD9, 0xA4, 0x18), (0x136, 0xFD, 0xA4, 0x18)];

/// Ce que règle une jauge.
#[derive(Clone, Copy)]
enum Setting {
    Volume(usize),
    Window(usize),
    TextSpeed,
    SkipSpeed(usize),
}

/// AD_InputDirect : une direction, à l'appui, puis tenue plus de `delay`
/// images, toutes les `every` images.
fn up(sys: &Sys, delay: u32, every: u32) -> bool {
    sys.input.direct(0, delay, every, true)
}

fn down(sys: &Sys, delay: u32, every: u32) -> bool {
    sys.input.direct(1, delay, every, true)
}

fn left(sys: &Sys, delay: u32, every: u32) -> bool {
    sys.input.direct(2, delay, every, true)
}

fn right(sys: &Sys, delay: u32, every: u32) -> bool {
    sys.input.direct(3, delay, every, true)
}

/// ← (−1) ou → (1), ← d'abord.
fn sideways(sys: &Sys, delay: u32, every: u32) -> i32 {
    if left(sys, delay, every) {
        -1
    } else if right(sys, delay, every) {
        1
    } else {
        0
    }
}

/// Un réglage avance d'un pas, entre 0 et `max`.
fn nudge(v: &mut u8, step: i32, max: u8) {
    if step < 0 && *v > 0 {
        *v -= 1;
    } else if step > 0 && *v < max {
        *v += 1;
    }
}

/// AD_OptionSkipTxtSet : la fenêtre de l'avance rapide ; chaque case est
/// en palette 0 si elle est cochée, 6 sinon.
fn skip_text(pen: &Pen, s: &Settings) -> Vec<u8> {
    let font = pen.font;
    let color = |bits: u8, bit: u8| (1 - i32::from(bits & bit != 0)) * 6;
    let [[_, forced], [_, read], [_, auto]] = s.skip;
    let mut out = b"#Scale[0.9]".to_vec();
    out.extend(pen.join(&[text::SKIP_TITLE, text::SKIP_SPEED, text::SKIP_STOP]));
    out.extend(sprintf(font.string(text::FORCED_BOXES), &[color(forced, 1), color(forced, 4)]));
    out.extend(pen.join(&[text::READ_TITLE, text::SKIP_SPEED, text::SKIP_STOP]));
    out.extend(sprintf(font.string(text::BOXES), &[color(read, 1), color(read, 2), color(read, 4)]));
    out.extend(pen.join(&[text::AUTO_TITLE, text::SKIP_SPEED, text::SKIP_STOP]));
    out.extend(sprintf(font.string(text::BOXES), &[color(auto, 1), color(auto, 2), color(auto, 4)]));
    out.extend_from_slice(font.string(text::SKIP_BACK));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les octets gardés redonnent les mêmes réglages.
    #[test]
    fn bytes_round_trip() {
        let s = Settings { volume: [1, 2, 3], skip_mode: 1, effect_cut: 0, vibration: 0, text_speed: 9, window: [10, 20], skip: [[1, 2], [3, 4], [5, 6]] };
        assert_eq!(Settings::from_bytes(s.bytes()), s);
        assert_eq!(Settings::from_bytes(Settings::default().bytes()), Settings::default());
    }

}
