//! Les services du jeu que partagent toutes ses parties : manette, effets,
//! caméra, fondus, sons, voix, musique, hasard.

use std::collections::HashMap;
use std::sync::Arc;

use crate::audio::{Audio, Kind};
use crate::data::{self, Pcm, Se, Uni, Vbg};
use crate::engine::{BlackOut, Camera, Effects, Input, Rng, SeQueue};
use crate::options::Settings;
use crate::rumble::Rumble;
use crate::original::Iso;

/// Ce que partagent le titre, les menus, l'histoire et les mini-jeux :
/// manette, effets, caméra, fondus, sons, voix, nombres aléatoires,
/// personnage et mini-jeu choisis, résultat de la partie.
pub struct Sys {
    pub input: Input,
    pub effects: Effects,
    pub camera: Camera,
    pub blackout: BlackOut,
    /// Le plan 5 des fondus, qui assombrit sous les fenêtres.
    pub dim: BlackOut,
    pub se: SeQueue,
    pub rng: Rng,
    /// Le personnage choisi : 0 Dokuro, 1 Shizuki, 2 Sabato, 3 Zakuro,
    /// 4 Venom.
    pub chara: i32,
    /// Le mini-jeu (AD_MiniGameTypeGet) : 0 filet, 1 course, 2 memory,
    /// 3 attrape, 4 batte.
    pub kind: i32,
    /// Le résultat de la partie (AD_MiniGameResultSet) et ses points de
    /// challenge (AD_MiniGamePointtSet).
    pub won: bool,
    pub points: i32,
    /// La souris (points de l'écran) tant qu'elle est le dernier appareil
    /// dont s'est servi le joueur, et son clic de l'image (qui vaut aussi
    /// ○).
    pub mouse: Option<(f32, f32)>,
    pub click: Option<(f32, f32)>,
    /// Les zones de la souris de l'image (pointer::pick), pour DOKURO_ZONES.
    pub zones: Vec<[f32; 4]>,
    /// Compte à rebours de l'enchaînement, qui bloque aussi les touches de
    /// menu tant qu'il est positif (0x1da756a).
    pub lock: i32,
    pub lock_asked: bool,
    /// Les réglages des options, la vibration de la manette.
    pub settings: Settings,
    pub rumble: Rumble,
    iso: Arc<Iso>,
    audio: Audio,
    bank: HashMap<i32, Se>,
    voices: Option<Vbg>,
    music: Option<Vbg>,
    /// La musique en cours, même arrêtée par AD_BGMPause (gpffff8aa4).
    playing: Option<u32>,
    /// La musique demandée par un lot (−1 : l'arrêt) et les images avant
    /// qu'elle parte (gpffff8aa0, 8aa2).
    pending: Option<(i32, i32)>,
    /// Le flux de musique qui démarre, et les images avant qu'on l'entende.
    starting: Option<(Arc<Pcm>, i32)>,
    /// Les deux bruitages en boucle de l'histoire (AD_LoopSE*, gpffff8aa8 et
    /// 8aaa) : la note (−1 : aucune) et les images avant qu'elle parte ou
    /// s'arrête (0 ou moins : rien en attente).
    loops: [(i32, i32); 2],
}

/// Les images avant que la musique relancée après une vidéo s'entende. Sur
/// PS2, Snd_Strm_BgmPlay ne fait que déposer une demande : le flux ne part
/// qu'une fois ses premières données lues sur le disque, et un arrêt
/// demandé avant l'annule. Le jeu compte sur ce délai sans le savoir : après
/// une vidéo, AD_BGMRestart relance la musique que le lot suivant arrête
/// onze images plus tard (101:sure1), et on n'entend rien. Le portage lit
/// sans attendre, et toute musique part aussitôt ; seule cette relance
/// garde un délai fixe (un quart de seconde, valeur choisie, comme les
/// messages de la carte mémoire), sans quoi la musique revenait un instant.
const RESTART_LATENCY: i32 = 15;

impl Sys {
    /// Les sons du jeu : la banque des bruitages (SOUND.UNI), les voix
    /// (SE.VBG), la musique (BGM.VBG) ; leurs volumes, d'après les réglages
    /// (AD_OptionSndVolProc, dès le titre).
    pub fn new(iso: Arc<Iso>, audio: Audio, seed: u64, settings: Settings) -> Sys {
        let bank = Uni::open(&iso, "UNION/SOUND.UNI").and_then(|u| Some(data::bank(&u.read(&iso, 2)?, &u.read(&iso, 3)?))).unwrap_or_default();
        let mut sys = Sys {
            input: Input::default(),
            effects: Effects::default(),
            camera: Camera::default(),
            blackout: BlackOut::black(),
            dim: BlackOut::new(5),
            se: SeQueue::default(),
            rng: Rng::new(seed),
            chara: 0,
            kind: 0,
            won: false,
            points: 0,
            mouse: None,
            click: None,
            zones: Vec::new(),
            lock: 10,
            lock_asked: false,
            settings,
            rumble: Rumble::default(),
            voices: Vbg::open(&iso, "UNION/SE.VBG"),
            music: Vbg::open(&iso, "UNION/BGM.VBG"),
            iso,
            audio,
            bank,
            playing: None,
            pending: None,
            starting: None,
            loops: [(-1, 0); 2],
        };
        sys.apply_volumes();
        sys
    }

