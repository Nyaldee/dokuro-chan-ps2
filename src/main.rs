//! Les mini-jeux de « Game ni Natta yo! Dokuro-chan » (PS2) : portage PC non
//! officiel.
//!
//! L'ISO originale du joueur est lue au lancement (images, sons, voix,
//! musique) ; la logique du mini-jeu et de son moteur, réécrite en Rust
//! d'après l'exécutable (SLPM_661.86), tourne à 60 images par seconde sur
//! l'écran de la PS2 (640 × 448, affiché en 4:3).

#![windows_subsystem = "windows"]

mod adv;
mod album;
mod audio;
mod backlog;
mod bat;
mod bread;
mod card;
mod cglib;
mod catch;
mod data;
mod dialog;
mod engine;
mod font;
mod game;
mod gfx;
mod mainmenu;
mod map;
mod mcs;
mod mini;
mod minigame;
mod mogura;
mod movie;
mod options;
mod original;
mod pad;
mod readmode;
mod record;
mod platform;
mod pointer;
mod render;
mod result;
mod rumble;
mod run;
mod save;
mod savedata;
mod scene;
mod shake;
mod stcm2;
mod story;
mod sys;
mod title;

use std::sync::Arc;

use miniquad::{EventHandler, KeyCode, KeyMods, MouseButton, conf, window};

use audio::Audio;
use engine::{BUTTONS, Button, SCREEN};
use game::Game;
use original::Iso;
use pad::{Buttons, Pads};
use render::Renderer;
use save::Save;

/// Une image du jeu (60 Hz).
const FRAME: f64 = 1.0 / 60.0;
/// Le pointeur se cache après ce temps sans bouger la souris (secondes).
const CURSOR_IDLE: f64 = 3.0;
/// Rattrapage maximal après un blocage, en secondes.
const MAX_CATCH_UP: f64 = 0.25;
/// L'écran de la PS2 est affiché en 4:3.
const ASPECT: f32 = 4.0 / 3.0;

/// Le mode d'affichage, dans la sauvegarde.
const DISPLAY_SETTING: &str = "displaysetting";
const BORDERLESS: &str = "borderless";
const WINDOW: &str = "window";

struct App {
    renderer: Renderer,
    game: Game,
    save: Save,
    pads: Pads,
    pad: Buttons,
    /// Les moteurs des manettes tels qu'envoyés : le petit, le gros.
    motors: (bool, u8),
    /// Touches du clavier tenues, en boutons de la PS2.
    keys: [bool; BUTTONS],
    /// Le bouton droit de la souris (×) ; un clic gauche à traiter (○ ou
    /// le choix d'un portrait).
    right: bool,
    /// La molette, à traiter : un cran vers le haut, vers le bas.
    wheel: (bool, bool),
    click: Option<(f32, f32)>,
    /// Où la souris vient d'aller (points de l'écran), à traiter.
    hover: Option<(f32, f32)>,
    /// Où est la souris (points de l'écran), et quand a-t-elle servi pour
    /// la dernière fois (−∞ : le clavier ou la manette ont repris la main).
    pointer: (f32, f32),
    moved: f64,
    last: f64,
    pending: f64,
    borderless: bool,
    test: Option<Test>,
}

/// Tests (debug) : DOKURO_SNAPSHOT=<images>:<fichier.png> joue ce nombre
/// d'images, enregistre la fenêtre en PNG et quitte ; DOKURO_INPUT=
/// <image>[-<image>]:<bouton>;… appuie une image, ou tient de l'une à
/// l'autre (circle, cross, square, triangle, up, down, left, right, l1, r1,
/// l2, r2, start, ctrl, auto), ou bouge la souris (mouse@x,y) ou clique
/// (click@x,y) ; DOKURO_EVERY=<n>[@<image>] enregistre aussi une image sur
/// n (à partir de cette image). Sans son ; la sauvegarde est lue, jamais
/// écrite.
struct Test {
    frame: u32,
    frames: u32,
    every: u32,
    from: u32,
    path: String,
    events: Vec<(u32, String)>,
}

