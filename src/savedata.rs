//! Une partie sauvegardée (AD_AdvSaveProc, AD_AdvLoadProc) : ce que le jeu
//! fige d'une page qui attend, pour y revenir en rejouant le point
//! d'entrée du script qui a rempli ses lots. Et son codage dans la
//! sauvegarde du portage (une chaîne hexadécimale par fichier, clés
//! `slot1` à `slot12`) ; la clé `gamedata` dit que les données du jeu
//! existent (le jeu les crée sur la carte mémoire à la première
//! sauvegarde, avec douze fichiers vides).

use crate::save::Save;
use crate::story::{Entry, Flag};

/// Les douze fichiers de la carte mémoire.
pub const SLOTS: usize = 12;
const GAME_DATA: &str = "gamedata";

/// Un personnage à l'écran (la table 0x1c24c00) : nom, image et expression,
/// numéro, place, couleur.
#[derive(Clone, Debug, PartialEq)]
pub struct Actor {
    pub name: Vec<u8>,
    pub image: Vec<u8>,
    pub emotion: Vec<u8>,
    pub index: i32,
    pub x: i32,
    pub rgb: [i32; 3],
}

/// Un plan de fondu tenu (0x1be95f0) : opacité, couleur, forme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plane {
    pub alpha: i32,
    pub rgb: [i32; 3],
    pub shape: (u8, u8),
}

/// La scène d'une page : décor, caméra (x, y, zoom), personnages, plans 0
/// à 3 (les plans 4 et 5 assombrissent sous les menus).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Scene {
    pub bg: Option<Vec<u8>>,
    pub camera: (i32, i32, i32),
    pub actors: Vec<Actor>,
    pub planes: [Option<Plane>; 4],
}

/// Ce qu'attendait le joueur (cRam01da7310 : 6, 7, 9).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Waiting {
    #[default]
    Read,
    Choose,
    Map,
}

/// Une partie sauvegardée.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Game {
    /// Le titre (save_name, suivi de « #n ») et l'heure de la sauvegarde
    /// (année, mois, jour, heure, minute ; mcsGetTime).
    pub title: Vec<u8>,
    pub time: [i32; 5],
    /// Le point d'entrée joué, les drapeaux de l'histoire à son départ
    /// (AD_AdvSaveEachProc), le lot de la page, les drapeaux à la page.
    pub entry: Entry,
    pub flags_at_entry: Vec<Option<Flag>>,
    pub batch: usize,
    pub flags: Vec<Option<Flag>>,
    /// La musique, les deux bruitages en boucle, le nom affiché, le script
    /// de la carte.
    pub music: Option<u32>,
    pub loops: [Option<i32>; 2],
    pub speaker: Option<Vec<u8>>,
    pub map_script: Entry,
    pub scene: Scene,
    pub state: Waiting,
}

/// Des nombres et des octets à la suite.
#[derive(Default)]
struct Writer(Vec<u8>);

impl Writer {
    fn int(&mut self, v: i32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }

    fn bytes(&mut self, b: &[u8]) {
        self.int(b.len() as i32);
        self.0.extend_from_slice(b);
    }

    fn option(&mut self, b: &Option<Vec<u8>>) {
        self.int(i32::from(b.is_some()));
        if let Some(b) = b {
            self.bytes(b);
        }
    }

    fn entry(&mut self, e: &Entry) {
        self.int(e.file);
        self.bytes(&e.name);
    }

    fn flags(&mut self, flags: &[Option<Flag>]) {
        self.int(flags.len() as i32);
        for f in flags {
            self.option(&f.as_ref().map(|f| f.name.clone()));
            self.int(f.as_ref().map_or(0, |f| i32::from(f.value)));
        }
    }
}

struct Reader<'a>(&'a [u8]);

impl Reader<'_> {
    fn int(&mut self) -> Option<i32> {
        let (head, rest) = self.0.split_at_checked(4)?;
        self.0 = rest;
        Some(i32::from_le_bytes(head.try_into().ok()?))
    }

    fn bytes(&mut self) -> Option<Vec<u8>> {
        let n = usize::try_from(self.int()?).ok()?;
        let (head, rest) = self.0.split_at_checked(n)?;
        self.0 = rest;
        Some(head.to_vec())
    }

    fn option(&mut self) -> Option<Option<Vec<u8>>> {
        Some(if self.int()? != 0 { Some(self.bytes()?) } else { None })
    }

    fn entry(&mut self) -> Option<Entry> {
        Some(Entry { file: self.int()?, name: self.bytes()? })
    }

    fn flags(&mut self) -> Option<Vec<Option<Flag>>> {
        (0..self.int()?).map(|_| Some(self.option()?.zip(Some(self.int()? as i16)).map(|(name, value)| Flag { name, value }))).collect()
    }
}

