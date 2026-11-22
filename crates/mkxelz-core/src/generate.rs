//! Spec -> atlas image + entries.

use crate::autotile::{self, BIT_NAMES, E, N, NE, NW, S, SE, SW, W};
use crate::color::Ramp;
use crate::error::Result;
use crate::image::Image;
use crate::layers::{self, Canvas};
use crate::noise::value_noise;
use crate::rng::{hash_combine, hash_str};
use crate::spec::{Autotile, Kind, Spec, TileSpec};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize)]
pub struct Entry {
    /// Cell index in the atlas (row-major).
    pub index: u32,
    pub name: String,
    pub kind: Kind,
    pub variant: u32,
    /// Neighbor mask for autotiled tiles (see `neighbor_bits` in tiles.json).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask: Option<u8>,
    pub tags: Vec<String>,
}

pub struct Output {
    /// The effective spec (switches/seed as used).
    pub spec: Spec,
    pub atlas: Image,
    pub entries: Vec<Entry>,
    pub columns: u32,
    pub rows: u32,
}

#[derive(Serialize)]
struct Meta<'a> {
    mkxelz_version: &'a str,
    spec_version: u32,
    name: &'a str,
    seed: u64,
    tile_size: u32,
    columns: u32,
    rows: u32,
    padding: u32,
    image: &'a str,
    neighbor_bits: BTreeMap<&'static str, u8>,
    tiles: &'a [Entry],
}

impl Output {
    /// Pixel origin of an atlas cell.
    pub fn cell_origin(&self, index: u32) -> (u32, u32) {
        let step = self.spec.tile_size + self.spec.padding;
        ((index % self.columns) * step, (index / self.columns) * step)
    }

    pub fn meta_json(&self, image_file: &str) -> Result<String> {
        let mut bits = BTreeMap::new();
        for (name, bit) in BIT_NAMES.iter() {
            bits.insert(*name, *bit);
        }
        let meta = Meta {
            mkxelz_version: crate::VERSION,
            spec_version: self.spec.spec_version,
            name: &self.spec.name,
            seed: self.spec.seed,
            tile_size: self.spec.tile_size,
            columns: self.columns,
            rows: self.rows,
            padding: self.spec.padding,
            image: image_file,
            neighbor_bits: bits,
            tiles: &self.entries,
        };
        Ok(serde_json::to_string_pretty(&meta)?)
    }

    /// Writes tiles.png, tiles.json, spec.json and preview.png into `dir`.
    pub fn export(&self, dir: &Path) -> Result<Vec<PathBuf>> {
        std::fs::create_dir_all(dir)?;
        let mut written = Vec::new();

        let png_path = dir.join("tiles.png");
        self.atlas.save_png(&png_path)?;
        written.push(png_path);

        let json_path = dir.join("tiles.json");
        std::fs::write(&json_path, self.meta_json("tiles.png")?)?;
        written.push(json_path);

        let spec_path = dir.join("spec.json");
        std::fs::write(&spec_path, self.spec.to_json()?)?;
        written.push(spec_path);

        let preview = crate::preview::compose_preview(self, 16, 10, self.spec.seed);
        let preview_path = dir.join("preview.png");
        preview.save_png(&preview_path)?;
        written.push(preview_path);

        Ok(written)
    }
}

const BAYER4: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

pub fn generate(spec: &Spec) -> Result<Output> {
    spec.validate()?;
    let size = spec.tile_size;

    let mut cells: Vec<(Entry, Vec<u8>)> = Vec::new();

    for tile in &spec.tiles {
        if !spec.gate(&tile.when, &tile.unless) {
            continue;
        }
        let ramp = tile.ramp.resolve()?;
        let edge_ramp = match &tile.edge_ramp {
            Some(r) => Some(r.resolve()?),
            None => None,
        };

        let masks: Vec<Option<u8>> = match tile.autotile {
            Autotile::Off => vec![None],
            Autotile::Edges16 => autotile::edges16_masks().into_iter().map(Some).collect(),
            Autotile::Blob47 => autotile::blob47_masks().into_iter().map(Some).collect(),
        };

        for variant in 0..tile.variants {
            let tile_seed = hash_combine(
                hash_combine(spec.seed, hash_str(&tile.name)),
                variant as u64,
            );
            let base = base_values(spec, tile, tile_seed);
            for mask in &masks {
                let rgba = render_cell(
                    size,
                    &base,
                    &ramp,
                    edge_ramp.as_ref(),
                    tile,
                    *mask,
                    tile_seed,
                );
                let entry = Entry {
                    index: cells.len() as u32,
                    name: tile.name.clone(),
                    kind: tile.kind,
                    variant,
                    mask: *mask,
                    tags: tile.tags.clone(),
                };
                cells.push((entry, rgba));
            }
        }
    }

    let n = cells.len() as u32;
    let columns = if spec.columns > 0 {
        spec.columns
    } else {
        n.clamp(1, 8)
    };
    let rows = n.div_ceil(columns);
    let pad = spec.padding;
    let step = size + pad;
    let mut atlas = Image::new(columns * step - pad, rows * step - pad);

    let mut entries = Vec::with_capacity(cells.len());
    for (entry, rgba) in cells {
        let cell = Image {
            width: size,
            height: size,
            rgba,
        };
        let cx = (entry.index % columns) * step;
        let cy = (entry.index / columns) * step;
        atlas.blit_region(&cell, 0, 0, size, size, cx, cy, false);
        entries.push(entry);
    }

    Ok(Output {
        spec: spec.clone(),
        atlas,
        entries,
        columns,
        rows,
    })
}