impl Test {
    fn from_env() -> Option<Test> {
        if !cfg!(debug_assertions) {
            return None;
        }
        let spec = std::env::var("DOKURO_SNAPSHOT").ok()?;
        let (frames, path) = spec.split_once(':')?;
        let events = std::env::var("DOKURO_INPUT")
            .unwrap_or_default()
            .split(';')
            .filter_map(|e| {
                let (frames, button) = e.split_once(':')?;
                let (from, to) = frames.split_once('-').unwrap_or((frames, frames));
                Some((from.parse::<u32>().ok()?..=to.parse().ok()?).map(move |f| (f, button.to_string())))
            })
            .flatten()
            .collect();
        let every = std::env::var("DOKURO_EVERY").unwrap_or_default();
        let (every, from) = every.split_once('@').unwrap_or((&every, "0"));
        Some(Test { frame: 0, frames: frames.parse().unwrap_or(1), every: every.parse().unwrap_or(0), from: from.parse().unwrap_or(0), path: path.to_string(), events })
    }
}

impl App {
    fn new(iso: Arc<Iso>, save: Save) -> App {
        let test = Test::from_env();
        let audio = if test.is_some() { Audio::silent() } else { Audio::new() };
        let mut renderer = Renderer::new(SCREEN);
        let seed = if test.is_some() { 1 } else { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64) };
        // Les vidéos du démarrage, sauf en test (DOKURO_BOOT les garde).
        let boot = test.is_none() || std::env::var_os("DOKURO_BOOT").is_some();
        let game = Game::new(iso, audio, &mut renderer, seed, record::Record::read(&save), boot);
        // La taille d'ouverture, que retrouve la fenêtre en quittant le
        // plein écran.
        platform::remember_window();
        // Sur l'écran du jeu, la flèche du système laisse place au pointeur
        // du portage (Game::draw_pointer).
        window::show_mouse(false);
        let borderless = test.is_none() && save.get(DISPLAY_SETTING) == BORDERLESS;
        if borderless {
            platform::set_borderless(true);
        }
        App { renderer, game, save, pads: Pads::new(), pad: Buttons::default(), motors: (false, 0), keys: [false; BUTTONS], right: false, wheel: (false, false), click: None, hover: None, pointer: (0.0, 0.0), moved: f64::NEG_INFINITY, last: miniquad::date::now(), pending: 0.0, borderless, test }
    }

    /// L'écran dans la fenêtre : en 4:3, agrandi pour tenir, centré.
    fn viewport(&self) -> (i32, i32, i32, i32) {
        let (ww, wh) = window::screen_size();
        let (w, h) = if ww / wh > ASPECT { (wh * ASPECT, wh) } else { (ww, ww / ASPECT) };
        (((ww - w) / 2.0) as i32, ((wh - h) / 2.0) as i32, w as i32, h as i32)
    }

    /// Un point de la fenêtre sur l'écran de la PS2.
    fn to_screen(&self, x: f32, y: f32) -> (f32, f32) {
        let (vx, vy, vw, vh) = self.viewport();
        let (_, wh) = window::screen_size();
        let top = wh - (vy + vh) as f32;
        ((x - vx as f32) / vw as f32 * SCREEN.0, (y - top) / vh as f32 * SCREEN.1)
    }

    /// Les boutons de la PS2 tenus : manettes, clavier, souris.
    fn buttons(&mut self) -> [bool; BUTTONS] {
        let pad = self.pads.poll();
        let old = self.pad;
        self.pad = pad;
        if pad != old {
            self.moved = f64::NEG_INFINITY;
        }
        if pad.start && pad.select && !(old.start && old.select) {
            window::order_quit();
        }
        let mut b = self.keys;
        let mut hold = |button: Button, on: bool| b[button as usize] |= on;
        hold(Button::Circle, pad.a);
        hold(Button::Cross, pad.b || self.right);
        hold(Button::Square, pad.x);
        hold(Button::Triangle, pad.y);
        hold(Button::L1, pad.l1);
        hold(Button::R1, pad.r1);
        hold(Button::L2, pad.l2);
        hold(Button::R2, pad.r2);
        hold(Button::Start, pad.start);
        hold(Button::Up, pad.up);
        hold(Button::Down, pad.down);
        hold(Button::Left, pad.left);
        hold(Button::Right, pad.right);
        b
    }

    fn key_button(key: KeyCode) -> Option<Button> {
        Some(match key {
            KeyCode::Enter | KeyCode::KpEnter | KeyCode::Z => Button::Circle,
            KeyCode::Escape => Button::Menu,
            KeyCode::Backspace | KeyCode::X => Button::Cross,
            KeyCode::C => Button::Square,
            KeyCode::V => Button::Triangle,
            KeyCode::Q => Button::L1,
            KeyCode::W => Button::R1,
            KeyCode::A => Button::Auto,
            KeyCode::S => Button::R2,
            KeyCode::Up => Button::Up,
            KeyCode::Down => Button::Down,
            KeyCode::Left => Button::Left,
            KeyCode::Right => Button::Right,
            KeyCode::Space => Button::Jump,
            KeyCode::LeftControl | KeyCode::RightControl => Button::Slide,
            _ => return None,
        })
    }
}