    /// AD_OptionSndVolProc : les volumes des réglages (la musique, les voix
    /// en flux, les bruitages de la banque).
    pub fn apply_volumes(&mut self) {
        let [music, effects, voices] = self.settings.volume;
        self.audio.set_volume(Kind::Music, music);
        self.audio.set_volume(Kind::Voice, voices);
        self.set_effect_volume(effects);
    }

    /// Snd_SetMsinVolume : le volume des bruitages de la banque.
    pub fn set_effect_volume(&mut self, v: u8) {
        self.audio.set_volume(Kind::Effect, v);
    }

    /// Snd_Strm_SePlayNum : une voix de SE.VBG.
    pub fn voice(&mut self, id: u32) {
        if let Some(pcm) = self.voices.as_ref().and_then(|v| v.pcm(&self.iso, id)) {
            self.audio.play_voice(Arc::new(pcm));
        }
    }

    pub fn voice_playing(&self) -> bool {
        self.audio.voice_playing()
    }

    pub fn stop_voice(&mut self) {
        self.audio.stop_voice();
    }

    /// AD_BGMStop.
    pub fn stop_music(&mut self) {
        self.audio.stop_music();
        self.playing = None;
        self.pending = None;
        self.starting = None;
    }

    /// AD_BGMPlay (Snd_Strm_BgmPlay) : le flux n (entrée n × 10 de
    /// BGM.VBG), en boucle.
    pub fn play_music(&mut self, n: u32) {
        self.pending = None;
        self.start_music(n, 0);
    }

    /// Snd_Strm_BgmPlay : la musique en cours change aussitôt ; on l'entend
    /// tout de suite, ou au bout de `latency` images.
    fn start_music(&mut self, n: u32, latency: i32) {
        self.playing = Some(n);
        self.audio.stop_music();
        self.starting = None;
        let Some(pcm) = self.music.as_ref().and_then(|m| m.pcm(&self.iso, n * 10)).map(Arc::new) else { return };
        if latency > 0 {
            self.starting = Some((pcm, latency));
        } else {
            self.audio.play_music(pcm);
        }
    }

    /// AD_BGMCheck : la musique en cours (même arrêtée par `pause_music`).
    pub fn music(&self) -> Option<u32> {
        self.playing
    }

    /// AD_BGMPause : la musique s'arrête, mais reste la musique en cours.
    pub fn pause_music(&mut self) {
        self.audio.stop_music();
        self.starting = None;
    }

    /// AD_BGMPlay (n ≥ 0) et AD_BGMStop (n < 0) : la musique d'un lot,
    /// demandée pour dans `delay` images (au moins une), que `end_frame`
    /// lance (AD_BGMProc).
    pub fn request_music(&mut self, n: i32, delay: i32) {
        self.pending = Some((n, delay.max(1)));
    }

    /// AD_AllSndAtOnceProc, quand le texte d'une page paraît : la musique
    /// demandée (AD_BGMAtOnceProc), les bruitages (AD_SEAtOnceProc) et les
    /// bruitages en boucle (AD_LoopSEAtOnceProc) en attente partent tout de
    /// suite.
    pub fn sounds_at_once(&mut self) {
        if let Some((n, _)) = self.pending.take() {
            self.start_requested(n);
        }
        for id in self.se.at_once() {
            self.note(id);
        }
        for slot in 0..2 {
            if self.loops[slot].1 > 0 {
                self.loop_note(slot, self.loops[slot].0);
                self.loops[slot].1 = -1;
            }
        }
    }

    /// AD_LoopSEPlay (note ≥ 0) et AD_LoopSEStop (−1) : un bruitage en
    /// boucle demandé dans une place, pour dans `delay` images (au moins une).
    pub fn loop_se(&mut self, slot: usize, id: i32, delay: i32) {
        if let Some(l) = self.loops.get_mut(slot) {
            *l = (id.max(-1), delay.max(1));
        }
    }

    /// Snd_CommandLOOPMSIN : la note en boucle d'une place, ou son arrêt.
    fn loop_note(&mut self, slot: usize, id: i32) {
        match self.bank.get(&id) {
            Some(se) if id >= 0 => self.audio.play_loop(slot, se.pcm.clone(), se.gain),
            _ => self.audio.stop_loop(Some(slot)),
        }
    }

    /// AD_LoopSEPause (options ou carte mémoire ouvertes depuis l'histoire) :
    /// les boucles se taisent, mais restent celles en place.
    pub fn pause_loops(&mut self) {
        self.audio.stop_loop(None);
    }

