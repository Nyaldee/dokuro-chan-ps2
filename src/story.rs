//! Ce que préparent les scripts (fonctions STCM2_* de l'exécutable) : les
//! lots que joue ensuite le moteur (une page de texte et tout ce qui
//! l'accompagne), les drapeaux de l'histoire, et ce qu'ils inscrivent pour
//! la carte, les choix, l'album, les résultats. Voir docs/moteur.md.

use crate::stcm2::{Args, Host};

/// Les lots d'un passage du script (0x40).
pub const MAX_BATCHES: usize = 64;
/// Les personnages d'un lot.
pub const MAX_CHARAS: usize = 6;
/// Les morceaux d'un personnage.
pub const PARTS: usize = 5;

/// `n` unités de temps des scripts en images (n × 7,5, arrondi vers 0).
pub fn ticks(n: i32) -> i32 {
    (f64::from(n) * 7.5) as i32
}

/// Un retard de son ou de voix : n × 7,5 + 1 images ; pendant un changement
/// de scène, s'y ajoutent la moitié de sa durée et 15.
fn delay(n: i32, scene: Option<i32>) -> i32 {
    let d = f64::from(n) * 7.5 + 1.0;
    match scene {
        Some(dur) => (d + f64::from(dur >> 1) + 15.0) as i32,
        None => d as i32,
    }
}

/// La sorte d'un lot (+0x190).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// Une page de texte qui attend ○.
    Text = 0,
    /// Rien à lire : le lot est fait d'autre chose.
    Silent = 1,
    Menu = 2,
    /// Un choix qui n'attend pas la fin du texte.
    MenuNoWait = 3,
    Map = 4,
    Result = 7,
    Distribute = 8,
    Mogura = 9,
    Catch = 10,
    Card = 11,
    Run = 12,
    Bat = 13,
    Movie = 14,
    GameEnd = 15,
}

/// Une étape de caméra (0x16 octets) : durée (−1 : aucune), zoom (−1 :
/// inchangé), position, angles, tremblement (x, y ; 0xff : aucun), couleur
/// (−1 : aucune).
#[derive(Clone, Copy, Debug)]
pub struct Target {
    pub dur: i16,
    pub zoom: i16,
    pub x: i16,
    pub y: i16,
    pub angle: [i16; 3],
    pub shake: (u8, u8),
    pub rgb: [i16; 3],
}

impl Target {
    const NONE: Target = Target { dur: -1, zoom: -1, x: 0, y: 0, angle: [0; 3], shake: (0xFF, 0), rgb: [-1, 0, 0] };
}

/// Un fondu d'un plan (+0x34c et suivants) : 0 apparaît, 1 s'efface, 2
/// éclair ; deux valeurs de mélange, la couleur (CLR_Get), l'opacité, la
/// durée.
#[derive(Clone, Copy, Debug)]
pub struct Fade {
    pub kind: i16,
    pub a: u8,
    pub b: u8,
    pub color: i16,
    pub alpha: u8,
    pub dur: i16,
}

/// Le décor du lot (+0x22c) : nom, durée du fondu, retard. (Le moteur
/// connaît d'autres transitions que le fondu enchaîné, la sorte 0 ; aucun
/// script ne s'en sert.)
#[derive(Clone, Debug)]
pub struct Bg {
    pub name: Vec<u8>,
    pub dur: i16,
    pub delay: i16,
}

/// Ce que le lot fait d'un personnage (0x248 octets). `tally` compte les
/// instructions reçues (unités : 1 tout afficher, 2 tout effacer ;
/// dizaines : 1 état, 2 état en fondu ; centaines : 1 apparition, 2
/// disparition, 3 effacement, 4 déplacement).
#[derive(Clone, Debug, Default)]
pub struct Chara {
    pub tally: i16,
    pub name: Vec<u8>,
    /// L'image et l'expression de chaque morceau.
    pub images: [Vec<u8>; PARTS],
    pub emotions: [Vec<u8>; PARTS],
    /// +0x20e à +0x215 : déplacement, mouvement, disparition, fondus de
    /// l'image et de l'expression, animation, couleur, compte.
    pub flags: [u8; 8],
    /// L'ordre (+0x216).
    pub index: i16,
    /// L'état visé de chaque morceau (+0x218 : 3 apparaît, 4 disparaît ;
    /// le second : 0 apparition, 1 disparition, 2 effacement).
    pub states: [i16; PARTS],
    /// Les positions (+0x222 : arrivée ; +0x224 : départ).
    pub values: [i16; PARTS],
    /// Les durées (+0x22c ; le second, −1).
    pub durs: [i16; PARTS],
    /// La couleur de chaque morceau (0xff : aucune).
    pub rgb: [[u8; 3]; PARTS],
    /// Le retard (+0x246 ; −1 : aucun).
    pub delay: i16,
}

impl Chara {
    fn digit(&self, place: i32) -> i32 {
        i32::from(self.tally) / 10i32.pow(place as u32 - 1) % 10
    }
}

/// Un lot (0x1268 octets à partir de 0x1da7770).
#[derive(Clone, Debug)]
pub struct Batch {
    pub kind: Kind,
    /// Le texte : les lignes, chacune finie par « #n ».
    pub text: Vec<u8>,
    pub speaker: Option<Vec<u8>>,
    pub info: Option<Vec<u8>>,
    /// Interdictions posées et levées (4 menu, 8 texte, 0x10 fenêtre).
    pub restrict: u16,
    pub restoff: u16,
    /// La route (−1 : inchangée ; −2 : fin de route).
    pub route: i16,
    pub bg: Option<Bg>,
    /// Les deux caméras, cinq étapes chacune.
    pub targets: [[Target; 5]; 2],
    /// Les fondus des sept plans.
    pub fades: [Option<Fade>; 7],
    /// Les attentes : apparitions (+0x3a8), décor et transitions (+0x3aa),
    /// disparitions (+0x3ac), imposée (+0x3ae).
    pub wait_app: i32,
    pub wait_bg: i32,
    pub wait_del: i32,
    pub wait: i32,
    pub movie: Vec<u8>,
    /// Musique : numéro (−1 : arrêt) et retard.
    pub bgm: Option<(i32, i32)>,
    /// Bruitages (−1 : arrêt) et retards.
    pub se: Vec<(i32, i32)>,
    pub loop_se: [Option<(i32, i32)>; 2],
    /// La voix, le bruitage en flux (numéros de SE.VBG) et leur retard.
    pub voice: Option<(u32, i32)>,
    pub stream: Option<(u32, i32)>,
    pub stream_cut: bool,
    pub charas: Vec<Chara>,
}

