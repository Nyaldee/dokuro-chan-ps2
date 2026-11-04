//! La machine des scripts STCM2 (STCM2_Run, GetFunctionVariable) : un fil
//! qui part d'un point d'entrée et s'exécute d'un bloc jusqu'à son retour,
//! les instructions du jeu étant confiées à un `Host`. Voir docs/scripts.md.

/// Le nombre d'actions qu'un fil peut faire avant d'être arrêté (une
/// boucle sans fin dans un script ne bloque pas le jeu).
const MAX_STEPS: usize = 1_000_000;
/// La profondeur des appels (la pile du fil, piVar10[0xaa]).
const MAX_CALLS: usize = 8;

fn u32_at(b: &[u8], at: usize) -> u32 {
    b.get(at..at + 4).map_or(0, |s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// Un script : ses octets (ses variables y changent en place) et ses
/// points d'entrée.
pub struct Script {
    data: Vec<u8>,
    exports: Vec<(Vec<u8>, usize)>,
}

impl Script {
    pub fn parse(data: Vec<u8>) -> Option<Script> {
        if !data.starts_with(b"STCM2") {
            return None;
        }
        let (at, count) = (u32_at(&data, 0x20) as usize, u32_at(&data, 0x24) as usize);
        let exports = (0..count)
            .map(|k| {
                let e = at + 0x28 * k;
                let name = data.get(e + 4..e + 0x24).unwrap_or_default();
                let len = name.iter().position(|&b| b == 0).unwrap_or(name.len());
                (name[..len].to_vec(), u32_at(&data, e + 0x24) as usize)
            })
            .collect();
        Some(Script { data, exports })
    }

    /// STCM2_GetExportPointer : l'adresse d'un point d'entrée.
    pub fn entry(&self, name: &[u8]) -> Option<usize> {
        self.exports.iter().find(|(n, _)| n == name).map(|&(_, at)| at)
    }
}

/// Où est la valeur d'un paramètre.
#[derive(Clone, Copy, Debug)]
pub enum Loc {
    /// Une donnée du fichier : la place de sa valeur, flottante ou non.
    Data { at: usize, float: bool },
    /// Une valeur écrite dans l'action (0xff000000 + n).
    Imm(i32),
    /// Un registre du fil (0 à 3 entiers, 4 à 7 flottants).
    Reg(usize),
    /// Une adresse du fichier (une cible de saut).
    Addr(usize),
}

/// Les paramètres d'une instruction, que lit et écrit le `Host`.
pub struct Args<'a> {
    data: &'a mut [u8],
    regs: &'a mut [i32; 8],
    locs: &'a [Loc],
}

impl Args<'_> {
    /// La valeur entière du paramètre k (un flottant est arrondi vers 0).
    pub fn int(&self, k: usize) -> i32 {
        match self.locs.get(k) {
            Some(&Loc::Data { at, float }) => {
                let raw = u32_at(self.data, at);
                if float { f32::from_bits(raw) as i32 } else { raw as i32 }
            }
            Some(&Loc::Imm(n)) => n,
            Some(&Loc::Reg(r)) => self.regs[r],
            Some(&Loc::Addr(a)) => a as i32,
            None => 0,
        }
    }

    /// Le texte du paramètre k : les octets de sa donnée jusqu'au premier
    /// zéro (le Shift-JIS du jeu).
    pub fn text(&self, k: usize) -> &[u8] {
        match self.locs.get(k) {
            Some(&Loc::Data { at, .. }) => {
                let rest = self.data.get(at..).unwrap_or_default();
                &rest[..rest.iter().position(|&b| b == 0).unwrap_or(rest.len())]
            }
            _ => &[],
        }
    }

    /// Écrit un entier dans le paramètre k (une donnée ou un registre).
    pub fn set(&mut self, k: usize, v: i32) {
        match self.locs.get(k) {
            Some(&Loc::Data { at, float }) => {
                let raw = if float { (v as f32).to_bits() } else { v as u32 };
                if let Some(s) = self.data.get_mut(at..at + 4) {
                    s.copy_from_slice(&raw.to_le_bytes());
                }
            }
            Some(&Loc::Reg(r)) => self.regs[r] = v,
            _ => {}
        }
    }
}

