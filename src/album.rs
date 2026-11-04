//! Ce que l'album sait déjà (AD_CgLib*) : les images et les vidéos qu'il
//! range (cglib_entry, cglib_movie_entry), et celles que le joueur a vues
//! (AD_CgLibCgRecord : à chaque changement de décor, à chaque vidéo). Gardé
//! avec les données système, comme dans le jeu (0x1ba9000 : 0x82 fiches).
//! Et l'image bonus (bonus_cg_*_entry), montrée à la fin du jeu quand ses
//! conditions sont réunies (AD_CgBonusGetJudg).

use crate::movie;
use crate::story::World;

/// Les fiches au plus.
const MAX: usize = 0x82;

/// Une fiche : vue ou non, image (0) ou vidéo (1), son numéro (le rang du
/// décor dans bg_entry, ou de la vidéo dans movie_entry).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cg {
    pub seen: bool,
    pub movie: bool,
    pub id: i16,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Album {
    pub cgs: Vec<Cg>,
}

/// Le rang d'un décor (AD_BGBackNum) ou d'une vidéo (AD_MVBackNum).
pub fn bg_id(world: &World, name: &[u8]) -> Option<i16> {
    world.bgs.iter().position(|b| b.0 == name).map(|k| k as i16)
}

/// Les vidéos des scripts (movie_entry), puis celles de l'exécutable (« op »
/// est aussi dans l'album). Le jeu les range dans une seule table ; les
/// numéros ne servent qu'à l'album, ceux des scripts d'abord pour garder les
/// sauvegardes du portage.
pub fn movie_id(world: &World, name: &[u8]) -> Option<i16> {
    movie_names(world).position(|m| m == name).map(|k| k as i16)
}

/// Le nom d'une vidéo d'après son numéro.
pub fn movie_name(world: &World, id: i16) -> Option<&[u8]> {
    movie_names(world).nth(usize::try_from(id).ok()?)
}

fn movie_names(world: &World) -> impl Iterator<Item = &[u8]> {
    world.movies.iter().map(|m| &m.0[..]).chain(movie::BUILT_IN.iter().map(|m| m.0.as_bytes()))
}

impl Album {
    /// AD_CgLibCgEntry, pour chaque image et vidéo de l'album : une fiche
    /// pas encore vue, si elle manque.
    pub fn register(&mut self, world: &World) {
        for e in &world.cg {
            let id = if e.movie { movie_id(world, &e.name) } else { bg_id(world, &e.name) };
            let Some(id) = id else { continue };
            if self.cgs.len() < MAX && !self.cgs.iter().any(|c| c.id == id && c.movie == e.movie) {
                self.cgs.push(Cg { seen: false, movie: e.movie, id });
            }
        }
    }

    /// AD_CgLibCgRecord : l'image ou la vidéo est vue (si l'album la range).
    pub fn record(&mut self, id: i16, movie: bool) {
        if let Some(c) = self.cgs.iter_mut().find(|c| c.id == id && c.movie == movie) {
            c.seen = true;
        }
    }

    /// AD_CgLibCgCheck2 : vue (Some(true)), pas encore (Some(false)), ou
    /// pas dans l'album.
    fn seen(&self, id: i16, movie: bool) -> Option<bool> {
        self.cgs.iter().find(|c| c.id == id && c.movie == movie).map(|c| c.seen)
    }

    /// Vue (AD_CgLibCgCheck2 == 1).
    pub fn is_seen(&self, id: i16, movie: bool) -> bool {
        self.seen(id, movie) == Some(true)
    }

    /// AD_CgBonusGetJudg(2), à l'ouverture de l'album : l'image bonus de la
    /// troisième sorte, quand il ne reste qu'une place de l'album pas encore
    /// vue (le jeu compte chaque place dont l'image, ou une vidéo de même
    /// numéro, n'est pas vue).
    pub fn complete_bonus(&self, world: &World) -> Option<i16> {
        let (kind, name, _, _) = world.cg_bonus.as_ref()?;
        if *kind != 2 {
            return None;
        }
        let unseen = world.cg.iter().filter_map(|e| if e.movie { movie_id(world, &e.name) } else { bg_id(world, &e.name) }).filter(|&id| self.seen(id, false) == Some(false) || self.seen(id, true) == Some(false)).count();
        if unseen == 1 { bg_id(world, name) } else { None }
    }

    /// AD_CgBonusGetJudg(0) puis (1) : l'image bonus à montrer, si elle n'a
    /// pas encore été vue et que son drapeau (ordinaire, puis spécial) a la
    /// valeur voulue. (La troisième sorte, « l'album complet sauf elle », ne
    /// se juge pas ici.)
    pub fn bonus(&self, world: &World) -> Option<i16> {
        let (kind, name, flag, value) = world.cg_bonus.as_ref()?;
        let id = bg_id(world, name)?;
        let special = match kind {
            0 => false,
            1 => true,
            _ => return None,
        };
        (self.seen(id, false) == Some(false) && world.flags.get(flag, special) == *value).then_some(id)
    }
}
