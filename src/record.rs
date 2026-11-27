//! Ce que le jeu garde d'une partie à l'autre (AD_AdvSystemSaveProc) : les
//! drapeaux spéciaux (fins vues, album complet, challenge ouvert et
//! réussi…), les mini-jeux déjà joués dans l'histoire
//! (AD_MiniGamePlayRecord), les réglages des options, ce qui a déjà été
//! lu (AD_BRead), les images de l'album vues (AD_CgLibCgRecord), et les parties sauvegardées (la carte mémoire), dans la
//! sauvegarde du portage.

use crate::album::{Album, Cg};
use crate::bread::BRead;
use crate::options::Settings;
use crate::save::Save;
use crate::savedata::Card;

/// Les clés de la sauvegarde : « nom=valeur,… » et un chiffre par mini-jeu.
const SPECIAL: &str = "special";
const PLAYED: &str = "played";
/// Les quinze octets des réglages, séparés par des virgules.
const OPTIONS: &str = "options";
/// Les fiches de ce qui a été lu, dans l'ordre de la liste : le numéro du
/// script (un octet) et le mot (quatre, petit-boutiste), en hexadécimal.
const READ: &str = "read";
/// Les fiches de l'album : vue, vidéo (un octet chacun), numéro (deux,
/// petit-boutiste), en hexadécimal.
const ALBUM: &str = "album";
/// L'ancienne clé du portage des seuls mini-jeux : le challenge réussi.
const LEGACY_CHALLENGE: &str = "challenge";

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Record {
    pub special: Vec<(Vec<u8>, i16)>,
    pub played: [bool; 5],
    pub settings: Settings,
    pub read: BRead,
    pub album: Album,
    pub card: Card,
    /// La sauvegarde du portage des seuls mini-jeux disait le challenge
    /// réussi : le drapeau de réussite sera posé.
    pub legacy_cleared: bool,
}

impl Record {
    pub fn read(save: &Save) -> Record {
        let special = save
            .get(SPECIAL)
            .split(',')
            .filter_map(|s| {
                let (name, value) = s.split_once('=')?;
                Some((name.as_bytes().to_vec(), value.parse().ok()?))
            })
            .collect();
        let digits = save.get(PLAYED).as_bytes();
        let played = std::array::from_fn(|k| digits.get(k) == Some(&b'1'));
        let bytes: Vec<u8> = save.get(OPTIONS).split(',').filter_map(|v| v.parse().ok()).collect();
        let settings = bytes.try_into().map_or_else(|_| Settings::default(), Settings::from_bytes);
        let read = BRead::from_records(from_hex(save.get(READ)).chunks_exact(5).map(|r| (r[0], u32::from_le_bytes([r[1], r[2], r[3], r[4]]))).collect());
        let cgs = from_hex(save.get(ALBUM)).chunks_exact(4).map(|c| Cg { seen: c[0] != 0, movie: c[1] != 0, id: i16::from_le_bytes([c[2], c[3]]) }).collect();
        Record { special, played, settings, read, album: Album { cgs }, card: Card::read(save), legacy_cleared: save.get(LEGACY_CHALLENGE) == "cleared" }
    }

    pub fn write(&self, save: &mut Save) {
        let special: Vec<String> = self.special.iter().map(|(n, v)| format!("{}={v}", String::from_utf8_lossy(n))).collect();
        save.set(SPECIAL, &special.join(","));
        let played: String = self.played.iter().map(|&p| if p { '1' } else { '0' }).collect();
        save.set(PLAYED, &played);
        let options: Vec<String> = self.settings.bytes().iter().map(u8::to_string).collect();
        save.set(OPTIONS, &options.join(","));
        save.set(READ, &to_hex(self.read.records().iter().flat_map(|&(file, word)| std::iter::once(file).chain(word.to_le_bytes()))));
        save.set(ALBUM, &to_hex(self.album.cgs.iter().flat_map(|c| [u8::from(c.seen), u8::from(c.movie)].into_iter().chain(c.id.to_le_bytes()))));
        self.card.write(save);
    }
}

fn from_hex(text: &str) -> Vec<u8> {
    text.as_bytes().chunks_exact(2).filter_map(|h| u8::from_str_radix(std::str::from_utf8(h).ok()?, 16).ok()).collect()
}

fn to_hex(bytes: impl Iterator<Item = u8>) -> String {
    bytes.map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Écrit puis relu, le relevé ne change pas.
    #[test]
    fn round_trip() {
        let mut save = Save::memory();
        let read = BRead::from_records(vec![(101, 0x0012_3400), (130, 0xFFFF_FFFF)]);
        let album = Album { cgs: vec![Cg { seen: true, movie: false, id: 3 }, Cg { seen: false, movie: true, id: 0x102 }] };
        let r = Record { special: vec![(b"ed_dokuro".to_vec(), 1), (b"comp_cg".to_vec(), 0)], played: [true, false, true, false, false], settings: Settings { text_speed: 8, ..Settings::default() }, read, album, card: Card::default(), legacy_cleared: false };
        r.write(&mut save);
        let back = Record::read(&save);
        assert_eq!(back.special, r.special);
        assert_eq!(back.played, r.played);
        assert_eq!(back.settings, r.settings);
        assert_eq!(back.read, r.read);
        assert_eq!(back.album, r.album);
    }
}
