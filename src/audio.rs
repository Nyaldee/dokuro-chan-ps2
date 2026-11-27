//! Le son, comme le pilote du jeu : deux flux (la musique, une voix à la
//! fois), les bruitages de la banque et le son des vidéos, mixés sur le fil
//! audio de cpal.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::data::Pcm;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Music,
    Voice,
    Effect,
}

struct Voice {
    kind: Kind,
    pcm: Arc<Pcm>,
    pos: f64,
    /// La musique boucle sur tout le morceau ; un bruitage sur sa boucle.
    looped: bool,
    gain: f32,
    /// Un bruitage en boucle de l'histoire : sa place (0 ou 1).
    slot: Option<usize>,
}

/// Le son d'une vidéo, qui arrive en flux : ses échantillons entrelacés en
/// attente, la position entre les deux premiers, ce qui a été joué (par
/// voie).
struct Stream {
    rate: u32,
    channels: usize,
    queue: VecDeque<f32>,
    pos: f64,
    played: u64,
}

impl Stream {
    /// Mixe le flux dans `out` ; s'il manque des échantillons, il attend.
    fn render(&mut self, out: &mut [f32], channels: usize, rate: f64) {
        let sc = self.channels.max(1);
        let step = f64::from(self.rate) / rate;
        for frame in out.chunks_exact_mut(channels) {
            if self.queue.len() < 2 * sc {
                return;
            }
            let t = self.pos as f32;
            let at = |k: usize| self.queue[k] * (1.0 - t) + self.queue[sc + k] * t;
            if channels == 1 {
                frame[0] += (0..sc).map(at).sum::<f32>() / sc as f32;
            } else {
                for (c, o) in frame.iter_mut().enumerate().take(sc) {
                    *o += at(c);
                }
            }
            self.pos += step;
            while self.pos >= 1.0 && self.queue.len() >= sc {
                self.queue.drain(..sc);
                self.pos -= 1.0;
                self.played += 1;
            }
        }
    }
}

struct Mixer {
    voices: Vec<Voice>,
    stream: Option<Stream>,
    rate: u32,
    channels: usize,
    /// Le volume de chaque sorte (musique, voix, bruitages), de 0 à 1.
    volume: [f32; 3],
}

impl Default for Mixer {
    fn default() -> Mixer {
        Mixer { voices: Vec::new(), stream: None, rate: 0, channels: 0, volume: [1.0; 3] }
    }
}

impl Mixer {
    fn render(&mut self, out: &mut [f32]) {
        out.fill(0.0);
        let channels = self.channels.max(1);
        let rate = f64::from(self.rate.max(1));
        let volume = self.volume;
        self.voices.retain_mut(|v| {
            let gain = v.gain * volume[v.kind as usize];
            let n = v.pcm.samples.len();
            if n == 0 {
                return false;
            }
            let step = f64::from(v.pcm.rate) / rate;
            // La musique boucle sur tout le morceau, un bruitage en boucle de
            // l'histoire sur sa boucle (ou tout entier), une note sur sa boucle.
            let restart = match (v.kind, v.slot) {
                (Kind::Music, _) => Some(0),
                (_, Some(_)) => Some(v.pcm.loop_start.unwrap_or(0)),
                _ => v.pcm.loop_start.filter(|_| v.looped),
            };
            for frame in out.chunks_exact_mut(channels) {
                if v.pos >= n as f64 {
                    let Some(start) = restart else { return false };
                    v.pos = v.pos - n as f64 + start.min(n - 1) as f64;
                }
                let i = v.pos as usize;
                let t = (v.pos - i as f64) as f32;
                let next = if i + 1 < n { v.pcm.samples[i + 1] } else { v.pcm.samples[i] };
                let s = (v.pcm.samples[i] * (1.0 - t) + next * t) * gain;
                for c in frame.iter_mut() {
                    *c += s;
                }
                v.pos += step;
            }
            true
        });
        if let Some(s) = &mut self.stream {
            s.render(out, channels, rate);
        }
        for s in out {
            *s = s.clamp(-1.0, 1.0);
        }
    }
}

pub struct Audio {
    mixer: Arc<Mutex<Mixer>>,
    _stream: Option<cpal::Stream>,
}

impl Audio {
    pub fn new() -> Audio {
        let mut audio = Audio::silent();
        audio._stream = audio.open();
        audio
    }

    pub fn silent() -> Audio {
        Audio { mixer: Arc::new(Mutex::new(Mixer::default())), _stream: None }
    }

    fn open(&self) -> Option<cpal::Stream> {
        let device = cpal::default_host().default_output_device()?;
        let config = device.default_output_config().ok()?.config();
        {
            let mut m = lock(&self.mixer);
            m.rate = config.sample_rate;
            m.channels = usize::from(config.channels);
        }
        let mixer = self.mixer.clone();
        let stream = device.build_output_stream::<f32, _, _>(config, move |out, _| lock(&mixer).render(out), |_| {}, None).ok()?;
        stream.play().ok()?;
        Some(stream)
    }

