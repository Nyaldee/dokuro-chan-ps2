//! Le résultat de la journée (AD_Result*) : où en est Sakura avec chacune
//! des cinq filles. Six cases sur deux rangées : chacune son image selon le
//! niveau atteint, la plaque de son nom et une jauge ; la deuxième case n'a
//! qu'une image. Le niveau et la jauge viennent d'un drapeau de l'histoire et
//! de trois seuils (result_data_set : drapeau, maximum de la jauge, niveau
//! 1, niveau 2).

use crate::engine::{EffectDef, Effects, Key};
use crate::gfx::{Blend, Gfx, GREY};
use crate::story::World;

/// Les images (SYSTEM.UNI 50 à 55).
pub const C0: &str = "resu_c0";
pub const C1: &str = "resu_c1";
pub const C2: &str = "resu_c2";
pub const C3: &str = "resu_c3";
pub const PARTS: &str = "resu_parts";
pub const BACK: &str = "resu_back";

/// Le niveau (−1 : drapeau négatif ; 0, 1, 2) et la jauge (0 à 100) de
/// chaque fille (AD_ResultFlagChack).
pub fn levels(world: &World) -> [(i32, i32); 5] {
    std::array::from_fn(|k| {
        let Some((_, name, [max, mid, top])) = world.results.iter().rev().find(|r| r.0 == k as i32) else { return (0, 0) };
        let v = i32::from(world.flags.get(name, false));
        let level = if v >= *top {
            2
        } else if v >= *mid {
            1
        } else if v < 0 {
            -1
        } else {
            0
        };
        let gauge = if v < 1 {
            0
        } else if v < *max {
            v * 100 / max
        } else {
            100
        };
        (level, gauge)
    })
}

/// La place d'une case (0 à 5, trois par rangée) : 190 × 199, en x = 0x1c +
/// 0xc4 × colonne, y = 0x17 + 0xce × rangée.
pub fn cell_rect(cell: i32) -> [f32; 4] {
    [(cell % 3 * 0xC4 + 0x1C) as f32, (cell / 3 * 0xCE + 0x17) as f32, 190.0, 199.0]
}

/// AD_ResultTran : le fond, puis les six cases.
pub fn draw(gfx: &mut Gfx, levels: &[(i32, i32); 5]) {
    gfx.tran(BACK, [0.0, 0.0, 512.0, 448.0], [0.0, 0.0, 640.0, 448.0], 0xD2, 0x7F, Blend::Normal, GREY);
    for cell in 0..6 {
        let [x, y, ..] = cell_rect(cell as i32);
        // La deuxième case n'a pas de fille ; la première est la première
        // fille, les suivantes les autres.
        let (level, gauge) = match cell {
            1 => (0, 0),
            0 => levels[0],
            _ => levels[cell - 1],
        };
        column(gfx, x, y, cell as i32, level, gauge);
    }
}

/// AD_DistributeTran : la répartition des points. Le fond, le cadre du
/// curseur (resu_parts 0, 0x108, 190 × 199, en x = 0x1c + 0xc4 × colonne,
/// y = 0x16 + 0xce × rangée), et les six cases voilées, sans jauge (le
/// niveau −1). Les points donnés, un gris sombre (0x14) couvre l'écran.
pub fn draw_share(gfx: &mut Gfx, (col, row): (i32, i32), done: bool) {
    if done {
        gfx.paint([0.0, 0.0, 640.0, 448.0], 0xD3, 0x7F, [0x14; 3]);
        return;
    }
    gfx.tran(BACK, [0.0, 0.0, 512.0, 448.0], [0.0, 0.0, 640.0, 448.0], 0xD2, 0x7F, Blend::Normal, GREY);
    let (x, y) = ((col * 0xC4 + 0x1C) as f32, (row * 0xCE + 0x16) as f32);
    gfx.tran(PARTS, [0.0, 264.0, 190.0, 199.0], [x, y, 190.0, 199.0], 0xD4, 0x7F, Blend::Normal, GREY);
    for cell in 0..6 {
        let [x, y, ..] = cell_rect(cell);
        column(gfx, x, y, cell, -1, 0);
    }
}

