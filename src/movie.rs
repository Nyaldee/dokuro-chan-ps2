//! Les vidéos (MovieTheater) : un fichier PSS du disque, décodé par
//! ps2-video. L'image prend l'écran aux proportions de la vidéo (640×448 en
//! 4:3, comme le reste du jeu), le son passe en flux et sert d'horloge.
//! START l'arrête, passé les 30 premières images ; sur PC, Échap et le clic
//! aussi.

use std::collections::VecDeque;
use std::io::{self, BufReader, Read};
use std::sync::Arc;

use ps2_video::{Frame, Movie};

use crate::data::Image;
use crate::engine::Button;
use crate::gfx::{Blend, Gfx};
use crate::original::Iso;
use crate::story::World;
use crate::sys::Sys;

/// L'image de la vidéo en cours.
const TEXTURE: &str = "movie";
/// Les images avant que START puisse arrêter la vidéo (iGpffff8a54).
const SKIP_AFTER: i32 = 0x1E;
/// Les images décodées d'avance (le son qui va avec arrive en chemin).
const AHEAD: usize = 6;
/// L'écran : 640×448, affiché en 4:3.
const SCREEN: (f32, f32) = (640.0, 448.0);
const SCREEN_ASPECT: f32 = 4.0 / 3.0;
/// Les vidéos de l'exécutable (AD_ADVSystemInit) ; les autres viennent des
/// scripts (movie_entry).
pub const BUILT_IN: [(&str, &str); 4] = [("op", "PSS/OP.PSS"), ("if", "PSS/IF_LOGO.PSS"), ("df", "PSS/DF.PSS"), ("mate", "PSS/IFMATE.PSS")];

/// Le fichier du disque d'une vidéo, d'après son nom (AD_MVBackNum).
pub fn file(world: &World, name: &[u8]) -> Option<String> {
    if let Some((_, f)) = BUILT_IN.iter().find(|(n, _)| n.as_bytes() == name) {
        return Some((*f).to_string());
    }
    world.movies.iter().find(|(n, _)| n == name).map(|(_, f)| iso_path(f))
}

/// « \PSS\AN02.PSS;1 » : « PSS/AN02.PSS ».
fn iso_path(f: &[u8]) -> String {
    let s = String::from_utf8_lossy(f);
    s.split(';').next().unwrap_or_default().trim_start_matches('\\').replace('\\', "/").to_uppercase()
}

/// Un fichier du disque, lu comme un flux.
struct IsoFile {
    iso: Arc<Iso>,
    sector: u32,
    pos: u64,
    size: u64,
}

impl Read for IsoFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = buf.len().min((self.size - self.pos) as usize);
        if n == 0 {
            return Ok(0);
        }
        let data = self.iso.read(u64::from(self.sector), self.pos, n).ok_or_else(|| io::Error::other("lecture de l'ISO"))?;
        buf[..n].copy_from_slice(&data);
        self.pos += n as u64;
        Ok(n)
    }
}

pub struct Player {
    movie: Movie<BufReader<IsoFile>>,
    /// Les images décodées d'avance, avec leur heure (s depuis la première).
    ahead: VecDeque<(f64, Frame)>,
    /// L'heure de la dernière image décodée, et de celle affichée.
    decoded: Option<f64>,
    shown: Option<f64>,
    first_pts: Option<u64>,
    /// Le décodeur n'a plus d'images.
    end: bool,
    /// La fréquence du son (son horloge), une fois lancé.
    rate: Option<u32>,
    clock: f64,
    frames: i32,
    /// La taille de l'image affichée.
    size: (usize, usize),
}

impl Player {
    /// La vidéo, si son fichier est sur le disque.
    pub fn open(iso: &Arc<Iso>, path: &str) -> Option<Player> {
        let (sector, size) = iso.file(path)?;
        let src = IsoFile { iso: iso.clone(), sector, pos: 0, size: u64::from(size) };
        Some(Player {
            movie: Movie::new(BufReader::with_capacity(1 << 18, src)),
            ahead: VecDeque::new(),
            decoded: None,
            shown: None,
            first_pts: None,
            end: false,
            rate: None,
            clock: 0.0,
            frames: 0,
            size: (0, 0),
        })
    }

    /// La durée d'une image.
    fn frame_time(&self) -> f64 {
        let (n, d) = self.movie.sequence().map_or((30000, 1001), |s| s.frame_rate);
        if n == 0 { 1.0 / 30.0 } else { f64::from(d) / f64::from(n) }
    }