impl Default for Batch {
    /// AD_AdvBthFormat.
    fn default() -> Batch {
        Batch {
            kind: Kind::Silent,
            text: Vec::new(),
            speaker: None,
            info: None,
            restrict: 0,
            restoff: 0,
            route: -1,
            bg: None,
            targets: [[Target::NONE; 5]; 2],
            fades: [None; 7],
            wait_app: 0,
            wait_bg: 0,
            wait_del: 0,
            wait: 0,
            movie: Vec::new(),
            bgm: None,
            se: Vec::new(),
            loop_se: [None; 2],
            voice: None,
            stream: None,
            stream_cut: false,
            charas: Vec::new(),
        }
    }
}

impl Batch {
    /// La durée d'un éclair du plan 3 (un changement de scène), qui
    /// retarde les voix du lot.
    pub fn flash(&self) -> Option<i32> {
        self.fades[3].filter(|f| f.kind == 2).map(|f| i32::from(f.dur))
    }
}

/// Un drapeau : sa valeur, son nom (16 octets). (Le jeu garde aussi une
/// valeur de départ, que ne lit aucune partie qui sert.)
#[derive(Clone, Debug, PartialEq)]
pub struct Flag {
    pub value: i16,
    pub name: Vec<u8>,
}

/// Les drapeaux de l'histoire (42) et les drapeaux gardés d'une partie à
/// l'autre (15), à places fixes (AD_Flag*).
#[derive(Clone, Debug)]
pub struct Flags {
    pub normal: Vec<Option<Flag>>,
    pub special: Vec<Option<Flag>>,
}

impl Default for Flags {
    /// AD_Flaginit : « select » et « switch » existent d'emblée.
    fn default() -> Flags {
        let mut f = Flags { normal: vec![None; 0x2A], special: vec![None; 0x0F] };
        f.make(b"select", false);
        f.make(b"switch", false);
        f
    }
}

impl Flags {
    fn table(&mut self, special: bool) -> &mut Vec<Option<Flag>> {
        if special { &mut self.special } else { &mut self.normal }
    }

    fn find(&self, name: &[u8], special: bool) -> Option<usize> {
        let t = if special { &self.special } else { &self.normal };
        t.iter().position(|f| f.as_ref().is_some_and(|f| f.name == name))
    }

    /// AD_FlagMake : un drapeau à la première place libre (rien s'il existe).
    pub fn make(&mut self, name: &[u8], special: bool) {
        if self.find(name, special).is_some() {
            return;
        }
        let name = name[..name.len().min(15)].to_vec();
        if let Some(slot) = self.table(special).iter_mut().find(|f| f.is_none()) {
            *slot = Some(Flag { value: 0, name });
        }
    }

    /// AD_FlagNumber : la valeur (0 si le drapeau n'existe pas).
    pub fn get(&self, name: &[u8], special: bool) -> i16 {
        let t = if special { &self.special } else { &self.normal };
        self.find(name, special).and_then(|k| t[k].as_ref()).map_or(0, |f| f.value)
    }

    pub fn set(&mut self, name: &[u8], value: i16, special: bool) {
        if let Some(k) = self.find(name, special)
            && let Some(f) = &mut self.table(special)[k]
        {
            f.value = value;
        }
    }

    /// AD_FlagChange : ajoute à la valeur.
    pub fn add(&mut self, name: &[u8], n: i16, special: bool) {
        let v = self.get(name, special);
        self.set(name, v.wrapping_add(n), special);
    }
}

/// Un script et un point d'entrée.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Entry {
    pub file: i32,
    pub name: Vec<u8>,
}

/// Un lieu de la carte (AD_MapPos*).
#[derive(Clone, Debug, Default)]
pub struct Place {
    pub entry: Entry,
    pub data: i32,
    pub state: i32,
    pub comment: Vec<u8>,
    pub comment_n: i32,
}

/// Une image ou une vidéo de l'album (AD_CgLibEntry) : page, sorte (0
/// image, 1 vidéo), place, nom, planche des vignettes et rectangle.
#[derive(Clone, Debug)]
pub struct CgEntry {
    pub page: i32,
    pub movie: bool,
    pub slot: i32,
    pub name: Vec<u8>,
    pub sheet: Vec<u8>,
    pub rect: [i16; 4],
}

/// chara_pt_set : image, personnage, nom ; rectangle ; place du visage ;
/// deux réglages.
pub type Pattern = (Vec<u8>, Vec<u8>, Vec<u8>, [i32; 4], [i32; 4]);
/// chara_emo_set : expression, personnage, image, planche ; rectangle.
pub type Emotion = (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, [i32; 4]);