/// AD_DistributeColumnEffMake : la case choisie clignote deux fois en blanc
/// (son image du niveau 0, prio 0xd4 : opacité 100 puis 0, deux fois, en
/// quatre fois 5 images).
pub fn flash(effects: &mut Effects, (col, row): (i32, i32)) {
    let cell = col + row * 3;
    let (texture, u, v) = match cell % 6 {
        0 => (C0, 0.0, 0.0),
        1 => (C3, 191.0, 200.0),
        2 => (C0, 191.0, 200.0),
        3 => (C1, 0.0, 200.0),
        4 => (C2, 191.0, 0.0),
        _ => (C3, 0.0, 0.0),
    };
    let mut d = EffectDef::ready();
    d.texture = Some(texture);
    d.src = [u, v, 190.0, 199.0];
    d.size = (190.0, 199.0);
    d.prio = 0xD4;
    d.pos = (col * 0xC4 + 0x1C + 0x5F, row * 0xCE + 0x17 + 99);
    d.alpha = 0;
    d.rgb = [0xFF; 3];
    for (k, alpha) in [100, 0, 100, 0].into_iter().enumerate() {
        d.keys[k] = Key { dur: 5, alpha, rgb: [200; 3], ..Key::NONE };
    }
    effects.make(d);
}

/// AD_ResultColumnTran : une case.
fn column(gfx: &mut Gfx, x: f32, y: f32, cell: i32, level: i32, gauge: i32) {
    // L'image de la case selon le niveau : la planche et le quart.
    let (texture, u, v) = match (cell % 6, level) {
        (0, 2) => (C0, 0, 200),
        (0, 1) => (C0, 0xBF, 0),
        (0, _) => (C0, 0, 0),
        (1, _) => (C3, 0xBF, 200),
        (2, 2) => (C1, 0xBF, 0),
        (2, 1) => (C1, 0, 0),
        (2, _) => (C0, 0xBF, 200),
        (3, 2) => (C2, 0, 0),
        (3, 1) => (C1, 0xBF, 200),
        (3, _) => (C1, 0, 200),
        (4, 2) => (C2, 0xBF, 200),
        (4, 1) => (C2, 0, 200),
        (4, _) => (C2, 0xBF, 0),
        (_, 2) => (C3, 0, 200),
        (_, 1) => (C3, 0xBF, 0),
        (_, _) => (C3, 0, 0),
    };
    gfx.tran(texture, [u as f32, v as f32, 190.0, 199.0], [x, y, 190.0, 199.0], 0xD3, 0x7F, Blend::Normal, GREY);
    if cell == 1 {
        return;
    }
    // Niveau 0 ou drapeau négatif : un voile par-dessus.
    match level {
        0 => gfx.tran(PARTS, [195.0, 264.0, 190.0, 199.0], [x, y, 190.0, 199.0], 0xD2, 0x7F, Blend::Normal, GREY),
        -1 => gfx.tran(PARTS, [195.0, 61.0, 190.0, 199.0], [x, y, 190.0, 199.0], 0xD2, 0x7F, Blend::Normal, GREY),
        _ => {}
    }
    // La plaque du nom (une par fille), la jauge et son remplissage.
    let n = if cell > 0 { cell - 1 } else { cell };
    gfx.tran(PARTS, [0.0, (n * 0x24 + 0x50) as f32, 123.0, 34.0], [x + 6.0, y + 135.0, 123.0, 34.0], 0xD5, 0x7F, Blend::Normal, GREY);
    if level >= 0 {
        gfx.tran(PARTS, [0.0, 0.0, 164.0, 31.0], [x + 10.0, y + 164.0, 164.0, 31.0], 0xD4, 0x7F, Blend::Normal, GREY);
        let w = (gauge * 0xA0 / 100) as f32;
        gfx.tran(PARTS, [2.0, 35.0, w, 27.0], [x + 12.0, y + 165.0, w, 27.0], 0xD4, 0x7F, Blend::Normal, GREY);
    }
}
