//! Les manettes (XInput sous Windows, /dev/input/js* sous Linux), à la
//! place de celle de la PS2 : A pour ○, B pour ×, X pour □, Y pour △, LB
//! et RB pour L1 et R1, les gâchettes pour L2 et R2, la croix et le stick
//! gauche pour la croix ; Start + Select quitte. Les deux moteurs de la
//! DualShock 2 vibrent par ceux de la manette : le gros (force de 0 à 255)
//! par le moteur basse fréquence, le petit (en marche ou non) par le moteur
//! haute fréquence.

/// Les boutons des manettes branchées, réunis.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Buttons {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub a: bool,
    pub b: bool,
    pub x: bool,
    pub y: bool,
    pub l1: bool,
    pub r1: bool,
    pub l2: bool,
    pub r2: bool,
    pub start: bool,
    pub select: bool,
}

#[cfg(windows)]
pub use windows::Pads;

#[cfg(not(windows))]
pub use unix::Pads;

/// Stick gauche : seuil de navigation (deux fois la zone morte conseillée
/// par XInput, un stick au repos ne bouge rien).
const STICK: i16 = 7849 * 2;
/// Images avant de réinterroger une manette absente.
const RETRY: u32 = 120;

#[cfg(not(windows))]
mod unix {
    use std::fs::File;
    use std::io::{ErrorKind, Read, Write};
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::OpenOptionsExt;

    use super::{Buttons, RETRY, STICK};

    /// O_NONBLOCK : lire sans attendre d'événement.
    const O_NONBLOCK: i32 = 0o4000;
    /// js_event : type bouton, axe ; drapeau de l'état initial.
    const JS_BUTTON: u8 = 1;
    const JS_AXIS: u8 = 2;
    const JS_INIT: u8 = 0x80;

    /// EVIOCSFF : _IOW('E', 0x80, struct ff_effect), qui fait 48 octets.
    const EVIOCSFF: std::ffi::c_ulong = 0x4030_4580;
    /// EV_FF, FF_RUMBLE.
    const EV_FF: u16 = 0x15;
    const FF_RUMBLE: u16 = 0x50;

    unsafe extern "C" {
        fn ioctl(fd: i32, request: std::ffi::c_ulong, ...) -> i32;
    }

    /// L'union de struct ff_effect (32 octets, alignée sur 8) : ici
    /// ff_rumble_effect, les moteurs fort (basse fréquence) et faible.
    #[repr(C)]
    #[derive(Default)]
    struct FfUnion {
        strong: u16,
        weak: u16,
        rest: [u16; 2],
        tail: [u64; 3],
    }

    /// struct ff_effect : sorte, numéro (−1 : à attribuer), direction,
    /// déclencheur, durée et retard (ms), l'union.
    #[repr(C)]
    #[derive(Default)]
    struct FfEffect {
        kind: u16,
        id: i16,
        direction: u16,
        trigger: [u16; 2],
        replay: [u16; 2],
        u: FfUnion,
    }

    // EVIOCSFF compte sur la taille de struct ff_effect.
    const _: () = assert!(std::mem::size_of::<FfEffect>() == 48);

    /// Le périphérique evdev d'une manette (l'interface joystick ne sait
    /// pas vibrer) et le numéro de son effet.
    struct Motors {
        file: File,
        id: i16,
    }

    impl Motors {
        /// Le périphérique « event » de la manette jsN.
        fn open(i: usize) -> Option<Motors> {
            let name = std::fs::read_dir(format!("/sys/class/input/js{i}/device")).ok()?.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).find(|n| n.starts_with("event"))?;
            let file = std::fs::OpenOptions::new().read(true).write(true).open(format!("/dev/input/{name}")).ok()?;
            Some(Motors { file, id: -1 })
        }

