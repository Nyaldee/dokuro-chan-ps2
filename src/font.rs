//! La police du jeu (FW_*, MkFontPrint, RunFontSystem), lue dans
//! l'exécutable du disque : des glyphes de 27 × 27 au plus, quatre niveaux
//! par pixel (fond, contour, demi-teinte, plein), coloriés par une des seize
//! palettes du jeu. Les textes sont en Shift-JIS, « #n » passe à la ligne.
//!
//! Les glyphes affichés sont rangés, au fur et à mesure, dans une seule
//! image (l'atlas).

use std::collections::HashMap;

use crate::gfx::{Blend, Gfx};
use crate::original::Iso;

/// L'exécutable est chargé en 0x00100000, au décalage 0x80 du fichier.
const LOAD: usize = 0x0010_0000 - 0x80;
/// Les caractères simples qui ont un glyphe (0xff : aucun).
const SINGLE: usize = 0x001E_0900;
/// Les palettes : trois couleurs RGBA par palette (plein, contour,
/// demi-teinte).
const PALETTES: usize = 0x001E_0A00;
/// L'en-tête de la police.
const FONT: usize = 0x001E_0B00;
/// La taille d'une case de glyphe dans l'image, et d'un glyphe.
const CELL: usize = 32;
const SIZE: usize = 27;

/// Les phrases du jeu dont se sert le portage (adresses dans l'exécutable).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Message {
    ChallengeFailed = 0x003642A0,
    ChallengeCleared = 0x003642C0,
    Retire = 0x00364320,
    BackToRoom = 0x00364360,
    BackToTitle = 0x00364280,
    MapTitle = 0x003644B0,
    GameMenu = 0x00363FC0,
    BackToRoomAsk = 0x00365280,
    Yes = 0x00363FA0,
    No = 0x00363FB0,
}

pub const MESSAGES: [Message; 10] = [Message::ChallengeFailed, Message::ChallengeCleared, Message::Retire, Message::BackToRoom, Message::BackToTitle, Message::MapTitle, Message::GameMenu, Message::BackToRoomAsk, Message::Yes, Message::No];

/// Un glyphe placé : son code et son centre, depuis l'origine du texte.
#[derive(Clone, Copy)]
pub struct Glyph {
    pub code: u16,
    pub x: i32,
    pub y: i32,
    /// L'échelle (#Scale) et le compte de départ de son apparition (0 : le
    /// premier glyphe ; il gagne 1 par image et se montre à partir de −10).
    pub scale: f32,
    pub time: f32,
    /// La palette donnée par #Color (sinon, celle du texte).
    pub color: Option<u8>,
}

pub struct Font {
    /// Plages de codes : début, fin (exclue), premier glyphe.
    ranges: Vec<(u16, u16, u16)>,
    /// Glyphes : largeur, hauteur, octets, décalage de l'image.
    glyphs: Vec<(u8, u8, u16, u32)>,
    bitmaps: Vec<u8>,
    single: Vec<u8>,
    palettes: Vec<[[u8; 4]; 3]>,
    /// L'exécutable, pour ses phrases.
    exe: Vec<u8>,
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

impl Font {
    /// FW_Init et InitFontSystem : la police, les palettes et les phrases,
    /// depuis SLPM_661.86.
    pub fn load(iso: &Iso) -> Option<Font> {
        let (sector, size) = iso.file("SLPM_661.86")?;
        let exe = iso.read(u64::from(sector), 0, size as usize)?;
        let at = |addr: usize| addr - LOAD;
        let head = at(FONT);
        let count = usize::from(u16_at(&exe, head + 4)?);
        let range_table = head + u32_at(&exe, head + 0x14)? as usize;
        let glyph_table = head + u32_at(&exe, head + 0x18)? as usize;
        let bitmap_start = head + u32_at(&exe, head + 0x1C)? as usize;
        let ranges = (0..count)
            .map(|i| Some((u16_at(&exe, range_table + i * 8)?, u16_at(&exe, range_table + i * 8 + 2)?, u16_at(&exe, range_table + i * 8 + 4)?)))
            .collect::<Option<Vec<_>>>()?;
        let total = ranges.iter().map(|&(start, end, base)| usize::from(base) + usize::from(end - start)).max()?;
        let glyphs = (0..total)
            .map(|i| {
                let g = glyph_table + i * 8;
                Some((*exe.get(g)?, *exe.get(g + 1)?, u16_at(&exe, g + 2)?, u32_at(&exe, g + 4)?))
            })
            .collect::<Option<Vec<_>>>()?;
        let end = glyphs.iter().map(|&(_, _, bytes, offset)| offset as usize + usize::from(bytes)).max()?;
        let bitmaps = exe.get(bitmap_start..bitmap_start + end)?.to_vec();
        let single = exe.get(at(SINGLE)..at(SINGLE) + 256)?.to_vec();
        let palettes = (0..16)
            .map(|p| {
                let b = at(PALETTES) + p * 12;
                let c = |k: usize| exe[b + k * 4..b + k * 4 + 4].try_into().unwrap_or([0; 4]);
                [c(0), c(1), c(2)]
            })
            .collect();
        Some(Font { ranges, glyphs, bitmaps, single, palettes, exe })
    }