    fn start(&mut self, kind: Kind, pcm: Arc<Pcm>, looped: bool, gain: f32) {
        // Sans sortie (tests), rien ne joue : une voix est aussitôt finie.
        if self._stream.is_none() {
            return;
        }
        let mut m = lock(&self.mixer);
        if kind != Kind::Effect {
            m.voices.retain(|v| v.kind != kind);
        }
        m.voices.push(Voice { kind, pcm, pos: 0.0, looped, gain, slot: None });
    }

    /// Snd_CommandLOOPMSIN (2) : une note de la banque en boucle, dans une
    /// des deux places des bruitages en boucle (remplace la précédente).
    pub fn play_loop(&mut self, slot: usize, pcm: Arc<Pcm>, gain: f32) {
        if self._stream.is_none() {
            return;
        }
        let mut m = lock(&self.mixer);
        m.voices.retain(|v| v.slot != Some(slot));
        m.voices.push(Voice { kind: Kind::Effect, pcm, pos: 0.0, looped: true, gain, slot: Some(slot) });
    }

    /// Snd_CommandLOOPMSIN (3) : la boucle d'une place s'arrête, ou toutes
    /// (None).
    pub fn stop_loop(&mut self, slot: Option<usize>) {
        lock(&self.mixer).voices.retain(|v| v.slot.is_none() || slot.is_some_and(|s| v.slot != Some(s)));
    }

    /// Snd_Strm_BgmPlay : la musique, en boucle.
    pub fn play_music(&mut self, pcm: Arc<Pcm>) {
        self.start(Kind::Music, pcm, true, 1.0);
    }

    pub fn stop_music(&mut self) {
        lock(&self.mixer).voices.retain(|v| v.kind != Kind::Music);
    }

    /// Snd_Strm_SePlayNum : une voix (remplace la précédente).
    pub fn play_voice(&mut self, pcm: Arc<Pcm>) {
        self.start(Kind::Voice, pcm, false, 1.0);
    }

    pub fn stop_voice(&mut self) {
        lock(&self.mixer).voices.retain(|v| v.kind != Kind::Voice);
    }

    /// Snd_Strm_SeCheck : une voix est-elle en cours ?
    pub fn voice_playing(&self) -> bool {
        lock(&self.mixer).voices.iter().any(|v| v.kind == Kind::Voice)
    }

    /// Un bruitage de la banque (une note).
    pub fn play_effect(&mut self, pcm: Arc<Pcm>, gain: f32) {
        self.start(Kind::Effect, pcm, true, gain);
    }

    /// Toutes les notes de la banque coupées (pas les bruitages en boucle,
    /// qui ont leur propre commande).
    pub fn stop_effects(&mut self) {
        lock(&self.mixer).voices.retain(|v| v.kind != Kind::Effect || v.slot.is_some());
    }

    /// Le son d'une vidéo commence (à plein volume, comme changeMasterVolume
    /// (0x3fff) de MovieTexture) ; sans sortie, il n'y en a pas.
    pub fn stream_start(&mut self, rate: u32, channels: usize) {
        if self._stream.is_some() {
            lock(&self.mixer).stream = Some(Stream { rate, channels, queue: VecDeque::new(), pos: 0.0, played: 0 });
        }
    }

    /// La suite du son de la vidéo (échantillons entrelacés).
    pub fn stream_push(&mut self, samples: &[i16]) {
        if let Some(s) = &mut lock(&self.mixer).stream {
            s.queue.extend(samples.iter().map(|&v| f32::from(v) / 32768.0));
        }
    }

    pub fn stream_stop(&mut self) {
        lock(&self.mixer).stream = None;
    }

    /// Le son de la vidéo : les échantillons (par voie) déjà joués et ceux
    /// en attente.
    pub fn stream_state(&self) -> Option<(u64, usize)> {
        lock(&self.mixer).stream.as_ref().map(|s| (s.played, s.queue.len() / s.channels.max(1)))
    }

    /// Le volume d'une sorte de son, de 0 à 0x7f (linéaire, comme
    /// Snd_Strm_VolSetCh : v × 0x3fff / 0x7f).
    pub fn set_volume(&mut self, kind: Kind, v: u8) {
        lock(&self.mixer).volume[kind as usize] = f32::from(v.min(0x7F)) / 127.0;
    }
}

/// Le mixeur, même si l'autre fil a paniqué en le tenant.
fn lock(mixer: &Mutex<Mixer>) -> MutexGuard<'_, Mixer> {
    mixer.lock().unwrap_or_else(|e| e.into_inner())
}