/// Ce que fait le jeu des instructions qui ne sont pas de la machine.
pub trait Host {
    /// Une instruction du jeu (son numéro dans la table 0x35bf60).
    fn op(&mut self, op: u32, args: &mut Args);
}

/// Un appel en cours : où revenir, et les paramètres de l'appel (les
/// arguments du sous-programme).
struct Frame {
    back: usize,
    args: Vec<Loc>,
}

/// Les numéros de la machine dont se servent les scripts.
const RETURN: u32 = 0;
const IF_INT: u32 = 6;
const GOTO: u32 = 7;
const IF_FLOAT: u32 = 0x0D;

/// Exécute un fil depuis `start` jusqu'à son retour (STCM2_Run).
pub fn run(script: &mut Script, start: usize, host: &mut impl Host) {
    let mut regs = [0i32; 8];
    let mut calls: Vec<Frame> = Vec::new();
    let mut pc = start;
    for _ in 0..MAX_STEPS {
        let data = &script.data;
        let (call, op, count, length) = (u32_at(data, pc), u32_at(data, pc + 4), u32_at(data, pc + 8) as usize, u32_at(data, pc + 12) as usize);
        if length == 0 || pc + length > data.len() {
            return;
        }
        let args = calls.last().map_or(&[][..], |f| f.args.as_slice());
        let locs: Vec<Loc> = (0..count).map(|k| resolve(data, pc + 16 + 12 * k, args)).collect();
        let next = pc + length;
        if call == 1 {
            if calls.len() >= MAX_CALLS {
                return;
            }
            calls.push(Frame { back: next, args: locs });
            pc = op as usize;
            continue;
        }
        let mut a = Args { data: &mut script.data, regs: &mut regs, locs: &locs };
        pc = match op {
            RETURN => match calls.pop() {
                Some(f) => f.back,
                None => return,
            },
            GOTO => target(&locs, 0).unwrap_or(next),
            IF_INT | IF_FLOAT => {
                let (x, y) = (a.int(0), a.int(2));
                let holds = match a.int(1) {
                    0 => x > y,
                    1 => x < y,
                    2 => x >= y,
                    3 => x <= y,
                    4 => x == y,
                    5 => x != y,
                    _ => true,
                };
                if holds { next } else { target(&locs, 3).unwrap_or(next) }
            }
            0xFFFF_FF00..=0xFFFF_FF14 => {
                calc(op, &mut a);
                next
            }
            _ => {
                host.op(op, &mut a);
                next
            }
        };
    }
}

fn target(locs: &[Loc], k: usize) -> Option<usize> {
    match locs.get(k) {
        Some(&Loc::Addr(a)) => Some(a),
        _ => None,
    }
}

/// Les calculs (0xffffff00 à 0xffffff14), en entiers : les scripts n'en
/// font que sur des entiers.
fn calc(op: u32, a: &mut Args) {
    let three = op <= 0xFFFF_FF09;
    let (x, y) = if three { (a.int(1), a.int(2)) } else { (a.int(0), a.int(1)) };
    let v = match op & 0xFF {
        0x00 | 0x0A => x.wrapping_add(y),
        0x01 | 0x0B => x.wrapping_sub(y),
        0x02 | 0x0C => x.wrapping_mul(y),
        0x03 | 0x0D => x.checked_div(y).unwrap_or(0),
        0x04 | 0x0E => x.checked_rem(y).unwrap_or(0),
        0x05 | 0x0F => x.wrapping_shl(y as u32 & 31),
        0x06 | 0x10 => x.wrapping_shr(y as u32 & 31),
        0x07 | 0x11 => x & y,
        0x08 | 0x12 => x ^ y,
        0x09 | 0x13 => x | y,
        _ => y,
    };
    a.set(0, v);
}