impl EventHandler for App {
    fn update(&mut self) {
        if let Some(test) = &mut self.test {
            let mut b = [false; BUTTONS];
            let (mut click, mut hover) = (None, None);
            for (_, name) in test.events.iter().filter(|e| e.0 == test.frame) {
                // « mouse@x,y » déplace la souris, « click@x,y » clique
                // (points de l'écran).
                if let Some((what, at)) = name.split_once('@') {
                    let at = at.split_once(',').and_then(|(x, y)| Some((x.parse().ok()?, y.parse().ok()?)));
                    if what == "click" {
                        click = at;
                    } else {
                        hover = at;
                    }
                    continue;
                }
                let button = match name.as_str() {
                    "circle" => Button::Circle,
                    "cross" => Button::Cross,
                    "square" => Button::Square,
                    "triangle" => Button::Triangle,
                    "up" => Button::Up,
                    "down" => Button::Down,
                    "left" => Button::Left,
                    "l1" => Button::L1,
                    "r1" => Button::R1,
                    "l2" => Button::L2,
                    "r2" => Button::R2,
                    "start" => Button::Start,
                    "ctrl" => Button::Slide,
                    "auto" => Button::Auto,
                    _ => Button::Right,
                };
                b[button as usize] = true;
            }
            test.frame += 1;
            self.game.step(b, click, hover);
            return;
        }
        let now = miniquad::date::now();
        self.pending = (self.pending + now - self.last).min(MAX_CATCH_UP);
        self.last = now;
        let buttons = self.buttons();
        while self.pending >= FRAME {
            self.pending -= FRAME;
            // Un cran de molette : un appui, le temps d'une image.
            let mut b = buttons;
            let (up, down) = std::mem::take(&mut self.wheel);
            b[Button::WheelUp as usize] |= up;
            b[Button::WheelDown as usize] |= down;
            self.game.step(b, self.click.take(), self.hover.take());
        }
        if let Some(record) = self.game.take_record() {
            record.write(&mut self.save);
        }
        let motors = self.game.sys.rumble.motors();
        if motors != self.motors {
            self.pads.rumble(motors.0, motors.1);
            self.motors = motors;
        }
        // Le pointeur ne se montre que si la souris sert : il se cache dès
        // qu'une touche ou la manette reprend la main, ou après quelques
        // secondes sans bouger.
        self.game.pointer = (now - self.moved < CURSOR_IDLE).then_some(self.pointer);
    }

    fn draw(&mut self) {
        self.game.gfx.upload(&mut self.renderer);
        self.renderer.begin(self.viewport());
        self.game.draw();
        self.game.gfx.render(&mut self.renderer);
        self.renderer.finish();
        if let Some(test) = &self.test {
            let (w, h) = window::screen_size();
            let shot = test.frame >= test.frames || (test.every > 0 && test.frame >= test.from && test.frame % test.every == 0);
            if shot {
                let pixels = self.renderer.read_pixels(w as i32, h as i32);
                let path = if test.frame >= test.frames { test.path.clone() } else { format!("{}_{}.png", test.path.trim_end_matches(".png"), test.frame) };
                let _ = std::fs::write(path, png(w as u32, h as u32, &pixels));
            }
            if test.frame >= test.frames {
                std::process::exit(0);
            }
        }
        self.renderer.commit();
    }

    /// La souris : un clic gauche pour ○ (ou le choix d'un portrait), le
    /// bouton droit pour ×.
    fn mouse_button_down_event(&mut self, button: MouseButton, x: f32, y: f32) {
        self.moved = miniquad::date::now();
        match button {
            MouseButton::Left => self.click = Some(self.to_screen(x, y)),
            MouseButton::Right => self.right = true,
            _ => {}
        }
    }

