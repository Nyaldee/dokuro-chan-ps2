//! L'ISO original du joueur : cherchée au chemin donné dans la sauvegarde,
//! puis à côté de l'exécutable (sous n'importe quel nom), reconnue par son
//! CRC32 pour ne jouer que la bonne
//! version. Le CRC32 d'un disque entier est long à calculer : une fois
//! vérifiée, l'image est reconnue ensuite à sa taille et à sa date.
//! Ses fichiers sont lus directement sur le disque ISO 9660.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::Mutex;

/// « Game ni Natta yo! Dokuro-chan ~Kenkou Shindan Daisakusen~ » (SLPM-66186).
const CRC32: u32 = 0xB9F5_F0C5;
const SIZE: u64 = 1_487_536_128;
pub const SECTOR: u64 = 2048;

pub struct Iso {
    file: Mutex<File>,
    /// Chemin (« UNION/BAT.UNI ») : premier secteur et taille en octets.
    files: HashMap<String, (u32, u32)>,
}

impl Iso {
    /// L'ISO, et sa marque (taille et date) à retenir pour ne plus la
    /// vérifier ; `path` est le chemin donné par le joueur (fichier ou
    /// dossier), `known` la marque retenue. Sinon un message pour le joueur.
    pub fn load(path: &str, known: &str) -> Result<(Iso, String), String> {
        const MISSING: &str = "The original disc image (ISO) of Game ni Natta yo! Dokuro-chan was not found.\n\nPut it next to this executable.";
        let candidates: Vec<PathBuf> = find(path).into_iter().filter(|p| std::fs::metadata(p).is_ok_and(|m| m.len() == SIZE)).collect();
        if candidates.is_empty() {
            return Err(MISSING.to_string());
        }
        for path in &candidates {
            let stamp = stamp(path);
            if stamp != known && crc32_file(path) != Some(CRC32) {
                continue;
            }
            let file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
            let mut iso = Iso { file: Mutex::new(file), files: HashMap::new() };
            iso.read_directory().ok_or_else(|| format!("{}: unreadable disc image", path.display()))?;
            return Ok((iso, stamp));
        }
        Err(format!("No disc image found is the expected version (CRC32 {CRC32:08X}).\n\nThis port needs the original Game ni Natta yo! Dokuro-chan (SLPM-66186)."))
    }

    /// Les tests : l'ISO à la racine du dépôt, sans vérification (aucune
    /// si elle n'y est pas).
    #[cfg(test)]
    pub fn for_tests() -> Option<Iso> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let path = std::fs::read_dir(root).ok()?.flatten().map(|e| e.path()).find(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("iso")))?;
        let mut iso = Iso { file: Mutex::new(File::open(path).ok()?), files: HashMap::new() };
        iso.read_directory()?;
        Some(iso)
    }

    /// Des octets du disque, depuis un secteur.
    pub fn read(&self, sector: u64, offset: u64, len: usize) -> Option<Vec<u8>> {
        let mut file = self.file.lock().unwrap_or_else(|e| e.into_inner());
        file.seek(SeekFrom::Start(sector * SECTOR + offset)).ok()?;
        let mut out = vec![0; len];
        file.read_exact(&mut out).ok()?;
        Some(out)
    }

    /// Premier secteur et taille d'un fichier du disque.
    pub fn file(&self, path: &str) -> Option<(u32, u32)> {
        self.files.get(path).copied()
    }

    /// Le répertoire ISO 9660, depuis le descripteur primaire (secteur 16).
    fn read_directory(&mut self) -> Option<()> {
        let pvd = self.read(16, 0, SECTOR as usize)?;
        let root = pvd.get(156..156 + 34)?;
        let mut pending = vec![(u32_at(root, 2)?, u32_at(root, 10)?, String::new())];
        while let Some((lba, size, path)) = pending.pop() {
            let data = self.read(u64::from(lba), 0, size as usize)?;
            let mut pos = 0;
            while pos < data.len() {
                let n = usize::from(data[pos]);
                if n == 0 {
                    // Fin des entrées de ce secteur : le suivant.
                    pos = (pos / SECTOR as usize + 1) * SECTOR as usize;
                    continue;
                }
                let rec = data.get(pos..pos + n)?;
                pos += n;
                let name = rec.get(33..33 + usize::from(rec[32]))?;
                if name == [0] || name == [1] {
                    continue;
                }
                let name = String::from_utf8_lossy(name);
                let name = name.split(';').next().unwrap_or_default();
                let full = if path.is_empty() { name.to_string() } else { format!("{path}/{name}") };
                if rec[25] & 2 != 0 {
                    pending.push((u32_at(rec, 2)?, u32_at(rec, 10)?, full));
                } else {
                    self.files.insert(full, (u32_at(rec, 2)?, u32_at(rec, 10)?));
                }
            }
        }
        Some(())
    }
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

