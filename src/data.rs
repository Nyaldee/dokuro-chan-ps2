//! Les formats du disque : paquets UNI2 d'images ART2, flux VBG en ADPCM
//! (musiques, voix), banque de sons Sony .hd/.bd (bruitages).

use std::collections::HashMap;

use crate::original::{Iso, SECTOR};

fn u16_at(b: &[u8], at: usize) -> u16 {
    b.get(at..at + 2).map_or(0, |s| u16::from_le_bytes([s[0], s[1]]))
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    b.get(at..at + 4).map_or(0, |s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// Un paquet UNI2 : un secteur d'en-tête (« UNI2 », nombre d'entrées en 8,
/// secteurs d'en-tête et de répertoire en 0x10), le répertoire (numéro,
/// place et taille en secteurs, taille en octets), puis les données.
pub struct Uni {
    sector: u64,
    entries: HashMap<u32, (u32, u32)>,
}

impl Uni {
    pub fn open(iso: &Iso, path: &str) -> Option<Uni> {
        let (lba, _) = iso.file(path)?;
        let first = iso.read(u64::from(lba), 0, SECTOR as usize)?;
        if &first[..4] != b"UNI2" {
            return None;
        }
        let sectors = u32_at(&first, 0x10).max(2);
        let head = iso.read(u64::from(lba), 0, sectors as usize * SECTOR as usize)?;
        let entries = (0..u32_at(&head, 8) as usize)
            .map(|k| {
                let at = SECTOR as usize + 16 * k;
                (u32_at(&head, at), (u32_at(&head, at + 4), u32_at(&head, at + 12)))
            })
            .collect();
        Some(Uni { sector: u64::from(lba) + u64::from(sectors), entries })
    }

    /// Une entrée, à sa taille exacte.
    pub fn read(&self, iso: &Iso, index: u32) -> Option<Vec<u8>> {
        let &(start, bytes) = self.entries.get(&index)?;
        iso.read(self.sector + u64::from(start), 0, bytes as usize)
    }
}

/// Une image en RGBA prémultiplié.
pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Une image ART2 : en-tête de 0x20 octets (« ART2 », bits par pixel,
/// largeur, hauteur, nom), pixels indexés sur 8 bits, palette de 256
/// couleurs en CSM1.
pub fn art2(data: &[u8]) -> Option<Image> {
    if data.get(..4)? != b"ART2" || u32_at(data, 4) != 8 {
        return None;
    }
    let (w, h) = (u32_at(data, 8), u32_at(data, 12));
    let n = (w * h) as usize;
    let pixels = data.get(0x20..0x20 + n)?;
    let colors = palette(data.get(0x20 + n..0x20 + n + 1024)?, true);
    let rgba = pixels.iter().flat_map(|&p| colors[usize::from(p)]).collect();
    Some(Image { width: w, height: h, rgba })
}

/// Une palette de 256 couleurs RGBA, prémultipliées (alpha de la PS2 : 0x80
/// opaque) ; en CSM1, rangée comme dans la mémoire de la PS2 (dans chaque
/// bloc de 32 couleurs, les 2e et 3e groupes de huit sont échangés).
fn palette(pal: &[u8], csm1: bool) -> Vec<[u8; 4]> {
    (0..256)
        .map(|i| {
            let j = if csm1 && matches!(i & 0x18, 0x08 | 0x10) { i ^ 0x18 } else { i };
            let c = &pal[4 * j..4 * j + 4];
            let a = (u16::from(c[3]) * 2).min(255);
            let pm = |v: u8| ((u16::from(v) * a + 127) / 255) as u8;
            [pm(c[0]), pm(c[1]), pm(c[2]), a as u8]
        })
        .collect()
}

/// Une image TIM2 (format de Sony ; ici la planche de l'écran de la carte
/// mémoire) : en-tête de 16 octets (« TIM2 », version, alignement, nombre
/// d'images), puis l'en-tête de 48 octets de la première image (tailles de
/// l'ensemble, de la palette, des pixels ; taille de l'en-tête ; nombre de
/// couleurs ; sorte de palette ; sorte de pixels ; largeur, hauteur), les
/// pixels, la palette. Le jeu ne s'en sert qu'en 8 bits par pixel avec une
/// palette de 256 couleurs RGBA, rangée comme celle d'ART2 (CSM1) sauf si
/// le bit 0x80 de sa sorte le dit.
pub fn tim2(data: &[u8]) -> Option<Image> {
    if data.get(..4)? != b"TIM2" {
        return None;
    }
    let at = if *data.get(5)? == 0 { 16 } else { 128 };
    let u16_at = |k: usize| Some(u16::from_le_bytes(data.get(at + k..at + k + 2)?.try_into().ok()?));
    let (clut_size, image_size) = (u32_at(data, at + 4) as usize, u32_at(data, at + 8) as usize);
    let (header, colors) = (usize::from(u16_at(12)?), u16_at(14)?);
    let (clut_type, kind) = (*data.get(at + 18)?, *data.get(at + 19)?);
    let (w, h) = (u32::from(u16_at(20)?), u32::from(u16_at(22)?));
    if kind != 5 || colors != 256 || clut_type & 0x3F != 3 {
        return None;
    }
    let pixels = data.get(at + header..at + header + image_size)?;
    let pal = data.get(at + header + image_size..at + header + image_size + clut_size)?;
    let colors = palette(pal.get(..1024)?, clut_type & 0x80 == 0);
    let rgba = pixels.iter().take((w * h) as usize).flat_map(|&p| colors[usize::from(p)]).collect();
    Some(Image { width: w, height: h, rgba })
}

/// Des échantillons mono, leur fréquence et le point de reprise d'une
/// boucle.
pub struct Pcm {
    pub samples: Vec<f32>,
    pub rate: u32,
    pub loop_start: Option<usize>,
}

/// ADPCM de la PS2 : blocs de 16 octets (décalage et filtre, drapeaux,
/// 28 échantillons de 4 bits). Drapeaux : 4 début de boucle, 1 fin (2 :
/// en boucle).
pub fn adpcm(data: &[u8], rate: u32) -> Pcm {
    const COEFS: [(i32, i32); 5] = [(0, 0), (60, 0), (115, -52), (98, -55), (122, -60)];
    let mut samples = Vec::with_capacity(data.len() / 16 * 28);
    let (mut h1, mut h2) = (0i32, 0i32);
    let mut loop_start = None;
    for block in data.chunks_exact(16) {
        let (shift, filter, flags) = (i32::from(block[0] & 0x0F), usize::from(block[0] >> 4).min(4), block[1]);
        if flags & 4 != 0 {
            loop_start = Some(samples.len());
        }
        let (c1, c2) = COEFS[filter];
        for &byte in &block[2..] {
            for nibble in [byte & 0x0F, byte >> 4] {
                let n = i32::from(nibble as i8) << 28 >> 28;
                let s = ((n << 12) >> shift) + ((h1 * c1 + h2 * c2 + 32) >> 6);
                let s = s.clamp(-32768, 32767);
                h2 = h1;
                h1 = s;
                samples.push(s as f32 / 32768.0);
            }
        }
        if flags & 1 != 0 {
            // Fin du son : en boucle, il reprend au début de boucle.
            if flags & 2 == 0 {
                loop_start = None;
            }
            break;
        }
    }
    Pcm { samples, rate, loop_start }
}

/// Un fichier VBG : un secteur d'en-tête (« VBG », nombre d'entrées, taille
/// du répertoire en secteurs à 0x10), le répertoire (numéro, place et
/// taille en secteurs, taille en octets), puis les données.
pub struct Vbg {
    sector: u64,
    entries: HashMap<u32, (u32, u32)>,
}

impl Vbg {
    pub fn open(iso: &Iso, path: &str) -> Option<Vbg> {
        let (lba, _) = iso.file(path)?;
        let head = iso.read(u64::from(lba), 0, SECTOR as usize)?;
        if &head[..4] != b"VBG " {
            return None;
        }
        let (count, dir_sectors) = (u32_at(&head, 8) as usize, u32_at(&head, 0x10));
        let dir = iso.read(u64::from(lba) + 1, 0, dir_sectors as usize * SECTOR as usize)?;
        let entries = (0..count).map(|k| (u32_at(&dir, 16 * k), (u32_at(&dir, 16 * k + 4), u32_at(&dir, 16 * k + 12)))).collect();
        Some(Vbg { sector: u64::from(lba) + 1 + u64::from(dir_sectors), entries })
    }

    /// Un flux, mono à 44 100 Hz (hauteur 0xEB3 des voix de flux).
    pub fn pcm(&self, iso: &Iso, id: u32) -> Option<Pcm> {
        let &(start, bytes) = self.entries.get(&id)?;
        let data = iso.read(self.sector + u64::from(start), 0, bytes as usize)?;
        let mut pcm = adpcm(&data, 44100);
        // Un flux se lit jusqu'au bout, sans boucle propre.
        pcm.loop_start = None;
        Some(pcm)
    }
}

/// Les bruitages de la banque « kihon » (SOUND.UNI, entrées 2 et 3 : .hd et
/// .bd), par numéro : le son n est la note n + 11 du programme 0, qui joue
/// un échantillon à sa hauteur d'origine. Avec son volume (zone et
/// échantillon, 0x80 = 1).
pub struct Se {
    pub pcm: std::sync::Arc<Pcm>,
    pub gain: f32,
}

pub fn bank(hd: &[u8], bd: &[u8]) -> HashMap<i32, Se> {
    let chunk = |name: &[u8]| -> Option<(usize, Vec<usize>)> {
        let pos = hd.windows(8).position(|w| w == name)?;
        let count = u32_at(hd, pos + 12) as usize + 1;
        Some((pos, (0..count).map(|i| u32_at(hd, pos + 16 + 4 * i) as usize).collect()))
    };
    let mut out = HashMap::new();
    let (Some((vpos, voffs)), Some((spos, soffs)), Some((sspos, ssoffs)), Some((ppos, poffs))) =
        (chunk(b"IECSigaV"), chunk(b"IECSlpmS"), chunk(b"IECStesS"), chunk(b"IECSgorP"))
    else {
        return out;
    };
    // Échantillons : place dans le .bd, fréquence, bouclage.
    let vags: Vec<(usize, u32)> = voffs.iter().map(|&o| (u32_at(hd, vpos + o) as usize, u32::from(u16_at(hd, vpos + o + 4)))).collect();
    let prog = ppos + poffs[0];
    let (nsplit, split_size) = (usize::from(hd[prog + 4]), usize::from(hd[prog + 5]));
    for s in 0..nsplit {
        let sp = prog + 0x24 + s * split_size;
        let (sset, note, zone) = (usize::from(u16_at(hd, sp)), i32::from(hd[sp + 2]), f32::from(hd[sp + 0x10]));
        let Some(&ss) = ssoffs.get(sset) else { continue };
        let Some(&sm) = soffs.get(usize::from(u16_at(hd, sspos + ss + 4))) else { continue };
        let vag = usize::from(u16_at(hd, spos + sm));
        let volume = f32::from(hd[spos + sm + 0x10]);
        let Some(&(at, rate)) = vags.get(vag) else { continue };
        let end = vags.get(vag + 1).map_or(bd.len(), |v| v.0).min(bd.len());
        let Some(data) = bd.get(at..end) else { continue };
        out.insert(note - 11, Se { pcm: std::sync::Arc::new(adpcm(data, rate)), gain: zone / 128.0 * volume / 128.0 });
    }
    out
}

#[cfg(test)]
mod tests {
    /// L'ADPCM : un bloc de zéros donne du silence, la fin s'arrête.
    #[test]
    fn adpcm() {
        let mut data = vec![0u8; 32];
        data[17] = 1;
        let pcm = super::adpcm(&data, 44100);
        assert_eq!(pcm.samples.len(), 56);
        assert!(pcm.samples.iter().all(|&s| s == 0.0));
        assert!(pcm.loop_start.is_none());
    }
}