/// Ce que les scripts inscrivent, en plus des lots.
#[derive(Default)]
pub struct World {
    pub batches: Vec<Batch>,
    pub flags: Flags,
    /// Le script à relancer (file_change, thread_change).
    pub next: Entry,
    /// Les scripts de départ : l'histoire (start_stcm), la carte, le
    /// scénario bonus, et ceux qu'init lance (init_stcm_set).
    pub start: Entry,
    pub map_script: Entry,
    pub bonus: Entry,
    pub init_scripts: Vec<Entry>,
    /// Le titre de la sauvegarde.
    pub save_name: Vec<u8>,
    /// Les réponses d'un choix, et le drapeau qui reçoit la réponse.
    pub menu: [Option<Vec<u8>>; 10],
    pub menu_flag: Vec<u8>,
    pub places: Vec<Place>,
    pub marks: Vec<(i32, i32)>,
    pub cg: Vec<CgEntry>,
    pub movies: Vec<(Vec<u8>, Vec<u8>)>,
    /// Les décors (bg_entry), personnages (chara_entry) et images
    /// (chara_gp_entry : nom, entrée de CHARA.UNI).
    pub bgs: Vec<(Vec<u8>, i32)>,
    pub charas: Vec<Vec<u8>>,
    pub images: Vec<(Vec<u8>, i32)>,
    /// Les images des personnages et leurs expressions.
    pub patterns: Vec<Pattern>,
    pub emotions: Vec<Emotion>,
    /// Les bulles du menu principal par personnage (chara_txt_set).
    pub chara_texts: Vec<(i32, i32, i32, Vec<u8>)>,
    pub results: Vec<(i32, Vec<u8>, [i32; 3])>,
    pub ending_flags: Vec<(i32, Vec<u8>)>,
    pub ending_bg: Vec<u8>,
    pub bonus_cg: Vec<(i32, i32, Vec<u8>)>,
    /// L'image bonus de l'album (AD_CgBonusEntry : la première posée) :
    /// sorte (0 drapeau, 1 drapeau spécial, 2 album complet), décor,
    /// drapeau, valeur voulue.
    pub cg_bonus: Option<(i32, Vec<u8>, Vec<u8>, i16)>,
    /// Le drapeau spécial qui ouvre le scénario bonus
    /// (bonus_scenario_openflag : « challenge », le challenge réussi).
    pub bonus_open: Vec<u8>,
    pub distribute: i32,
    /// Le dernier mini-jeu : personnage choisi, gagné (1) ou perdu.
    pub selchara: i32,
    pub won: i32,
    pub challenge_open: Vec<u8>,
    pub challenge_clear: Vec<u8>,
    /// Les couleurs des fondus (color_set, CLR_Set ; 40).
    pub colors: Vec<Option<[u8; 3]>>,
    /// Un nombre au hasard pour `rand`.
    pub random: i32,
}

impl World {
    /// Le lot que remplit le script (le dernier ; un vide au départ).
    fn batch(&mut self) -> &mut Batch {
        if self.batches.is_empty() {
            self.batches.push(Batch::default());
        }
        self.batches.last_mut().unwrap()
    }

    /// AD_AdvBthCharaNum : le personnage du lot à ce nom (ou une place
    /// libre).
    fn chara(&mut self, name: &[u8]) -> Option<&mut Chara> {
        let b = self.batch();
        if let Some(k) = b.charas.iter().position(|c| c.tally != 0 && c.name == name) {
            return Some(&mut b.charas[k]);
        }
        if b.charas.len() >= MAX_CHARAS {
            return b.charas.iter_mut().find(|c| c.tally == 0);
        }
        b.charas.push(Chara { delay: -1, rgb: [[0xFF; 3]; PARTS], ..Chara::default() });
        b.charas.last_mut()
    }
}

fn text(a: &Args, k: usize) -> Vec<u8> {
    a.text(k).to_vec()
}