/// La marque d'une image : sa taille et sa date de modification.
fn stamp(path: &PathBuf) -> String {
    let meta = std::fs::metadata(path).ok();
    let modified = meta.as_ref().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
    format!("{}:{modified}", meta.map_or(0, |m| m.len()))
}

/// Les images ISO : celle donnée par le joueur (ou celles de son dossier ;
/// un chemin relatif part de l'exécutable), puis celles à côté de
/// l'exécutable ; en debug, aussi à la racine du dépôt.
fn find(path: &str) -> Vec<PathBuf> {
    let exe = crate::platform::exe_dir();
    let (mut files, mut dirs) = (Vec::new(), Vec::new());
    if !path.is_empty() {
        let given = exe.as_ref().map_or_else(|| PathBuf::from(path), |e| e.join(path));
        if given.is_dir() { dirs.push(given) } else { files.push(given) }
    }
    dirs.extend(exe);
    if cfg!(debug_assertions) {
        dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    }
    files.extend(
        dirs.iter()
            .filter_map(|d| std::fs::read_dir(d).ok())
            .flatten()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("iso"))),
    );
    files.retain(|p| p.is_file());
    files.dedup();
    files
}

/// Tables du CRC32 par tranches de 8 octets (le disque fait 1,4 Go).
fn tables() -> [[u32; 256]; 8] {
    let mut t = [[0u32; 256]; 8];
    for (i, entry) in t[0].iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
        *entry = c;
    }
    for i in 0..256 {
        for k in 1..8 {
            t[k][i] = (t[k - 1][i] >> 8) ^ t[0][(t[k - 1][i] & 0xFF) as usize];
        }
    }
    t
}

fn crc32_update(t: &[[u32; 256]; 8], mut c: u32, data: &[u8]) -> u32 {
    let mut chunks = data.chunks_exact(8);
    for b in &mut chunks {
        let lo = c ^ u32::from_le_bytes([b[0], b[1], b[2], b[3]]);
        c = t[7][(lo & 0xFF) as usize]
            ^ t[6][((lo >> 8) & 0xFF) as usize]
            ^ t[5][((lo >> 16) & 0xFF) as usize]
            ^ t[4][(lo >> 24) as usize]
            ^ t[3][b[4] as usize]
            ^ t[2][b[5] as usize]
            ^ t[1][b[6] as usize]
            ^ t[0][b[7] as usize];
    }
    for &b in chunks.remainder() {
        c = t[0][((c ^ u32::from(b)) & 0xFF) as usize] ^ (c >> 8);
    }
    c
}

pub fn crc32(data: &[u8]) -> u32 {
    !crc32_update(&tables(), !0, data)
}

fn crc32_file(path: &PathBuf) -> Option<u32> {
    let t = tables();
    let mut file = File::open(path).ok()?;
    let mut buf = vec![0u8; 4 << 20];
    let mut c = !0u32;
    loop {
        let n = file.read(&mut buf).ok()?;
        if n == 0 {
            return Some(!c);
        }
        c = crc32_update(&t, c, &buf[..n]);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn crc32() {
        assert_eq!(super::crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(super::crc32(&[7u8; 1001]), {
            let mut c = !0u32;
            for _ in 0..1001 {
                c ^= 7;
                for _ in 0..8 {
                    c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
                }
            }
            !c
        });
    }
}