        /// L'effet mis à jour (fort, faible), puis joué, ou arrêté.
        fn set(&mut self, strong: u16, weak: u16) {
            if strong != 0 || weak != 0 {
                // La durée la plus longue (65 s) : le jeu dit quand arrêter.
                let mut e = FfEffect { kind: FF_RUMBLE, id: self.id, replay: [u16::MAX, 0], u: FfUnion { strong, weak, ..FfUnion::default() }, ..FfEffect::default() };
                // SAFETY : `e` a la forme de struct ff_effect, que le noyau lit
                // et où il écrit le numéro de l'effet.
                if unsafe { ioctl(self.file.as_raw_fd(), EVIOCSFF, &mut e) } < 0 {
                    return;
                }
                self.id = e.id;
            }
            if self.id < 0 {
                return;
            }
            // struct input_event : l'heure (ignorée), EV_FF, le numéro, 1 joue
            // et 0 arrête.
            let mut event = [0u8; 24];
            event[16..18].copy_from_slice(&EV_FF.to_ne_bytes());
            event[18..20].copy_from_slice(&(self.id as u16).to_ne_bytes());
            event[20..24].copy_from_slice(&i32::from(strong != 0 || weak != 0).to_ne_bytes());
            let _ = self.file.write_all(&event);
        }
    }

    /// Une manette ouverte : l'état de ses boutons et de ses axes, ses
    /// moteurs (cherchés à la première vibration).
    struct Pad {
        file: File,
        buttons: [bool; 16],
        axes: [i16; 8],
        motors: Option<Option<Motors>>,
    }

    /// Les manettes de l'interface joystick du noyau (js0 à js3), dans la
    /// disposition des manettes Xbox (pilote xpad) : A 0, B 1, X 2, Y 3,
    /// LB 4, RB 5, Select 6, Start 7 ; stick gauche axes 0 et 1, gâchettes
    /// axes 2 et 5, croix axes 6 et 7.
    pub struct Pads {
        /// Ouverte, ou images avant de réessayer.
        slots: [Result<Pad, u32>; 4],
    }

    impl Pads {
        pub fn new() -> Pads {
            Pads { slots: [Err(0), Err(0), Err(0), Err(0)] }
        }

        /// Les boutons de toutes les manettes branchées.
        pub fn poll(&mut self) -> Buttons {
            let mut pad = Buttons::default();
            for (i, slot) in self.slots.iter_mut().enumerate() {
                if let Err(wait) = slot {
                    if *wait > 0 {
                        *wait -= 1;
                        continue;
                    }
                    match std::fs::OpenOptions::new().read(true).custom_flags(O_NONBLOCK).open(format!("/dev/input/js{i}")) {
                        Ok(file) => *slot = Ok(Pad { file, buttons: [false; 16], axes: [0; 8], motors: None }),
                        Err(_) => {
                            *slot = Err(RETRY);
                            continue;
                        }
                    }
                }
                let Ok(p) = slot else { continue };
                if !p.drain() {
                    // Débranchée : on réessaiera plus tard.
                    *slot = Err(RETRY);
                    continue;
                }
                let (b, a) = (&p.buttons, &p.axes);
                pad.up |= a[7] < -STICK || a[1] < -STICK;
                pad.down |= a[7] > STICK || a[1] > STICK;
                pad.left |= a[6] < -STICK || a[0] < -STICK;
                pad.right |= a[6] > STICK || a[0] > STICK;
                pad.a |= b[0];
                pad.b |= b[1];
                pad.x |= b[2];
                pad.y |= b[3];
                pad.l1 |= b[4];
                pad.r1 |= b[5];
                // Les gâchettes vont de -32767 (relâchées) à 32767.
                pad.l2 |= a[2] > 0;
                pad.r2 |= a[5] > 0;
                pad.select |= b[6];
                pad.start |= b[7];
            }
            pad
        }

        /// Les moteurs de toutes les manettes branchées : le petit, le gros.
        pub fn rumble(&mut self, small: bool, big: u8) {
            for (i, slot) in self.slots.iter_mut().enumerate() {
                if let Ok(p) = slot
                    && let Some(m) = p.motors.get_or_insert_with(|| Motors::open(i))
                {
                    m.set(u16::from(big) * 257, if small { u16::MAX } else { 0 });
                }
            }
        }
    }

    impl Pad {
        /// Lit les événements en attente ; faux si la manette a disparu.
        fn drain(&mut self) -> bool {
            let mut event = [0u8; 8];
            loop {
                match self.file.read(&mut event) {
                    Ok(8) => {
                        // js_event : temps (u32), valeur (i16), type, numéro.
                        let value = i16::from_ne_bytes([event[4], event[5]]);
                        let (kind, number) = (event[6] & !JS_INIT, event[7] as usize);
                        match kind {
                            JS_BUTTON if number < self.buttons.len() => self.buttons[number] = value != 0,
                            JS_AXIS if number < self.axes.len() => self.axes[number] = value,
                            _ => {}
                        }
                    }
                    Ok(_) => return false,
                    Err(e) if e.kind() == ErrorKind::WouldBlock => return true,
                    Err(e) if e.kind() == ErrorKind::Interrupted => {}
                    Err(_) => return false,
                }
            }
        }
    }
}