impl Game {
    fn encode(&self) -> String {
        let mut w = Writer::default();
        w.bytes(&self.title);
        for v in self.time {
            w.int(v);
        }
        w.entry(&self.entry);
        w.flags(&self.flags_at_entry);
        w.int(self.batch as i32);
        w.flags(&self.flags);
        w.int(self.music.map_or(-1, |m| m as i32));
        w.option(&self.speaker);
        w.entry(&self.map_script);
        let s = &self.scene;
        w.option(&s.bg);
        for v in [s.camera.0, s.camera.1, s.camera.2] {
            w.int(v);
        }
        w.int(s.actors.len() as i32);
        for a in &s.actors {
            w.bytes(&a.name);
            w.bytes(&a.image);
            w.bytes(&a.emotion);
            for v in [a.index, a.x, a.rgb[0], a.rgb[1], a.rgb[2]] {
                w.int(v);
            }
        }
        for p in &s.planes {
            w.int(i32::from(p.is_some()));
            if let Some(p) = p {
                for v in [p.alpha, p.rgb[0], p.rgb[1], p.rgb[2], i32::from(p.shape.0), i32::from(p.shape.1)] {
                    w.int(v);
                }
            }
        }
        w.int(self.state as i32);
        for v in self.loops {
            w.int(v.unwrap_or(-1));
        }
        w.0.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn decode(text: &str) -> Option<Game> {
        let raw: Vec<u8> = (0..text.len() / 2).map(|k| u8::from_str_radix(text.get(2 * k..2 * k + 2)?, 16).ok()).collect::<Option<_>>()?;
        let mut r = Reader(&raw);
        let title = r.bytes()?;
        let time = [r.int()?, r.int()?, r.int()?, r.int()?, r.int()?];
        let entry = r.entry()?;
        let flags_at_entry = r.flags()?;
        let batch = usize::try_from(r.int()?).ok()?;
        let flags = r.flags()?;
        let music = u32::try_from(r.int()?).ok();
        let speaker = r.option()?;
        let map_script = r.entry()?;
        let bg = r.option()?;
        let camera = (r.int()?, r.int()?, r.int()?);
        let actors = (0..r.int()?)
            .map(|_| Some(Actor { name: r.bytes()?, image: r.bytes()?, emotion: r.bytes()?, index: r.int()?, x: r.int()?, rgb: [r.int()?, r.int()?, r.int()?] }))
            .collect::<Option<_>>()?;
        let mut planes = [None; 4];
        for p in &mut planes {
            if r.int()? != 0 {
                *p = Some(Plane { alpha: r.int()?, rgb: [r.int()?, r.int()?, r.int()?], shape: (r.int()? as u8, r.int()? as u8) });
            }
        }
        // Un fichier écrit avant que l'état n'y soit gardé attend une page.
        let state = match r.int() {
            Some(1) => Waiting::Choose,
            Some(2) => Waiting::Map,
            _ => Waiting::Read,
        };
        // Un fichier écrit avant que les boucles n'y soient gardées n'en a pas.
        let mut slot = || r.int().filter(|&v| v >= 0);
        let loops = [slot(), slot()];
        Some(Game { title, time, entry, flags_at_entry, batch, flags, music, loops, speaker, map_script, scene: Scene { bg, camera, actors, planes }, state })
    }
}

/// La carte mémoire du portage : les données du jeu existent-elles, et
/// les douze fichiers (None : vide, « NON DATA »).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Card {
    pub game_data: bool,
    pub slots: [Option<Game>; SLOTS],
}

impl Card {
    pub fn read(save: &Save) -> Card {
        Card { game_data: save.get(GAME_DATA) == "1", slots: std::array::from_fn(|k| Game::decode(save.get(&format!("slot{}", k + 1)))) }
    }

    pub fn write(&self, save: &mut Save) {
        save.set(GAME_DATA, if self.game_data { "1" } else { "" });
        for (k, slot) in self.slots.iter().enumerate() {
            save.set(&format!("slot{}", k + 1), &slot.as_ref().map(Game::encode).unwrap_or_default());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Écrite puis relue, la carte ne change pas.
    #[test]
    fn round_trip() {
        let game = Game {
            title: "プロローグ#n".as_bytes().to_vec(),
            time: [2026, 10, 28, 21, 5],
            entry: Entry { file: 101, name: b"sure1".to_vec() },
            flags_at_entry: vec![Some(Flag { value: 2, name: b"switch".to_vec() }), None],
            batch: 3,
            flags: vec![Some(Flag { value: -1, name: b"select".to_vec() })],
            music: Some(3),
            loops: [Some(5), None],
            speaker: Some(b"\x82\xa0".to_vec()),
            map_script: Entry::default(),
            scene: Scene {
                bg: Some(b"bg03_a".to_vec()),
                camera: (0, -20, 110),
                actors: vec![Actor { name: b"dokuro".to_vec(), image: b"a".to_vec(), emotion: b"b".to_vec(), index: 1, x: 320, rgb: [0x80; 3] }],
                planes: [None, Some(Plane { alpha: 0x40, rgb: [0; 3], shape: (0, 0) }), None, None],
            },
            state: Waiting::Choose,
        };
        let mut card = Card { game_data: true, ..Card::default() };
        card.slots[4] = Some(game);
        let mut save = Save::memory();
        card.write(&mut save);
        assert_eq!(Card::read(&save), card);
    }
}
