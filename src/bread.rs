//! Ce que le joueur a déjà lu (AD_BRead*) : une liste de 3000 fiches au
//! plus, gardée avec les données système, rangée par numéro de script.
//! Chaque fiche est un mot de 32 bits : la sorte (0 les pages d'un passage
//! du script, 1 celles d'une route, 2 les réponses d'un choix), puis le
//! numéro du point d'entrée, la valeur de « switch », le compte des pages,
//! la route, et une valeur. Pour la sorte 0, le compte des pages est la
//! plus loin lue ; pour la sorte 1, la valeur est la plus loin lue dans la
//! route ; pour la sorte 2, les réponses déjà choisies (un bit chacune).
//!
//! Le jeu garde une fiche « en main » par sorte (0x1cf6280), qui évite de
//! chercher : elle porte les valeurs de la fiche, et une fiche toute neuve
//! y vaut « rien lu » alors que la fiche elle-même dit « page 0 lue » (voir
//! docs/moteur.md). Le portage la reproduit.

/// Les fiches au plus (0x1cf04c0 : 3000 × 8 octets).
const MAX: usize = 3000;

/// Une fiche en main : son rang dans la liste, la clé cherchée, les
/// valeurs lues (0x1cf6280 + 8 × sorte).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Held {
    index: Option<usize>,
    file: u8,
    p3: u8,
    p4: i8,
    p5: i8,
    p6: i8,
    value: i8,
}

/// Une page ou un choix : script (0x1da731c), numéro du point d'entrée
/// (0x1da731e), « switch » (0x1da7320), compte des pages (0x1da7322), route
/// (0x1da7324), compte dans la route (0x1da7326). Le jeu les passe en octets.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Place {
    pub file: i32,
    pub entry: i32,
    pub switch: i32,
    pub page: i32,
    pub route: i32,
    pub in_route: i32,
}

#[derive(Clone, Debug, Default)]
pub struct BRead {
    /// Les fiches dans l'ordre de la liste : (script, mot).
    records: Vec<(u8, u32)>,
    held: [Held; 3],
}

/// Seules les fiches sont gardées : deux relevés sont égaux quand elles le
/// sont.
impl PartialEq for BRead {
    fn eq(&self, other: &BRead) -> bool {
        self.records == other.records
    }
}

/// La clé d'une sorte : ((sorte × 64 + p3) × 64 + p4), suivie pour les
/// sortes 1 et 2 de p5 et p6 ; et le décalage qui la sépare du mot.
fn key(kind: usize, p3: u8, p4: i8, p5: i8, p6: i8) -> (u32, u32) {
    let k = (kind as i32 * 0x40 + i32::from(p3)).wrapping_mul(0x40).wrapping_add(i32::from(p4));
    if kind == 0 { (k as u32, 0x12) } else { (i32::from(p6).wrapping_add(k.wrapping_mul(0x40).wrapping_add(i32::from(p5)).wrapping_mul(0x10)) as u32, 8) }
}

impl BRead {
    /// Relu de la sauvegarde : les fiches, rien en main (AD_BReadLoadProc).
    pub fn from_records(records: Vec<(u8, u32)>) -> BRead {
        let mut records = records;
        records.truncate(MAX);
        BRead { records, held: Default::default() }
    }

    pub fn records(&self) -> &[(u8, u32)] {
        &self.records
    }

    /// AD_BReadSearch : la fiche de la clé, créée s'il n'y en a pas (à la
    /// fin des fiches de son script). None : la liste est pleine.
    fn search(&mut self, kind: usize, file: u8, p3: u8, p4: i8, p5: i8, p6: i8) -> Option<usize> {
        let (wanted, shift) = key(kind, p3, p4, p5, p6);
        let h = self.held[kind];
        if h.index.is_some() && h.file == file && key(kind, h.p3, h.p4, h.p5, h.p6).0 == wanted {
            return h.index;
        }
        self.held[kind] = Held { index: None, file, p3, p4, p5, p6, value: h.value };
        let start = self.records.iter().position(|r| r.0 >= file).unwrap_or(self.records.len());
        let end = start + self.records[start..].iter().take_while(|r| r.0 == file).count();
        if let Some(k) = (start..end).find(|&k| self.records[k].1 >> shift == wanted) {
            let word = self.records[k].1;
            let h = &mut self.held[kind];
            h.p5 = ((word >> 12) & 0x3F) as i8;
            h.p6 = ((word >> 8) & 0xF) as i8;
            h.value = word as i8;
            h.index = Some(k);
            return Some(k);
        }
        // Une fiche neuve. Placée avant les fiches d'un autre script, elle
        // déplace celles-ci : le jeu oublie alors les autres fiches en main.
        // Ajoutée au bout de la liste, elle se crée même quand la liste
        // devient pleine (la recherche échoue tout de même).
        let at_end = end == self.records.len();
        if !at_end && self.records.len() >= MAX || at_end && self.records.len() > MAX - 1 {
            return None;
        }
        self.records.insert(end, (file, wanted << shift));
        if !at_end {
            for (k, h) in self.held.iter_mut().enumerate() {
                if k != kind {
                    h.index = None;
                }
            }
        }
        let h = &mut self.held[kind];
        h.index = Some(end);
        h.value = 0;
        match kind {
            0 => h.p5 = -1,
            1 => h.value = -1,
            _ => {}
        }
        (!at_end || self.records.len() < MAX).then_some(end)
    }