    pub fn message(&self, m: Message) -> &[u8] {
        self.string(m as usize)
    }

    /// Le mot de 32 bits de l'exécutable à cette adresse (une table).
    pub fn word(&self, addr: usize) -> u32 {
        addr.checked_sub(LOAD).and_then(|at| self.exe.get(at..at + 4)).map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// La phrase de l'exécutable à cette adresse, jusqu'au premier zéro.
    pub fn string(&self, addr: usize) -> &[u8] {
        let rest = addr.checked_sub(LOAD).and_then(|at| self.exe.get(at..)).unwrap_or(&[]);
        &rest[..rest.iter().position(|&b| b == 0).unwrap_or(rest.len())]
    }

    /// FW_CheckFont : le glyphe d'un code.
    fn glyph(&self, code: u16) -> Option<(u8, u8, u16, u32)> {
        let &(start, _, base) = self.ranges.iter().find(|&&(start, end, _)| start <= code && code < end)?;
        self.glyphs.get(usize::from(base + code - start)).copied()
    }

    /// MkFontPrint (taille 1, sans délai) : les glyphes d'un texte.
    pub fn layout(&self, text: &[u8]) -> Vec<Glyph> {
        self.print(text, 0.0)
    }

    /// MkFontPrint : les glyphes d'un texte, chacun avec le moment où il
    /// apparaît (`speed` images entre deux glyphes, multipliées par
    /// `#Speed[n]`). Les positions sont entières, comme dans le jeu : un
    /// caractère double se centre à 13,5 × échelle et avance de la largeur
    /// du dernier glyphe cherché moins 2 ; un simple se centre à
    /// 7,5 × échelle + 10 et avance de 15 ; « #n » revient au début de ligne
    /// et descend de 29 ; tout à l'échelle de `#Scale[x]`. `#Pos[x,y]` place
    /// la suite (et le début des lignes suivantes), `#Color[n]` change de
    /// palette. Les espaces avancent sans glyphe ; les autres commandes
    /// (`#Effect[…]`…) sont passées.
    pub fn print(&self, text: &[u8], speed: f32) -> Vec<Glyph> {
        let mut out = Vec::new();
        let (mut x, mut y, mut left) = (0i32, 0i32, 0i32);
        let (mut scale, mut step, mut time) = (1.0f32, speed, 0.0f32);
        let mut color = None;
        // La largeur du dernier glyphe cherché : celle d'un caractère double
        // avant le premier.
        let mut width = SIZE as f32;
        let mut i = 0;
        let number = |s: &[u8]| -> f32 {
            let end = s.iter().position(|&b| b == b']' || b == b',').unwrap_or(s.len());
            std::str::from_utf8(&s[..end]).ok().and_then(|v| v.trim().parse().ok()).unwrap_or(0.0)
        };
        while i < text.len() {
            let b = text[i];
            if b == b'#' {
                let rest = &text[i + 1..];
                if rest.first() == Some(&b'n') {
                    x = left;
                    y += (scale * 29.0) as i32;
                    i += 2;
                    continue;
                }
                if rest.starts_with(b"Speed[") {
                    step = speed * number(&rest[6..]);
                } else if rest.starts_with(b"Scale[") {
                    scale = number(&rest[6..]);
                } else if rest.starts_with(b"Color[") {
                    color = Some(number(&rest[6..]) as u8).filter(|&c| c <= 0xF).or(Some(0));
                } else if rest.starts_with(b"Pos[") {
                    let args = &rest[4..];
                    x = number(args) as i32;
                    left = x;
                    let comma = args.iter().position(|&c| c == b',').map_or(args.len(), |p| p + 1);
                    y = number(&args[comma..]) as i32;
                }
                i += 1 + rest.iter().position(|&c| c == b']').map_or(rest.len(), |p| p + 1);
                continue;
            }
            let next = text.get(i + 1).copied().unwrap_or(0);
            let double = matches!(b, 0x81..=0x9F | 0xE0..=0xFC) && matches!(next, 0x40..=0x7E | 0x80..=0xFC);
            let (code, wide) = if double { (u16::from(b) << 8 | u16::from(next), true) } else { (u16::from(b), false) };
            let shown = if wide { code != 0x8140 } else { b != b' ' && self.single[usize::from(b)] != 0xFF };
            if shown {
                width = self.glyph(code).map_or(SIZE as f32, |g| f32::from(g.0));
                let (gx, gy) = if wide {
                    ((scale * 13.5 + x as f32) as i32, (scale * 13.5 + y as f32) as i32)
                } else {
                    ((scale * 7.5 + x as f32 + 10.0) as i32, (scale * 13.5 + y as f32 + 1.0) as i32)
                };
                out.push(Glyph { code, x: gx, y: gy, scale, time, color });
                time -= step;
            }
            x += if wide { ((width - 2.0) * scale) as i32 } else { (15.0 * scale) as i32 };
            i += if wide { 2 } else { 1 };
        }
        out
    }
    /// AD_TextNumCountx et AD_TextNumCounty : la plus longue ligne en
    /// octets (un caractère double en compte deux) et le nombre de lignes.
    pub fn measure(text: &[u8]) -> (i32, i32) {
        let (mut widest, mut line, mut lines) = (0, 0, 1);
        let mut i = 0;
        while i < text.len() {
            if text[i] == b'#' {
                if text.get(i + 1) == Some(&b'n') {
                    widest = widest.max(line);
                    line = 0;
                    lines += 1;
                    i += 2;
                } else {
                    i += text[i..].iter().position(|&b| b == b']').map_or(text.len(), |p| p + 1);
                }
                continue;
            }
            let double = matches!(text[i], 0x81..=0x9F | 0xE0..=0xFC);
            line += if double { 2 } else { 1 };
            i += if double { 2 } else { 1 };
        }
        (widest.max(line), lines)
    }

    /// FW_MakeFontFrame : les niveaux (0 à 3) d'un glyphe, 27 × 27. Un code
    /// absent de la police donne le motif rayé du jeu.
    fn levels(&self, code: u16) -> [u8; SIZE * SIZE] {
        let mut out = [0; SIZE * SIZE];
        let Some((_, h, bytes, offset)) = self.glyph(code) else {
            for py in 0..16 {
                for px in (1..16).step_by(2) {
                    out[py * SIZE + px] = 3;
                }
            }
            return out;
        };
        let (rows, row_bytes) = if h == 0 { (1, 1) } else { (usize::from(h), usize::from(bytes) / usize::from(h)) };
        for row in 0..rows.min(SIZE) {
            for k in 0..row_bytes {
                let b = self.bitmaps.get(offset as usize + row * row_bytes + k).copied().unwrap_or(0);
                for (n, shift) in [6, 4, 2, 0].into_iter().enumerate() {
                    let px = k * 4 + n;
                    if px < SIZE {
                        out[row * SIZE + px] = b >> shift & 3;
                    }
                }
            }
        }
        out
    }

    /// La couleur prémultipliée d'un niveau dans une palette (alpha de la
    /// PS2 : 0x80 = opaque).
    fn color(&self, palette: u8, level: u8) -> [u8; 4] {
        let Some(p) = self.palettes.get(usize::from(palette)) else { return [0; 4] };
        let [r, g, b, a] = match level {
            3 => p[0],
            1 => p[1],
            2 => p[2],
            _ => return [0; 4],
        };
        let a = (u32::from(a) * 255 / 128).min(255);
        let pre = |c: u8| (u32::from(c) * a / 255) as u8;
        [pre(r), pre(g), pre(b), a as u8]
    }
}

/// Les glyphes des phrases, chacun dans sa palette, rangés seize par ligne
/// dans des cases de 32 (le glyphe en 2, 2, comme dans la mémoire du GS).
pub struct Atlas {
    cells: HashMap<(u16, u8), usize>,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Le nom de l'image des glyphes.
pub const TEXTURE: &str = "font";

/// De quoi mettre un texte en glyphes au fil du jeu : la police, l'atlas,
/// les dessins (où l'atlas est renvoyé s'il grandit).
pub struct Pen<'a> {
    pub font: &'a Font,
    pub atlas: &'a mut Atlas,
    pub gfx: &'a mut Gfx,
}

impl Pen<'_> {
    /// Un texte qui paraît d'un coup.
    pub fn glyphs(&mut self, text: &[u8]) -> Vec<Glyph> {
        typeset(self.font, self.atlas, self.gfx, text, 0.0)
    }