fn wall_cap_height(size: u32, tile: &TileSpec) -> u32 {
    let cap = ((size as f32) * (1.0 - tile.wall.face)).round() as u32;
    cap.clamp(2, size.saturating_sub(2).max(2))
}

fn base_values(spec: &Spec, tile: &TileSpec, tile_seed: u64) -> Vec<f32> {
    let size = spec.tile_size;
    let wall = tile.kind == Kind::Wall;
    let cap_h = if wall { wall_cap_height(size, tile) } else { 0 };
    let mut canvas = Canvas::new(size, wall, cap_h);
    for (i, layer) in tile.layers.iter().enumerate() {
        if !spec.gate(&layer.when, &layer.unless) {
            continue;
        }
        layers::apply(&mut canvas, layer, hash_combine(tile_seed, i as u64 + 1));
    }
    if wall {
        layers::apply_wall(&mut canvas, &tile.wall);
    }
    canvas.val
}

/// Distance from the pixel to the nearest "open" edge or inner corner, or f32::MAX if none.
fn edge_distance(mask: u8, x: u32, y: u32, size: u32) -> f32 {
    let s = size as f32;
    let fx = x as f32 + 0.5;
    let fy = y as f32 + 0.5;
    let mut d = f32::MAX;
    if mask & N == 0 {
        d = d.min(fy);
    }
    if mask & E == 0 {
        d = d.min(s - fx);
    }
    if mask & S == 0 {
        d = d.min(s - fy);
    }
    if mask & W == 0 {
        d = d.min(fx);
    }
    if mask & N != 0 && mask & E != 0 && mask & NE == 0 {
        d = d.min(((s - fx) * (s - fx) + fy * fy).sqrt());
    }
    if mask & S != 0 && mask & E != 0 && mask & SE == 0 {
        d = d.min(((s - fx) * (s - fx) + (s - fy) * (s - fy)).sqrt());
    }
    if mask & S != 0 && mask & W != 0 && mask & SW == 0 {
        d = d.min((fx * fx + (s - fy) * (s - fy)).sqrt());
    }
    if mask & N != 0 && mask & W != 0 && mask & NW == 0 {
        d = d.min((fx * fx + fy * fy).sqrt());
    }
    d
}

fn render_cell(
    size: u32,
    val: &[f32],
    ramp: &Ramp,
    edge_ramp: Option<&Ramp>,
    tile: &TileSpec,
    mask: Option<u8>,
    seed: u64,
) -> Vec<u8> {
    // Wobble depends on one axis at a time and wraps, so bands continue across neighbors.
    let wob_cells = tile.edge.cells.max(1);
    let mut wob = Vec::with_capacity(size as usize);
    for i in 0..size {
        let u = (i as f32 + 0.5) / size as f32;
        wob.push(value_noise(
            seed ^ 0xED6E,
            u * wob_cells as f32,
            0.37,
            wob_cells as i32,
            1,
        ));
    }

    let mut out = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let mut v = val[(y * size + x) as usize];
            let mut in_edge = false;
            if let Some(m) = mask {
                let d = edge_distance(m, x, y, size);
                if d < f32::MAX {
                    let w = tile.edge.width
                        * (1.0 + tile.edge.wobble * (wob[x as usize] + wob[y as usize] - 1.0));
                    if d < w.max(0.0) {
                        in_edge = true;
                    }
                }
            }
            let used: &Ramp = if in_edge {
                edge_ramp.unwrap_or(ramp)
            } else {
                ramp
            };
            if in_edge && edge_ramp.is_none() {
                v += tile.edge.shade;
            }
            let n = used.len();
            if tile.dither > 0.0 && n > 1 {
                let b = BAYER4[(y % 4) as usize][(x % 4) as usize] as f32 / 16.0;
                v += (b - 0.5) * tile.dither / (n - 1) as f32;
            }
            let c = used.sample(v);
            let a = if in_edge && tile.edge.cutout { 0 } else { 255 };
            out.push(c.0);
            out.push(c.1);
            out.push(c.2);
            out.push(a);
        }
    }
    out
}