/// Les numéros des instructions du jeu (table 0x35bf60).
mod op {
    pub const CHARA_GP_ENTRY: u32 = 0xC8;
    pub const CHARA_ENTRY: u32 = 0xC9;
    pub const BG_ENTRY: u32 = 0xCA;
    pub const MOVIE_ENTRY: u32 = 0xCB;
    pub const CGLIB_ENTRY: u32 = 0xCC;
    pub const THUM_GP_ENTRY: u32 = 0xD0;
    pub const INIT_STCM_SET: u32 = 0xD1;
    pub const CHARA_PT_SET: u32 = 0xD2;
    pub const CHARA_EMO_SET: u32 = 0xD3;
    pub const COLOR_SET: u32 = 0xD5;
    pub const CGLIB_MOVIE_ENTRY: u32 = 0xD7;
    pub const BONUS_CG_FLAG_ENTRY: u32 = 0xD9;
    pub const BONUS_CG_SPFLAG_ENTRY: u32 = 0xDA;
    pub const BONUS_CG_COMP_ENTRY: u32 = 0xDB;
    pub const TRCHARA_UP_ALL: u32 = 0xE1;
    pub const TRCHARA_DEL_ALL: u32 = 0xE2;
    pub const EZCHARA_STATESET: u32 = 0x104;
    pub const EZCHARA_NEXTSTATE: u32 = 0x105;
    pub const EZCHARA_APP: u32 = 0x106;
    pub const EZCHARA_DIS: u32 = 0x107;
    pub const EZCHARA_DEL: u32 = 0x108;
    pub const EZCHARA_MOVE: u32 = 0x109;
    pub const MAINTXT_PRINT: u32 = 0x118;
    pub const BATCH: u32 = 0x119;
    pub const SPEAKER_CHANGE: u32 = 0x11A;
    pub const MENUTXT_SET: u32 = 0x11B;
    pub const MENU_SET: u32 = 0x11C;
    pub const THREAD_CHANGE: u32 = 0x11D;
    pub const FILE_CHANGE: u32 = 0x11E;
    pub const START_STCM: u32 = 0x11F;
    pub const MAPSTCM_CHANGE: u32 = 0x120;
    pub const START_MAPSTCM: u32 = 0x121;
    pub const TXTWAITSET: u32 = 0x122;
    pub const GAME_END: u32 = 0x123;
    pub const SAVE_NAME: u32 = 0x124;
    pub const MENU_NOWAIT: u32 = 0x125;
    pub const INFOR_WIN: u32 = 0x126;
    pub const BONUS_STCM: u32 = 0x12A;
    pub const BONUS_SCENARIO_OPENFLAG: u32 = 0x12B;
    pub const BG_PRINT_CHANGE: u32 = 0x12C;
    pub const TARGET_POINT_SET: u32 = 0x130;
    pub const TARGET_ANGLE_SET: u32 = 0x131;
    pub const TARGET_SHAKE_SET: u32 = 0x132;
    pub const TARGET_RGB_SET: u32 = 0x133;
    pub const TARGET_COUNT_SET: u32 = 0x134;
    pub const EZBG_CHANGE: u32 = 0x136;
    pub const BLACKOUT_START: u32 = 0x140;
    pub const BLACKOUT_END: u32 = 0x141;
    pub const BLACKOUT_FLASH: u32 = 0x142;
    pub const STRSE_WAITCUT: u32 = 0x149;
    pub const BGM_PLAY: u32 = 0x14A;
    pub const BGM_STOP: u32 = 0x14B;
    pub const SE_PLAY: u32 = 0x14C;
    pub const SE_STOP: u32 = 0x14D;
    pub const VOICE_PLAY: u32 = 0x14E;
    pub const LOOPSE_PLAY: u32 = 0x14F;
    pub const LOOPSE_STOP: u32 = 0x150;
    pub const MOVIE_PLAY: u32 = 0x151;
    pub const STRSE_PLAY: u32 = 0x152;
    pub const EZEFFECT_SCENECHANGE: u32 = 0x15E;
    pub const EZEFFECT_SCENECHANGE2: u32 = 0x15F;
    pub const FLAG_MAKE: u32 = 0x168;
    pub const FLAG_SET: u32 = 0x16A;
    pub const FLAG_CHANGE: u32 = 0x16B;
    pub const FLAG_NUMBER: u32 = 0x16C;
    pub const SPFLAG_MAKE: u32 = 0x16D;
    pub const SPFLAG_SET: u32 = 0x16F;
    pub const SPFLAG_CHANGE: u32 = 0x170;
    pub const SPFLAG_NUMBER: u32 = 0x171;
    pub const CDMODE_CHECK: u32 = 0x17C;
    pub const RAND: u32 = 0x17D;
    pub const FLAG_CMP: u32 = 0x17E;
    pub const MAP_POS_DATASET: u32 = 0x186;
    pub const MAP_SELMODE: u32 = 0x187;
    pub const MAP_POS_STATESET: u32 = 0x189;
    pub const MAP_POS_COMMENTSET: u32 = 0x18A;
    pub const MAP_MARK_POS_SET: u32 = 0x18C;
    pub const RESTRICT_MENU: u32 = 0x1AE;
    pub const RESTOFF_MENU: u32 = 0x1AF;
    pub const RESTRICT_TXT: u32 = 0x1B0;
    pub const RESTOFF_TXT: u32 = 0x1B1;
    pub const RESTRICT_WIN: u32 = 0x1B2;
    pub const RESTOFF_WIN: u32 = 0x1B3;
    pub const ROUTE: u32 = 0x1B8;
    pub const ROUTEEND: u32 = 0x1B9;
    pub const CHARA_TXT_SET: u32 = 0x1C2;
    pub const MOGURA_MODE: u32 = 0x1CC;
    pub const CATCH_MODE: u32 = 0x1CD;
    pub const CARD_MODE: u32 = 0x1CE;
    pub const RUN_MODE: u32 = 0x1CF;
    pub const BAT_MODE: u32 = 0x1D0;
    pub const MINIGAME_SELCHARA_GET: u32 = 0x1D1;
    pub const MINIGAME_RESULT_GET: u32 = 0x1D2;
    pub const MINIGAME_BONUS_CG_SET: u32 = 0x1D3;
    pub const MINIGAME_CHALLENGE_OPENFLAG_SET: u32 = 0x1D4;
    pub const MINIGAME_CHALLENGE_CLEARFLAG_SET: u32 = 0x1D5;
    pub const RESULT_MODE: u32 = 0x1E0;
    pub const RESULT_DATA_SET: u32 = 0x1E1;
    pub const DISTRIBUTE_MODE: u32 = 0x1E4;
    pub const ENDING_CHECK_FLAG_SET: u32 = 0x1EA;
    pub const ENDING_CHECK_BG_SET: u32 = 0x1EB;
}

