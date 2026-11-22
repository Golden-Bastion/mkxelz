//! A sample three-quarter-view map built from the generated atlas, for previewing.

use crate::autotile;
use crate::generate::Output;
use crate::image::Image;
use crate::noise::fbm;
use crate::rng::{hash_combine, Rng};
use crate::spec::{Autotile, Kind};

struct Group {
    kind: Kind,
    autotile: Autotile,
    ids: Vec<usize>,
}

fn mask_for(mode: Autotile, raw: u8) -> Option<u8> {
    match mode {
        Autotile::Off => None,
        Autotile::Blob47 => Some(autotile::normalize(raw)),
        Autotile::Edges16 => Some(autotile::edges16_from(raw)),
    }
}

fn pick_entry(out: &Output, ids: &[usize], mask: Option<u8>, hsh: u64) -> usize {
    let cands: Vec<usize> = ids
        .iter()
        .copied()
        .filter(|&i| out.entries[i].mask == mask)
        .collect();
    if cands.is_empty() {
        ids[0]
    } else {
        cands[(hsh % cands.len() as u64) as usize]
    }
}

fn neighbor_mask(grid: &[usize], w: usize, h: usize, cx: usize, cy: usize, g: usize) -> u8 {
    let mut m = 0u8;
    for &(bit, dx, dy) in autotile::OFFSETS.iter() {
        let nx = cx as i32 + dx;
        let ny = cy as i32 + dy;
        let outside = nx < 0 || ny < 0 || nx >= (w as i32) || ny >= (h as i32);
        // Outside the map counts as "same" so edges do not draw at the map border.
        let same = outside || grid[(ny as usize) * w + (nx as usize)] == g;
        if same {
            m |= bit;
        }
    }
    m
}

/// Floors fill the map, further floor tiles form noise patches, walls ring the map.
pub fn compose_preview(out: &Output, cols: u32, rows: u32, seed: u64) -> Image {
    let size = out.spec.tile_size;
    let cols = cols.max(3);
    let rows = rows.max(3);
    let mut img = Image::new(cols * size, rows * size);

    let mut groups: Vec<Group> = Vec::new();
    for t in &out.spec.tiles {
        let ids: Vec<usize> = out
            .entries
            .iter()
            .filter(|e| e.name == t.name)
            .map(|e| e.index as usize)
            .collect();
        if !ids.is_empty() {
            groups.push(Group {
                kind: t.kind,
                autotile: t.autotile,
                ids,
            });
        }
    }
    if groups.is_empty() {
        return img;
    }

    let floors: Vec<usize> = (0..groups.len())
        .filter(|&i| groups[i].kind == Kind::Floor)
        .collect();
    let walls: Vec<usize> = (0..groups.len())
        .filter(|&i| groups[i].kind == Kind::Wall)
        .collect();
    let base = floors.first().copied().unwrap_or(0);

    let w = cols as usize;
    let h = rows as usize;
    let mut grid = vec![base; w * h];

    for (k, &g) in floors.iter().enumerate().skip(1) {
        let s = hash_combine(seed, 100 + k as u64);
        for cy in 0..h {
            for cx in 0..w {
                let n = fbm(
                    s,
                    (cx as f32 + 0.5) / w as f32,
                    (cy as f32 + 0.5) / h as f32,
                    3,
                    2,
                );
                if n > 0.56 {
                    grid[cy * w + cx] = g;
                }
            }
        }
    }

    if let Some(&wg) = walls.first() {
        for cy in 0..h {
            for cx in 0..w {
                if cy == 0 || cy == h - 1 || cx == 0 || cx == w - 1 {
                    grid[cy * w + cx] = wg;
                }
            }
        }
        let mut rng = Rng::new(hash_combine(seed, 7));
        let blocks = (w * h) / 30;
        let span = (w as u32).saturating_sub(4).max(1);
        let span_y = (h as u32).saturating_sub(4).max(1);
        for _ in 0..blocks {
            let cx = 2 + rng.below(span) as usize;
            let cy = 2 + rng.below(span_y) as usize;
            if cx < w && cy < h {
                grid[cy * w + cx] = wg;
            }
        }
    }

    for cy in 0..h {
        for cx in 0..w {
            let g = grid[cy * w + cx];
            let hsh = hash_combine(hash_combine(seed, cx as u64), cy as u64 + 1000);
            let dx = (cx as u32) * size;
            let dy = (cy as u32) * size;

            // Underlay so cutout edges show the base floor.
            if g != base {
                let bg = &groups[base];
                let bm = match bg.autotile {
                    Autotile::Off => None,
                    _ => Some(255u8),
                };
                let idx = pick_entry(out, &bg.ids, bm, hsh);
                let (sx, sy) = out.cell_origin(idx as u32);
                img.blit_region(&out.atlas, sx, sy, size, size, dx, dy, true);
            }

            let grp = &groups[g];
            let raw = neighbor_mask(&grid, w, h, cx, cy, g);
            let mask = mask_for(grp.autotile, raw);
            let idx = pick_entry(out, &grp.ids, mask, hsh);
            let (sx, sy) = out.cell_origin(idx as u32);
            img.blit_region(&out.atlas, sx, sy, size, size, dx, dy, true);
        }
    }

    img
}