    /// AD_BReadCheck : la page (sorte 0 ou 1) ou la réponse `answer`
    /// (sorte 2) a-t-elle déjà été lue ? Une liste pleine répond oui.
    pub fn check(&mut self, kind: usize, at: &Place, answer: i32) -> bool {
        let (file, p3, p4, p5) = (at.file as u8, at.entry as u8, at.switch as i8, at.page as i8);
        let (p6, p7) = if kind == 1 { (at.route as i8, at.in_route as i8) } else { (0, answer as i8) };
        if self.search(kind, file, p3, p4, p5, p6).is_none() {
            return true;
        }
        let h = &self.held[kind];
        match kind {
            2 => (i32::from(h.value) & (1 << (p7 & 0x1F))) != 0,
            1 => p7 <= h.value,
            _ => p5 <= h.p5,
        }
    }

    /// AD_BReadWrite : la page (sorte 0 ou 1) ou la réponse `answer`
    /// (sorte 2) est lue.
    pub fn write(&mut self, kind: usize, at: &Place, answer: i32) {
        let (file, p3, p4, p5) = (at.file as u8, at.entry as u8, at.switch as i8, at.page as i8);
        let (p6, p7) = if kind == 1 { (at.route as i8, at.in_route as i8) } else { (0, answer as i8) };
        let Some(k) = self.search(kind, file, p3, p4, p5, p6) else { return };
        match kind {
            0 if p5 <= self.held[0].p5 => return,
            0 => self.held[0].p5 = p5,
            1 if p7 <= self.held[1].value => return,
            1 => self.held[1].value = p7,
            _ => self.held[2].value |= (1i32 << (p7 & 0x1F)) as i8,
        }
        let value = i32::from(self.held[kind].value);
        let word = (kind as i32 * 0x40 + i32::from(p3))
            .wrapping_mul(0x40)
            .wrapping_add(i32::from(p4))
            .wrapping_mul(0x40)
            .wrapping_add(i32::from(p5))
            .wrapping_mul(0x10)
            .wrapping_add(i32::from(p6))
            .wrapping_mul(0x100)
            .wrapping_add(value);
        self.records[k].1 = word as u32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les pages lues d'un passage, une route, un choix ; et la fiche toute
    /// neuve qui, relue, dit la page 0 lue.
    #[test]
    fn pages_routes_choices() {
        let mut r = BRead::default();
        let mut at = Place { file: 101, entry: 2, switch: 0, page: 0, route: -1, in_route: 0 };
        assert!(!r.check(0, &at, 0));
        r.write(0, &at, 0);
        at.page = 1;
        r.write(0, &at, 0);
        at.page = 2;
        assert!(!r.check(0, &at, 0));
        at.page = 1;
        assert!(r.check(0, &at, 0));
        at = Place { route: 3, in_route: 4, ..at };
        r.write(1, &at, 0);
        assert!(r.check(1, &at, 0));
        at.in_route = 5;
        assert!(!r.check(1, &at, 0));
        r.write(2, &at, 1);
        assert!(r.check(2, &at, 1));
        assert!(!r.check(2, &at, 0));
        let mut fresh = BRead::default();
        let other = Place { file: 7, ..Place::default() };
        assert!(!fresh.check(0, &other, 0));
        let mut reloaded = BRead::from_records(fresh.records().to_vec());
        assert!(reloaded.check(0, &other, 0));
    }
}