    /// Décode d'avance ; le son lu en chemin part au mixeur.
    fn fill(&mut self, sys: &mut Sys) {
        while self.ahead.len() < AHEAD && !self.end {
            match self.movie.frame() {
                Ok(Some(f)) => {
                    let first = *self.first_pts.get_or_insert(f.pts.unwrap_or(0));
                    let t = match (f.pts, self.decoded) {
                        (Some(p), _) => p.saturating_sub(first) as f64 / 90_000.0,
                        (None, Some(last)) => last + self.frame_time(),
                        (None, None) => 0.0,
                    };
                    self.decoded = Some(t);
                    self.ahead.push_back((t, f));
                }
                _ => self.end = true,
            }
        }
        if self.rate.is_none() {
            // Le son commence dès que son en-tête est lu (avec la première
            // image) ; une vidéo muette suit le temps du jeu.
            match self.movie.audio() {
                Some(a) => {
                    sys.audio().stream_start(a.rate, a.channels);
                    self.rate = Some(a.rate);
                }
                None if self.end => self.rate = Some(0),
                None => return,
            }
        }
        let samples = self.movie.take_samples();
        sys.audio().stream_push(&samples);
    }

    /// Une image du jeu ; vrai quand la vidéo est finie ou arrêtée.
    pub fn step(&mut self, sys: &mut Sys, gfx: &mut Gfx) -> bool {
        self.frames += 1;
        self.fill(sys);
        // L'horloge : le son joué tant qu'il peut avancer, sinon le temps du
        // jeu. Le mixeur interpole entre deux échantillons : le dernier reste
        // en attente, et l'horloge s'y arrêterait (la vidéo, dont la fin
        // tombe avec celle du son, resterait figée).
        self.clock += 1.0 / 60.0;
        if let (Some(rate @ 1..), Some((played, queued))) = (self.rate, sys.audio().stream_state())
            && queued > 1
        {
            self.clock = played as f64 / f64::from(rate);
        }
        let mut newest = None;
        while let Some(&(t, _)) = self.ahead.front()
            && t <= self.clock
        {
            newest = self.ahead.pop_front();
        }
        if let Some((t, f)) = newest {
            self.size = (f.width, f.height);
            gfx.load(TEXTURE, Image { width: f.width as u32, height: f.height as u32, rgba: f.rgba_smooth() });
            self.shown = Some(t);
        }
        let stop = self.frames > SKIP_AFTER && (sys.input.key(Button::Start as i32) || sys.input.key(Button::Menu as i32) || sys.click.is_some());
        let over = self.end && self.ahead.is_empty() && self.shown.is_none_or(|t| self.clock >= t + self.frame_time());
        if stop || over {
            sys.audio().stream_stop();
            gfx.unload(TEXTURE);
            return true;
        }
        false
    }

    /// L'image en cours (sur le noir, avant la première).
    pub fn draw(&self, gfx: &mut Gfx) {
        let (Some(_), Some(seq)) = (self.shown, self.movie.sequence()) else { return };
        let (w, h) = (self.size.0 as f32, self.size.1 as f32);
        gfx.tran(TEXTURE, [0.0, 0.0, w, h], place(seq.display_aspect() as f32), 0, 0x80, Blend::Normal, [0x80; 3]);
    }
}

/// Le rectangle d'une vidéo de proportions `aspect` dans l'écran 4:3 :
/// tout l'écran pour du 4:3, des bandes noires sinon.
fn place(aspect: f32) -> [f32; 4] {
    let (sw, sh) = SCREEN;
    if (aspect - SCREEN_ASPECT).abs() < 0.01 {
        [0.0, 0.0, sw, sh]
    } else if aspect > SCREEN_ASPECT {
        let h = sh * SCREEN_ASPECT / aspect;
        [0.0, (sh - h) / 2.0, sw, h]
    } else {
        let w = sw * aspect / SCREEN_ASPECT;
        [(sw - w) / 2.0, 0.0, w, sh]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_and_aspect() {
        assert_eq!(iso_path(b"\\PSS\\an02.pss;1"), "PSS/AN02.PSS");
        assert_eq!(place(4.0 / 3.0), [0.0, 0.0, 640.0, 448.0]);
        // Du 16:9 dans l'écran 4:3 : des bandes en haut et en bas.
        let [_, y, w, h] = place(16.0 / 9.0);
        assert_eq!(w, 640.0);
        assert!((h - 336.0).abs() < 0.01 && (y - 56.0).abs() < 0.01);
    }

    /// Toutes les vidéos des scripts et de l'exécutable sont sur le disque.
    #[test]
    fn every_movie_is_on_the_disc() {
        let Some(iso) = Iso::for_tests() else { return };
        let iso = Arc::new(iso);
        let mut adv = crate::adv::Adv::new(iso.clone());
        adv.boot(0);
        let names: Vec<Vec<u8>> = BUILT_IN.iter().map(|(n, _)| n.as_bytes().to_vec()).chain(adv.world.movies.iter().map(|(n, _)| n.clone())).collect();
        for name in names {
            let path = file(&adv.world, &name).unwrap();
            assert!(Player::open(&iso, &path).is_some(), "{} : {path}", String::from_utf8_lossy(&name));
        }
        // Chaque vidéo de l'album a son numéro (« op » comprise).
        for e in adv.world.cg.iter().filter(|e| e.movie) {
            let id = crate::album::movie_id(&adv.world, &e.name);
            assert!(id.is_some(), "{}", String::from_utf8_lossy(&e.name));
            assert_eq!(crate::album::movie_name(&adv.world, id.unwrap()), Some(&e.name[..]));
        }
    }
}