impl Host for World {
    fn op(&mut self, code: u32, a: &mut Args) {
        use op::*;
        let i = |k: usize| a.int(k);
        match code {
            // Le texte et l'enchaînement des lots.
            MAINTXT_PRINT => {
                let b = self.batch();
                b.text.extend_from_slice(a.text(0));
                b.text.extend_from_slice(b"#n");
                if b.kind == Kind::Silent {
                    b.kind = Kind::Text;
                }
            }
            BATCH => {
                if self.batches.len() < MAX_BATCHES {
                    self.batches.push(Batch::default());
                }
            }
            SPEAKER_CHANGE => self.batch().speaker = Some(text(a, 0)),
            INFOR_WIN => self.batch().info = Some(text(a, 0)),
            TXTWAITSET => self.batch().wait = i(0),
            MENUTXT_SET => {
                if let Ok(k) = usize::try_from(i(1))
                    && k < 10
                {
                    self.menu[k] = Some(text(a, 0));
                }
                let b = self.batch();
                if b.kind != Kind::MenuNoWait {
                    b.kind = Kind::Menu;
                }
            }
            MENU_SET => {
                self.menu_flag = text(a, 0);
                let b = self.batch();
                if b.kind != Kind::MenuNoWait {
                    b.kind = Kind::Menu;
                }
            }
            MENU_NOWAIT => self.batch().kind = Kind::MenuNoWait,
            RESTRICT_MENU | RESTRICT_TXT | RESTRICT_WIN => self.batch().restrict |= 4 << ((code - RESTRICT_MENU) / 2),
            RESTOFF_MENU | RESTOFF_TXT | RESTOFF_WIN => self.batch().restoff |= 4 << ((code - RESTOFF_MENU) / 2),
            ROUTE => self.batch().route = i(0) as i16,
            ROUTEEND => self.batch().route = -2,
            // STCM2_save_name : « %s#n ».
            SAVE_NAME => {
                self.save_name = text(a, 0);
                self.save_name.extend_from_slice(b"#n");
            }

            // Les scripts à suivre.
            THREAD_CHANGE => self.next.name = text(a, 0),
            FILE_CHANGE => self.next = Entry { file: i(0), name: text(a, 1) },
            START_STCM => self.start = Entry { file: i(0), name: text(a, 1) },
            START_MAPSTCM | MAPSTCM_CHANGE => self.map_script = Entry { file: i(0), name: text(a, 1) },
            BONUS_STCM => self.bonus = Entry { file: i(0), name: text(a, 1) },
            INIT_STCM_SET => {
                if self.init_scripts.len() < 10 {
                    self.init_scripts.push(Entry { file: i(0), name: text(a, 1) });
                }
            }

            // Les sortes de lots.
            MAP_SELMODE => self.batch().kind = Kind::Map,
            RESULT_MODE => self.batch().kind = Kind::Result,
            DISTRIBUTE_MODE => {
                self.batch().kind = Kind::Distribute;
                self.distribute = i(0);
            }
            MOGURA_MODE => self.batch().kind = Kind::Mogura,
            CATCH_MODE => self.batch().kind = Kind::Catch,
            CARD_MODE => self.batch().kind = Kind::Card,
            RUN_MODE => self.batch().kind = Kind::Run,
            BAT_MODE => self.batch().kind = Kind::Bat,
            GAME_END => self.batch().kind = Kind::GameEnd,
            MOVIE_PLAY => {
                let b = self.batch();
                b.kind = Kind::Movie;
                b.movie = a.text(0).to_vec();
            }

            // Le décor, la caméra, les fondus.
            BG_PRINT_CHANGE => {
                let b = self.batch();
                b.bg = Some(Bg { name: text(a, 0), dur: i(3) as i16, delay: i(4) as i16 });
                b.wait_bg = b.wait_bg.max(i(3) + 0x1E);
            }
            EZBG_CHANGE => {
                // (Le jeu tire aussi une variante au hasard, rand() % 9, qui
                // ne sert pas au fondu enchaîné.)
                let b = self.batch();
                b.bg = Some(Bg { name: text(a, 0), dur: (i(1) * 0x1E) as i16, delay: 0 });
                b.wait_bg = b.wait_bg.max(i(1) * 0x1E + 10);
            }
            TARGET_POINT_SET | TARGET_COUNT_SET | TARGET_SHAKE_SET | TARGET_ANGLE_SET | TARGET_RGB_SET => {
                let (c, s) = (i(0), i(1));
                if !(0..2).contains(&c) || !(0..5).contains(&s) {
                    return;
                }
                let t = &mut self.batch().targets[c as usize][s as usize];
                match code {
                    TARGET_POINT_SET => (t.x, t.y, t.zoom) = (i(2) as i16, i(3) as i16, i(4) as i16),
                    TARGET_COUNT_SET => t.dur = i(2) as i16,
                    TARGET_SHAKE_SET => t.shake = (i(2) as u8, i(3) as u8),
                    TARGET_ANGLE_SET => t.angle = [i(4) as i16, i(3) as i16, i(2) as i16],
                    _ => t.rgb = [i(2) as i16, i(3) as i16, i(4) as i16],
                }
            }
            BLACKOUT_START | BLACKOUT_FLASH => {
                if let Ok(p) = usize::try_from(i(3))
                    && p < 7
                {
                    let kind = if code == BLACKOUT_START { 0 } else { 2 };
                    self.batch().fades[p] = Some(Fade { kind, a: i(0) as u8, b: i(1) as u8, color: i(2) as i16, alpha: (i(4) >> 1) as u8, dur: i(5) as i16 });
                }
            }
            BLACKOUT_END => {
                if let Ok(p) = usize::try_from(i(2))
                    && p < 7
                {
                    let alpha = self.batch().fades[p].map_or(0, |f| f.alpha);
                    let color = self.batch().fades[p].map_or(0, |f| f.color);
                    self.batch().fades[p] = Some(Fade { kind: 1, a: i(0) as u8, b: i(1) as u8, color, alpha, dur: i(3) as i16 });
                }
            }
            EZEFFECT_SCENECHANGE | EZEFFECT_SCENECHANGE2 => {
                // Un éclair sur le plan 3 : couleur, deux valeurs de mélange
                // (3, 3 pour la première version), durée (n / 2 × 7,5).
                let dur = ticks(i(1) >> 1);
                let (ma, mb) = if code == EZEFFECT_SCENECHANGE { (3, 3) } else { (i(2) as u8, i(3) as u8) };
                let b = self.batch();
                b.fades[3] = Some(Fade { kind: 2, a: ma, b: mb, color: i(0) as i16, alpha: 0x7F, dur: dur as i16 });
                b.wait_bg = b.wait_bg.max(dur * 2 + 5);
            }

            // Musique, bruitages, voix.
            BGM_PLAY => self.batch().bgm = Some((i(0), delay(i(1), None))),
            BGM_STOP => self.batch().bgm = Some((-1, delay(i(0), None))),
            SE_PLAY => {
                let b = self.batch();
                if b.se.len() < 0x32 {
                    b.se.push((i(0), delay(i(1), None)));
                }
            }
            SE_STOP => {
                let b = self.batch();
                if b.se.len() < 0x32 {
                    b.se.push((-1, delay(i(0), None)));
                }
            }
            LOOPSE_PLAY => {
                if let Ok(k) = usize::try_from(i(2))
                    && k < 2
                {
                    self.batch().loop_se[k] = Some((i(0), delay(i(1), None)));
                }
            }
            LOOPSE_STOP => {
                if let Ok(k) = usize::try_from(i(1))
                    && k < 2
                {
                    self.batch().loop_se[k] = Some((-1, delay(i(0), None)));
                }
            }
            VOICE_PLAY => {
                let b = self.batch();
                b.voice = Some((i(0) as u32, delay(i(1), b.flash())));
            }
            STRSE_PLAY => {
                let b = self.batch();
                b.stream = Some((i(0) as u32, delay(i(1), b.flash())));
            }
            STRSE_WAITCUT => self.batch().stream_cut = true,

            // Les personnages du lot.
            TRCHARA_UP_ALL | TRCHARA_DEL_ALL => {
                let name = text(a, 0);
                if let Some(c) = self.chara(&name) {
                    let ones = c.digit(1);
                    // Pas de « tout afficher » après « tout effacer », ni
                    // l'inverse.
                    if (code == TRCHARA_UP_ALL && !matches!(ones, 2 | 6)) || (code == TRCHARA_DEL_ALL && !matches!(ones, 1 | 5)) {
                        c.tally += if code == TRCHARA_UP_ALL { 1 } else { 2 };
                        c.name = name;
                    }
                }
            }
            EZCHARA_STATESET | EZCHARA_NEXTSTATE => {
                let name = text(a, 0);
                let fade = code == EZCHARA_NEXTSTATE;
                let (image, emotion) = (text(a, 1), text(a, 2));
                let rgb = |v: i32| if fade && !(0..=0xFF).contains(&v) { 0x7F } else { v as u8 };
                let (r, g, b) = (rgb(i(3)), rgb(i(4)), rgb(i(5)));
                let index = i(6) as i16;
                let dur = if fade { ticks(i(7)) } else { 0x1E };
                let wait = if fade { ticks(i(7)) } else { 0 };
                let Some(c) = self.chara(&name) else { return };
                if c.digit(2) >= 1 {
                    return;
                }
                c.tally += if fade { 0x14 } else { 10 };
                c.name = name;
                c.flags[3] = u8::from(fade);
                c.flags[4] = u8::from(fade);
                c.flags[6] = u8::from(fade);
                c.images = std::array::from_fn(|_| image.clone());
                c.emotions = std::array::from_fn(|_| emotion.clone());
                c.rgb = [[r, g, b]; PARTS];
                c.index = index;
                if c.digit(3) < 1 {
                    c.flags[1] = 0;
                    c.flags[2] = 1;
                    c.durs[0] = if dur < 1 { 2 } else { dur as i16 };
                    c.durs[1] = -1;
                }
                let b = self.batch();
                b.wait = b.wait.max(wait);
            }
            EZCHARA_APP | EZCHARA_DIS | EZCHARA_DEL | EZCHARA_MOVE => {
                let name = text(a, 0);
                let n = |x: i32| ticks(x);
                let Some(c) = self.chara(&name) else { return };
                if c.digit(3) >= 1 {
                    return;
                }
                c.name = name;
                let (dur, del) = match code {
                    EZCHARA_APP => {
                        // L'écran partagé en `cols` colonnes : le centre de
                        // la colonne `col`, d'où il part décalé.
                        let w = 0x280 / i(1).max(1);
                        let centre = (w >> 1) + w * i(2);
                        c.tally += 100;
                        c.states[0] = 3;
                        c.states[1] = 0;
                        c.flags[2] = 0;
                        c.values[1] = centre as i16;
                        c.values[0] = (centre + i(3) * 0x1E) as i16;
                        (i(4), i(5))
                    }
                    EZCHARA_DIS | EZCHARA_DEL => {
                        c.tally += if code == EZCHARA_DIS { 200 } else { 300 };
                        c.flags[0] = 0;
                        c.states[0] = 4;
                        c.states[1] = if code == EZCHARA_DIS { 1 } else { 2 };
                        c.values[0] = (i(1) * 0x1E) as i16;
                        c.flags[1] = 0;
                        c.flags[2] = 1;
                        (i(2), i(3))
                    }
                    _ => {
                        let w = 0x280 / i(1).max(1);
                        c.tally += 400;
                        c.flags[0] = 0;
                        c.flags[1] = 1;
                        c.flags[2] = 0;
                        c.values[0] = ((w >> 1) + w * i(2)) as i16;
                        (i(3), i(4))
                    }
                };
                c.durs[0] = if n(dur) < 1 { 2 } else { n(dur) as i16 };
                c.durs[1] = -1;
                c.delay = n(del) as i16;
                let total = n(del) + n(dur);
                let b = self.batch();
                match code {
                    EZCHARA_APP => b.wait_app = b.wait_app.max(total + 10),
                    EZCHARA_MOVE => b.wait = b.wait.max(total),
                    _ => b.wait_del = b.wait_del.max(total),
                }
            }

            // Les drapeaux.
            FLAG_MAKE | SPFLAG_MAKE => self.flags.make(a.text(0), code == SPFLAG_MAKE),
            FLAG_SET | SPFLAG_SET => self.flags.set(a.text(0), i(1) as i16, code == SPFLAG_SET),
            FLAG_CHANGE | SPFLAG_CHANGE => self.flags.add(a.text(0), i(1) as i16, code == SPFLAG_CHANGE),
            FLAG_NUMBER | SPFLAG_NUMBER => {
                let v = self.flags.get(a.text(0), code == SPFLAG_NUMBER);
                a.set(1, i32::from(v));
            }
            FLAG_CMP => {
                // Le plus grand de cinq drapeaux (le premier à égalité).
                let mut best = (-1, i32::MIN);
                for k in 0..5 {
                    let v = i32::from(self.flags.get(a.text(k + 1), false));
                    if v > best.1 {
                        best = (k as i32, v);
                    }
                }
                a.set(0, best.0);
            }
            RAND => {
                // STCM2_rand : dest = min + rand() % n (le hasard, posé par
                // le moteur avant chaque passage).
                a.set(0, i(2) + self.random.rem_euclid(i(1).max(1)));
            }
            CDMODE_CHECK => a.set(0, 1),
            BONUS_SCENARIO_OPENFLAG => self.bonus_open = text(a, 0),

            // La carte.
            MAP_POS_DATASET => {
                let k = i(0).max(0) as usize;
                if self.places.len() <= k {
                    self.places.resize(k + 1, Place::default());
                }
                self.places[k].entry = Entry { file: i(1), name: text(a, 2) };
                self.places[k].data = i(3);
            }
            MAP_POS_STATESET => {
                let k = i(0).max(0) as usize;
                if self.places.len() <= k {
                    self.places.resize(k + 1, Place::default());
                }
                self.places[k].state = i(1);
            }
            MAP_POS_COMMENTSET => {
                let k = i(0).max(0) as usize;
                if self.places.len() <= k {
                    self.places.resize(k + 1, Place::default());
                }
                self.places[k].comment = text(a, 1);
                self.places[k].comment_n = i(2);
            }
            MAP_MARK_POS_SET => self.marks.push((i(0), i(1))),

            // Ce que retient le jeu : images, personnages, album, vidéos.
            CHARA_GP_ENTRY => self.images.push((text(a, 0), i(1))),
            THUM_GP_ENTRY => self.images.push((text(a, 0), i(1))),
            CHARA_ENTRY => self.charas.push(text(a, 0)),
            BG_ENTRY => self.bgs.push((text(a, 0), i(1))),
            CHARA_PT_SET => self.patterns.push((text(a, 0), text(a, 1), text(a, 2), [i(3), i(4), i(5), i(6)], [i(7), i(8), i(9), i(10)])),
            CHARA_EMO_SET => self.emotions.push((text(a, 0), text(a, 1), text(a, 2), text(a, 3), [i(4), i(5), i(6), i(7)])),
            CGLIB_ENTRY | CGLIB_MOVIE_ENTRY => self.cg.push(CgEntry {
                page: i(0),
                movie: code == CGLIB_MOVIE_ENTRY,
                slot: i(1),
                name: text(a, 2),
                sheet: text(a, 3),
                rect: [i(4) as i16, i(5) as i16, i(6) as i16, i(7) as i16],
            }),
            MOVIE_ENTRY => self.movies.push((text(a, 0), text(a, 1))),
            CHARA_TXT_SET => self.chara_texts.push((i(0), i(1), i(2), text(a, 3))),
            COLOR_SET => {
                if let Ok(k) = usize::try_from(i(0))
                    && k < 0x28
                {
                    if self.colors.len() <= k {
                        self.colors.resize(k + 1, None);
                    }
                    self.colors[k] = Some([i(1) as u8, i(2) as u8, i(3) as u8]);
                }
            }

            // Mini-jeux, résultats, fins.
            MINIGAME_SELCHARA_GET => a.set(0, self.selchara),
            MINIGAME_RESULT_GET => a.set(0, self.won),
            MINIGAME_BONUS_CG_SET => self.bonus_cg.push((i(0), i(1), text(a, 2))),
            BONUS_CG_FLAG_ENTRY | BONUS_CG_SPFLAG_ENTRY | BONUS_CG_COMP_ENTRY if self.cg_bonus.is_none() => {
                let kind = (code - BONUS_CG_FLAG_ENTRY) as i32;
                let (flag, value) = if kind == 2 { (Vec::new(), 0) } else { (text(a, 1), i(2) as i16) };
                self.cg_bonus = Some((kind, text(a, 0), flag, value));
            }
            MINIGAME_CHALLENGE_OPENFLAG_SET => self.challenge_open = text(a, 0),
            MINIGAME_CHALLENGE_CLEARFLAG_SET => self.challenge_clear = text(a, 0),
            RESULT_DATA_SET => self.results.push((i(0), text(a, 1), [i(2), i(3), i(4)])),
            ENDING_CHECK_FLAG_SET => self.ending_flags.push((i(0), text(a, 1))),
            ENDING_CHECK_BG_SET => self.ending_bg = text(a, 0),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Uni;
    use crate::original::Iso;
    use crate::stcm2::{Script, run};

    /// Les vrais scripts (si l'ISO est à la racine du dépôt) : « init »
    /// donne le prologue, qui commence par se présenter.
    #[test]
    fn prologue() {
        let Some(iso) = Iso::for_tests() else { return };
        let uni = Uni::open(&iso, "UNION/SCRIPT.UNI").unwrap();
        let load = |n: u32| Script::parse(uni.read(&iso, n).unwrap()).unwrap();
        let mut world = World::default();
        let mut init = load(0);
        let at = init.entry(b"init").unwrap();
        run(&mut init, at, &mut world);
        assert_eq!(world.start, Entry { file: 101, name: b"sure1".to_vec() });
        assert_eq!(world.init_scripts.len(), 4);
        let mut flags = load(4);
        let at = flags.entry(b"sure1").unwrap();
        run(&mut flags, at, &mut world);
        assert!(world.flags.normal.iter().flatten().any(|f| f.name == b"dokuro"));
        let mut prologue = load(101);
        let at = prologue.entry(b"sure1").unwrap();
        run(&mut prologue, at, &mut world);
        let first = &world.batches[0];
        // « 僕の », en Shift-JIS.
        assert!(first.text.starts_with(&[0x96, 0x6C, 0x82, 0xCC]));
        assert_eq!(first.bgm.map(|b| b.0), Some(3));
        assert_eq!(first.bg.as_ref().map(|b| b.name.as_slice()), Some(&b"bg03_a"[..]));
        assert!(world.batches.len() > 20 && world.batches.len() <= MAX_BATCHES);
        assert_eq!(world.flags.get(b"switch", false), 1);
    }

    /// Le relevé des scènes suspectes (à lancer à la main :
    /// `cargo test audit -- --ignored --nocapture`). Il suit l'histoire sur
    /// les vrais scripts comme le jeu (la première réponse à chaque choix, le
    /// premier lieu choisissable de la carte, les mini-jeux gagnés), tient la
    /// caméra, le tremblement, les plans de fondu et les personnages comme
    /// la scène du portage, et signale ce qui dure : un zoom ou un décalage
    /// de la caméra, un tremblement, un plan posé, un personnage hors de
    /// l'écran. Chaque ligne donne le point d'entrée et le lot où cela
    /// commence (à reprendre avec DOKURO_STORY) ; ce sont des suspects, à
    /// vérifier dans le code du jeu.
    #[test]
    #[ignore]
    fn audit() {
        let Some(iso) = Iso::for_tests() else { return };
        let uni = Uni::open(&iso, "UNION/SCRIPT.UNI").unwrap();
        let play = |world: &mut World, e: &Entry| -> bool {
            let Some(mut s) = uni.read(&iso, e.file as u32).and_then(Script::parse) else { return false };
            let Some(at) = s.entry(&e.name) else { return false };
            run(&mut s, at, world);
            true
        };
        let mut world = World::default();
        play(&mut world, &Entry { file: 0, name: b"init".to_vec() });
        for e in world.init_scripts.clone() {
            play(&mut world, &e);
        }
        // DOKURO_AUDIT=<script>:<point d'entrée> : partir d'ailleurs (une
        // fin, par exemple 121:sure1).
        world.next = match std::env::var("DOKURO_AUDIT").ok().and_then(|v| v.split_once(':').map(|(f, e)| (f.parse().ok(), e.as_bytes().to_vec()))) {
            Some((Some(file), name)) => Entry { file, name },
            _ => world.start.clone(),
        };
        // L'état suivi, et depuis où (point d'entrée, lot) et combien de
        // pages dure chaque écart.
        let name = |e: &Entry, k: usize| format!("{}:{}#{k}", e.file, String::from_utf8_lossy(&e.name));
        let mut bg = Vec::new();
        let mut camera = (0, 0, 100);
        let mut shake = 0u8;
        let mut planes: [Option<(i16, u8)>; 7] = [None; 7];
        let mut actors: Vec<(Vec<u8>, i16)> = Vec::new();
        let mut since: std::collections::HashMap<String, (String, u32)> = Default::default();
        let mut found = Vec::new();
        let mut same = 0;
        let mut last = Entry::default();
        let mut end = "5000 passages";
        for _ in 0..5000 {
            let entry = world.next.clone();
            same = if entry == last { same + 1 } else { 0 };
            last = entry.clone();
            if entry.name.is_empty() {
                end = "plus de point d'entrée à suivre";
                break;
            }
            if same > 50 {
                end = "le même point d'entrée 50 fois";
                break;
            }
            world.batches.clear();
            world.menu = Default::default();
            if !play(&mut world, &entry) {
                end = "point d'entrée introuvable";
                break;
            }
            for (k, b) in world.batches.clone().iter().enumerate() {
                if b.kind == Kind::Movie {
                    // La musique du lot qui suit (la vidéo finie, le jeu relance
                    // la musique en cours ; un arrêt ici la coupe aussitôt).
                    let next = world.batches.get(k + 1).map(|n| (n.kind, n.bgm));
                    eprintln!("vidéo « {} » en {} ; lot suivant {:?}", String::from_utf8_lossy(&b.movie), name(&entry, k), next);
                }
                if let Some(g) = &b.bg
                    && g.name != bg
                {
                    bg = g.name.clone();
                    camera = (0, 0, 100);
                    shake = 0;
                }
                for t in b.targets[0].iter().filter(|t| t.dur >= 0) {
                    if t.zoom >= 0 {
                        camera = (i32::from(t.x), i32::from(t.y), i32::from(t.zoom));
                    }
                    if t.shake.0 != 0xFF {
                        shake = t.shake.0;
                    }
                }
                for (p, f) in b.fades.iter().enumerate() {
                    if let Some(f) = f {
                        planes[p] = (f.kind == 0).then_some((f.color, f.alpha));
                    }
                }
                for c in &b.charas {
                    let (ones, hundreds) = (c.tally % 10, c.tally / 100 % 10);
                    actors.retain(|a| !(a.0 == c.name && (ones == 2 || hundreds == 2 || hundreds == 3)));
                    let x = match hundreds {
                        1 => Some(c.values[1]),
                        4 => Some(c.values[0]),
                        _ => None,
                    };
                    if let Some(x) = x {
                        actors.retain(|a| a.0 != c.name);
                        actors.push((c.name.clone(), x));
                    }
                }
                match b.kind {
                    Kind::Menu | Kind::MenuNoWait => {
                        let flag = world.menu_flag.clone();
                        world.flags.set(&flag, 0, false);
                    }
                    Kind::Map => {
                        let script = world.map_script.clone();
                        play(&mut world, &script);
                        if let Some(p) = world.places.iter().find(|p| p.state == 2).cloned() {
                            world.next = p.entry;
                            world.flags.set(b"switch", p.data as i16, false);
                        }
                    }
                    Kind::Mogura | Kind::Run | Kind::Card | Kind::Catch | Kind::Bat => {
                        world.selchara = 0;
                        world.won = 1;
                    }
                    _ => {}
                }
                if b.kind != Kind::Text {
                    continue;
                }
                // Les écarts de cette page : leur compte continue, ou
                // commence ici ; ceux qui ont cessé sont oubliés.
                let mut now = Vec::new();
                if camera.2 != 100 {
                    now.push(format!("zoom {}", camera.2));
                }
                if (camera.0, camera.1) != (0, 0) {
                    now.push(format!("caméra en {}, {}", camera.0, camera.1));
                }
                if shake != 0 {
                    now.push(format!("tremblement {shake}"));
                }
                for (p, on) in planes.iter().enumerate() {
                    if let Some((color, alpha)) = on {
                        now.push(format!("plan {p} posé (couleur {color}, opacité {alpha:#x})"));
                    }
                }
                for a in actors.iter().filter(|a| !(0..=640).contains(&a.1)) {
                    now.push(format!("{} en x = {}", String::from_utf8_lossy(&a.0), a.1));
                }
                let here = name(&entry, k);
                since.retain(|what, (start, pages)| {
                    let goes_on = now.contains(what);
                    if !goes_on && *pages >= 10 {
                        found.push(format!("{what} : {pages} pages, de {start} à {here}"));
                    }
                    goes_on
                });
                for what in now {
                    since.entry(what).or_insert((here.clone(), 0)).1 += 1;
                }
            }
            if world.batches.iter().any(|b| b.kind == Kind::GameEnd) {
                end = "fin du jeu";
                break;
            }
        }
        for (what, (start, pages)) in since.iter().filter(|s| s.1.1 >= 10) {
            found.push(format!("{what} : {pages} pages, de {start} à la fin du parcours"));
        }
        for f in &found {
            println!("{f}");
        }
        println!("{} suspects (10 pages ou plus) ; parcours arrêté à {}:{} ({end})", found.len(), last.file, String::from_utf8_lossy(&last.name));
    }
}