/// GetFunctionVariable : où est la valeur d'un paramètre (trois mots).
fn resolve(data: &[u8], at: usize, args: &[Loc]) -> Loc {
    let (v0, v1, v2) = (u32_at(data, at), u32_at(data, at + 4), u32_at(data, at + 8));
    match v0 {
        0xFFFF_FF40 | 0xFFFF_FF41 => Loc::Addr(v1 as usize),
        0xFFFF_FF20..=0xFFFF_FF27 => Loc::Reg((v0 - 0xFFFF_FF20) as usize),
        0xFFFF_FF00..=0xFFFF_FF1F => args.get((v0 - 0xFFFF_FF00) as usize).copied().unwrap_or(Loc::Imm(0)),
        0xFF00_0000..=0xFFFF_FEFF => Loc::Imm((v0 - 0xFF00_0000) as i32),
        _ => {
            // Une donnée, et deux indices (une valeur ou une donnée qui la
            // donne) : la place du mot voulu dans son tableau.
            let index = |w: u32| -> usize {
                if w >= 0xFF00_0000 {
                    if w < 0xFFFF_FF00 { (w - 0xFF00_0000) as usize } else { 0 }
                } else {
                    u32_at(data, w as usize + 0x10) as usize
                }
            };
            let o = v0 as usize;
            let width = u32_at(data, o + 4) as usize;
            Loc::Data { at: o + 0x10 + 4 * index(v1) + 4 * index(v2) * width, float: u32_at(data, o) == 1 }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un petit script : une variable, un calcul, une condition, un appel,
    /// une instruction du jeu.
    #[test]
    fn machine() {
        let mut d = vec![0u8; 0x20];
        d[..5].copy_from_slice(b"STCM2");
        let word = |d: &mut Vec<u8>, w: u32| d.extend_from_slice(&w.to_le_bytes());
        // La variable : un entier, 0.
        let var = d.len() as u32;
        for w in [0, 1, 1, 4, 0] {
            word(&mut d, w);
        }
        let imm = |n: u32| [0xFF00_0000 + n, 0xFF00_0000, 0xFF00_0000];
        let data = |at: u32| [at, 0xFF00_0000, 0xFF00_0000];
        let action = |d: &mut Vec<u8>, call: u32, op: u32, params: &[[u32; 3]]| {
            let at = d.len();
            for w in [call, op, params.len() as u32, 16 + 12 * params.len() as u32] {
                word(d, w);
            }
            for p in params {
                for &w in p {
                    word(d, w);
                }
            }
            at
        };
        let start = d.len() as u32;
        // var += 3 ; appel ; si var == 4, op 0x118(var) ; fin.
        action(&mut d, 0, 0xFFFF_FF0A, &[data(var), imm(3)]);
        let call = action(&mut d, 1, 0, &[data(var)]);
        let test = action(&mut d, 0, IF_INT, &[data(var), imm(4), imm(4), [0xFFFF_FF41, 0, 0]]);
        action(&mut d, 0, 0x118, &[data(var)]);
        let end = action(&mut d, 0, RETURN, &[]);
        // Le sous-programme : arg0 += 1 ; retour.
        let sub = action(&mut d, 0, 0xFFFF_FF0A, &[[0xFFFF_FF00, 0, 0], imm(1)]);
        action(&mut d, 0, RETURN, &[]);
        d[call + 4..call + 8].copy_from_slice(&(sub as u32).to_le_bytes());
        d[test + 16 + 36 + 4..test + 16 + 36 + 8].copy_from_slice(&(end as u32).to_le_bytes());
        struct Seen(Vec<i32>);
        impl Host for Seen {
            fn op(&mut self, op: u32, args: &mut Args) {
                assert_eq!(op, 0x118);
                self.0.push(args.int(0));
            }
        }
        let mut script = Script { data: d, exports: Vec::new() };
        let mut seen = Seen(Vec::new());
        run(&mut script, start as usize, &mut seen);
        assert_eq!(seen.0, [4]);
    }
}