    fn mouse_wheel_event(&mut self, _x: f32, y: f32) {
        self.moved = miniquad::date::now();
        if y > 0.0 {
            self.wheel.0 = true;
        } else if y < 0.0 {
            self.wheel.1 = true;
        }
    }

    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        self.moved = miniquad::date::now();
        self.pointer = self.to_screen(x, y);
        self.hover = Some(self.to_screen(x, y));
    }

    fn mouse_button_up_event(&mut self, button: MouseButton, _x: f32, _y: f32) {
        if button == MouseButton::Right {
            self.right = false;
        }
    }

    /// Alt+Entrée ou F11 : fenêtre ou plein écran sans bordure. Le clavier
    /// fait aussi la manette.
    fn key_down_event(&mut self, key: KeyCode, mods: KeyMods, repeat: bool) {
        if repeat {
            return;
        }
        self.moved = f64::NEG_INFINITY;
        if key == KeyCode::Enter && mods.alt || key == KeyCode::F11 {
            self.borderless = !self.borderless;
            platform::set_borderless(self.borderless);
            self.save.set(DISPLAY_SETTING, if self.borderless { BORDERLESS } else { WINDOW });
            return;
        }
        if let Some(b) = App::key_button(key) {
            self.keys[b as usize] = true;
            // Espace est aussi START (hors de la course, où il saute).
            if key == KeyCode::Space {
                self.keys[Button::Start as usize] = true;
            }
        }
    }

    fn key_up_event(&mut self, key: KeyCode, _mods: KeyMods) {
        if let Some(b) = App::key_button(key) {
            self.keys[b as usize] = false;
            if key == KeyCode::Space {
                self.keys[Button::Start as usize] = false;
            }
        }
    }
}

/// Un PNG RGBA (captures de test).
fn png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut raw = Vec::with_capacity(rgba.len() + height as usize);
    for line in rgba.chunks_exact(width as usize * 4) {
        raw.push(0);
        raw.extend_from_slice(line);
    }
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut chunk = |kind: &[u8], data: &[u8]| {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let start = out.len();
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        let crc = original::crc32(&out[start..]);
        out.extend_from_slice(&crc.to_be_bytes());
    };
    let mut header = Vec::new();
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(b"IHDR", &header);
    chunk(b"IDAT", &miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6));
    chunk(b"IEND", &[]);
    out
}

fn main() {
    let testing = cfg!(debug_assertions) && std::env::var_os("DOKURO_SNAPSHOT").is_some();
    let mut save = if testing { Save::memory() } else { Save::load() };
    let (path, checked) = save.assets();
    let iso = match Iso::load(path, checked) {
        Ok((iso, stamp)) => {
            save.set_assets_checked(&stamp);
            Arc::new(iso)
        }
        Err(message) => {
            message_box(&format!("{message}\n\nIts path can also be set in \"assets\" > \"path\" of {}.", save.file()));
            return;
        }
    };
    let conf = conf::Conf {
        window_title: "ゲームになったよ！ ドクロちゃん ～健康診断大作戦～".to_string(),
        window_width: 960,
        window_height: 720,
        window_resizable: true,
        high_dpi: true,
        sample_count: 4,
        ..Default::default()
    };
    miniquad::start(conf, move || Box::new(App::new(iso, save)));
}

/// Une boîte de message Windows (le jeu ne peut pas démarrer).
fn message_box(text: &str) {
    #[cfg(windows)]
    {
        #[link(name = "user32")]
        unsafe extern "system" {
            fn MessageBoxW(wnd: *mut core::ffi::c_void, text: *const u16, caption: *const u16, kind: u32) -> i32;
        }
        let wide = |s: &str| s.encode_utf16().chain([0]).collect::<Vec<u16>>();
        // SAFETY : les deux chaînes finissent par 0 et vivent pendant l'appel.
        unsafe { MessageBoxW(std::ptr::null_mut(), wide(text).as_ptr(), wide("Dokuro-chan").as_ptr(), 0x10) };
    }
    #[cfg(not(windows))]
    eprintln!("{text}");
}