#[cfg(windows)]
mod windows {
    use super::{Buttons, RETRY, STICK};

    /// XINPUT_GAMEPAD_TRIGGER_THRESHOLD.
    const TRIGGER: u8 = 30;

    #[repr(C)]
    #[derive(Default)]
    struct XInputState {
        packet: u32,
        buttons: u16,
        triggers: [u8; 2],
        thumbs: [i16; 4],
    }

    /// XINPUT_VIBRATION : moteurs basse et haute fréquence.
    #[repr(C)]
    struct XInputVibration {
        low: u16,
        high: u16,
    }

    #[link(name = "xinput")]
    unsafe extern "system" {
        fn XInputGetState(index: u32, state: *mut XInputState) -> u32;
        fn XInputSetState(index: u32, vibration: *mut XInputVibration) -> u32;
    }

    pub struct Pads {
        /// Branchée, ou images avant de réessayer.
        state: [Result<(), u32>; 4],
    }

    impl Pads {
        pub fn new() -> Pads {
            Pads { state: [Ok(()); 4] }
        }

        /// Les boutons de toutes les manettes branchées.
        pub fn poll(&mut self) -> Buttons {
            let mut pad = Buttons::default();
            for (i, state) in self.state.iter_mut().enumerate() {
                if let Err(wait) = state
                    && *wait > 0
                {
                    *wait -= 1;
                    continue;
                }
                let mut s = XInputState::default();
                // SAFETY : `s` a la forme de XINPUT_STATE.
                if unsafe { XInputGetState(i as u32, &mut s) } != 0 {
                    *state = Err(RETRY);
                    continue;
                }
                *state = Ok(());
                let (b, [lx, ly, ..]) = (s.buttons, s.thumbs);
                pad.up |= b & 0x0001 != 0 || ly > STICK;
                pad.down |= b & 0x0002 != 0 || ly < -STICK;
                pad.left |= b & 0x0004 != 0 || lx < -STICK;
                pad.right |= b & 0x0008 != 0 || lx > STICK;
                pad.start |= b & 0x0010 != 0;
                pad.select |= b & 0x0020 != 0;
                pad.l1 |= b & 0x0100 != 0;
                pad.r1 |= b & 0x0200 != 0;
                // Les gâchettes : au-delà du seuil conseillé par XInput.
                pad.l2 |= s.triggers[0] > TRIGGER;
                pad.r2 |= s.triggers[1] > TRIGGER;
                pad.a |= b & 0x1000 != 0;
                pad.b |= b & 0x2000 != 0;
                pad.x |= b & 0x4000 != 0;
                pad.y |= b & 0x8000 != 0;
            }
            pad
        }

        /// Les moteurs de toutes les manettes branchées : le petit, le gros.
        pub fn rumble(&mut self, small: bool, big: u8) {
            let mut v = XInputVibration { low: u16::from(big) * 257, high: if small { u16::MAX } else { 0 } };
            for (i, state) in self.state.iter().enumerate() {
                if state.is_ok() {
                    // SAFETY : `v` a la forme de XINPUT_VIBRATION.
                    unsafe { XInputSetState(i as u32, &mut v) };
                }
            }
        }
    }
}