    /// Des phrases de l'exécutable, mises bout à bout.
    pub fn join(&self, addrs: &[usize]) -> Vec<u8> {
        addrs.iter().flat_map(|&a| self.font.string(a).iter().copied()).collect()
    }
}

/// Le `sprintf` du jeu, pour ses seuls « %d » et « %02d ».
pub fn sprintf(fmt: &[u8], ints: &[i32]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut ints = ints.iter();
    let mut i = 0;
    while i < fmt.len() {
        if fmt[i..].starts_with(b"%02d") {
            out.extend_from_slice(format!("{:02}", ints.next().copied().unwrap_or(0)).as_bytes());
            i += 4;
        } else if fmt[i..].starts_with(b"%d") {
            out.extend_from_slice(ints.next().copied().unwrap_or(0).to_string().as_bytes());
            i += 2;
        } else {
            out.push(fmt[i]);
            i += 1;
        }
    }
    out
}

/// Les glyphes d'un texte, rangés dans l'atlas dans leur palette (0 sans
/// #Color) et dans celle de l'ombre (8) ; l'atlas est renvoyé à la carte
/// graphique s'il a grandi.
pub fn typeset(font: &Font, atlas: &mut Atlas, gfx: &mut Gfx, text: &[u8], speed: f32) -> Vec<Glyph> {
    let glyphs = font.print(text, speed);
    let wanted: Vec<(u16, u8)> = glyphs.iter().flat_map(|g| [(g.code, g.color.unwrap_or(0)), (g.code, 8)]).collect();
    if atlas.add(font, &wanted) {
        gfx.load(TEXTURE, atlas.image());
    }
    glyphs
}

impl Default for Atlas {
    /// Un atlas vide (une rangée de cases).
    fn default() -> Atlas {
        Atlas { cells: HashMap::new(), width: (16 * CELL) as u32, height: CELL as u32, rgba: vec![0; 16 * CELL * CELL * 4] }
    }
}

impl Atlas {
    /// Ajoute les glyphes qui manquent (l'image s'allonge vers le bas, les
    /// cases déjà rangées ne bougent pas) ; vrai s'il y en avait.
    pub fn add(&mut self, font: &Font, wanted: &[(u16, u8)]) -> bool {
        let first = self.cells.len();
        for &key in wanted {
            let n = self.cells.len();
            self.cells.entry(key).or_insert(n);
        }
        if self.cells.len() == first {
            return false;
        }
        let width = self.width as usize;
        let height = self.cells.len().div_ceil(16).max(1) * CELL;
        self.rgba.resize(width * height * 4, 0);
        self.height = height as u32;
        for (&(code, palette), &n) in self.cells.iter().filter(|&(_, &n)| n >= first) {
            let levels = font.levels(code);
            let (cx, cy) = (n % 16 * CELL + 2, n / 16 * CELL + 2);
            for py in 0..SIZE {
                for px in 0..SIZE {
                    let at = ((cy + py) * width + cx + px) * 4;
                    self.rgba[at..at + 4].copy_from_slice(&font.color(palette, levels[py * SIZE + px]));
                }
            }
        }
        true
    }