    /// AD_LoopSERePlay : les boucles en place reprennent.
    pub fn resume_loops(&mut self) {
        for slot in 0..2 {
            if self.loops[slot].0 >= 0 {
                self.loop_note(slot, self.loops[slot].0);
            }
        }
    }

    /// Snd_CommandMSIN et Snd_CommandLOOPMSIN (3, −1) : les bruitages et les
    /// boucles se taisent (au retour dans la chambre, au chargement d'une
    /// partie, au retour à une page de l'historique).
    pub fn stop_sounds(&mut self) {
        self.audio.stop_effects();
        self.audio.stop_loop(None);
    }

    /// En quittant l'histoire : plus aucune boucle en place. Le jeu, lui,
    /// ne les oublie qu'au démarrage (AD_SoundInit) : une boucle coupée en
    /// retournant dans la chambre restait en place, et la partie suivante la
    /// relançait au retour des options ou de la carte mémoire
    /// (AD_LoopSERePlay), sans fin. Bug corrigé, voir le README.
    pub fn forget_loops(&mut self) {
        self.audio.stop_loop(None);
        self.loops = [(-1, 0); 2];
    }

    /// Les bruitages en boucle en place (gardés dans une partie sauvegardée).
    pub fn loops(&self) -> [Option<i32>; 2] {
        self.loops.map(|l| (l.0 >= 0).then_some(l.0))
    }

    /// AD_AdvLoadProc : les boucles d'une partie reprise repartent (ou
    /// s'arrêtent) à la première image.
    pub fn restore_loops(&mut self, ids: [Option<i32>; 2]) {
        for (slot, id) in ids.into_iter().enumerate() {
            self.loop_se(slot, id.unwrap_or(-1), 1);
        }
    }

    fn start_requested(&mut self, n: i32) {
        match u32::try_from(n) {
            Ok(n) => self.play_music(n),
            Err(_) => self.stop_music(),
        }
    }

    /// AD_BGMRestart : la musique en cours repart du début ; après une vidéo
    /// (`after_movie`), au bout de RESTART_LATENCY images.
    pub fn restart_music(&mut self, after_movie: bool) {
        if let Some(n) = self.playing {
            self.start_music(n, if after_movie { RESTART_LATENCY } else { 0 });
        }
    }

    /// Le disque (les vidéos s'y lisent en flux).
    pub fn iso(&self) -> &Arc<Iso> {
        &self.iso
    }

    /// Le son des vidéos (voir `Audio::stream_start`).
    pub fn audio(&mut self) -> &mut Audio {
        &mut self.audio
    }

    /// Une note de la banque, ou l'arrêt de toutes (-1).
    fn note(&mut self, id: i32) {
        if id < 0 {
            self.audio.stop_effects();
        } else if let Some(se) = self.bank.get(&id) {
            self.audio.play_effect(se.pcm.clone(), se.gain);
        }
    }

    /// AD_SEPlay : les sons du menu (0 retour, 1 déplacement, 2 validation).
    pub fn menu_sound(&mut self, n: i32) {
        self.note(n + 1);
    }

    /// AD_InputKey pour une touche de menu (0x1e, 0x1f) : sans effet tant que
    /// le compte à rebours est positif, qui baisse alors d'une image.
    pub fn menu_key(&mut self, n: i32) -> bool {
        self.lock_asked = true;
        self.lock <= 0 && self.input.key(n)
    }

    /// Les touches de menu attendent `frames` images.
    pub fn lock_menu(&mut self, frames: i32) {
        self.lock = frames;
    }

    /// Après la logique : les effets, la caméra, les fondus et les
    /// bruitages dus.
    pub fn end_frame(&mut self) {
        // AD_BGMProc : la musique demandée part au bout de son retard.
        if let Some((n, delay)) = self.pending {
            self.pending = if delay > 1 { Some((n, delay - 1)) } else { None };
            if delay <= 1 {
                self.start_requested(n);
            }
        }
        // AD_LoopSEProc : les bruitages en boucle partent ou s'arrêtent au
        // bout de leur retard.
        for slot in 0..2 {
            let (id, delay) = self.loops[slot];
            if delay > 0 {
                self.loops[slot].1 = delay - 1;
                if delay == 1 {
                    self.loop_note(slot, id);
                    self.loops[slot].1 = -1;
                }
            }
        }
        // Le flux de musique s'entend au bout de son délai de démarrage.
        if let Some((pcm, left)) = self.starting.take() {
            if left > 1 {
                self.starting = Some((pcm, left - 1));
            } else {
                self.audio.play_music(pcm);
            }
        }
        self.effects.step();
        self.camera.step();
        self.blackout.step(&mut self.effects);
        self.dim.step(&mut self.effects);
        for id in self.se.step() {
            self.note(id);
        }
    }
}