    /// L'image de l'atlas, à envoyer à la carte graphique.
    pub fn image(&self) -> crate::data::Image {
        crate::data::Image { width: self.width, height: self.height, rgba: self.rgba.clone() }
    }

    /// Le glyphe dans l'image (27 × 27 depuis 2, 2 dans sa case).
    fn src(&self, code: u16, palette: u8) -> Option<[f32; 4]> {
        let n = *self.cells.get(&(code, palette))?;
        Some([(n % 16 * CELL + 2) as f32, (n / 16 * CELL + 2) as f32, SIZE as f32, SIZE as f32])
    }

    /// RunFontSystem : chaque glyphe sur 26 × 26 autour de son centre,
    /// depuis `origin` ; l'ombre (une autre palette) 3 plus bas à droite,
    /// sous le texte.
    /// Les glyphes à l'image `clock` de leur apparition (None : tous) :
    /// un glyphe se montre à partir de −10, son opacité monte jusqu'à 0 et
    /// il glisse de 10 pixels vers sa place pendant ce temps.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(&self, gfx: &mut Gfx, glyphs: &[Glyph], origin: (i32, i32), prio: i32, palette: u8, shadow: Option<u8>, clock: Option<f32>) {
        for g in glyphs {
            let t = clock.map_or(0.0, |c| g.time + c);
            if t < -10.0 {
                continue;
            }
            let alpha = if t < 0.0 { ((t + 10.0) * 127.0 / 10.0) as i32 } else { 0x7F };
            let slide = (0x7F - alpha) * 10 / 0x7F;
            let size = 26.0 * g.scale;
            let (x, y) = ((origin.0 + g.x + slide) as f32 - size / 2.0, (origin.1 + g.y) as f32 - size / 2.0);
            if let Some(src) = shadow.and_then(|s| self.src(g.code, s)) {
                gfx.tran(TEXTURE, src, [x + 3.0, y + 3.0, size, size], prio, alpha, Blend::Normal, [0x80; 3]);
            }
            if let Some(src) = self.src(g.code, g.color.unwrap_or(palette)) {
                gfx.tran(TEXTURE, src, [x, y, size, size], prio + 1, alpha, Blend::Normal, [0x80; 3]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn sprintf() {
        assert_eq!(super::sprintf(b"#Color[%d]a %d/%02d", &[6, 2026, 3]), b"#Color[6]a 2026/03".to_vec());
    }
}
