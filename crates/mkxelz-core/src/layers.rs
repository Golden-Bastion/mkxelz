//! Layer passes. Every pass edits a float "value" buffer (0 = darkest ramp color,
//! 1 = lightest). Quantizing to the ramp happens last, so all detail stays on-palette.
//! All writes wrap around the cell edges, so tiles repeat seamlessly.

use crate::noise::{fbm, value_noise};
use crate::rng::{hash_combine, unit, Rng};
use crate::spec::{Layer, LayerKind, Region, ScatterKind, WallSpec};

pub struct Canvas {
    pub size: u32,
    pub wall: bool,
    pub cap_h: u32,
    pub val: Vec<f32>,
}

impl Canvas {
    pub fn new(size: u32, wall: bool, cap_h: u32) -> Self {
        Canvas {
            size,
            wall,
            cap_h,
            val: vec![0.5; (size * size) as usize],
        }
    }

    fn region_ok(&self, region: Region, y: u32) -> bool {
        match region {
            Region::All => true,
            Region::Cap => self.wall && y < self.cap_h,
            Region::Face => !self.wall || y >= self.cap_h,
        }
    }

    fn add(&mut self, region: Region, x: i32, y: i32, d: f32) {
        let s = self.size as i32;
        let xx = x.rem_euclid(s) as u32;
        let yy = y.rem_euclid(s) as u32;
        if !self.region_ok(region, yy) {
            return;
        }
        let i = (yy * self.size + xx) as usize;
        self.val[i] += d;
    }

    fn set(&mut self, region: Region, x: u32, y: u32, v: f32) {
        if !self.region_ok(region, y) {
            return;
        }
        let i = (y * self.size + x) as usize;
        self.val[i] = v;
    }
}

const DIRS: [(i32, i32); 8] = [
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];

pub fn apply(c: &mut Canvas, layer: &Layer, seed: u64) {
    let region = layer.region;
    let s = c.size;
    let sf = s as f32;
    let mut rng = Rng::new(seed);

    match &layer.kind {
        LayerKind::Fill(p) => {
            for y in 0..s {
                for x in 0..s {
                    c.set(region, x, y, p.level);
                }
            }
        }
        LayerKind::Noise(p) => {
            for y in 0..s {
                for x in 0..s {
                    let u = (x as f32 + 0.5) / sf;
                    let v = (y as f32 + 0.5) / sf;
                    let n = fbm(seed, u, v, p.cells, p.octaves.min(6));
                    c.add(
                        region,
                        x as i32,
                        y as i32,
                        (n - 0.5) * 2.0 * p.strength + p.bias,
                    );
                }
            }
        }
        LayerKind::Speckle(p) => {
            let count = ((s * s) as f32 * p.density) as u32;
            for _ in 0..count {
                let x = rng.below(s) as i32;
                let y = rng.below(s) as i32;
                let sign = if rng.chance(p.light) { 1.0 } else { -1.0 };
                let mag = 0.5 + 0.5 * rng.f32();
                c.add(region, x, y, sign * mag * p.strength);
            }
        }
        LayerKind::Cracks(p) => {
            for _ in 0..p.count {
                let x0 = rng.below(s) as i32;
                let y0 = rng.below(s) as i32;
                let d0 = rng.below(8) as usize;
                let mut stack: Vec<(i32, i32, usize, u32)> = vec![(x0, y0, d0, p.length)];
                while let Some((mut x, mut y, mut dir, mut rem)) = stack.pop() {
                    while rem > 0 {
                        c.add(region, x, y, -p.strength);
                        if rng.chance(0.4) {
                            dir = (dir + if rng.chance(0.5) { 1 } else { 7 }) % 8;
                        }
                        if p.branch > 0.0 && rem > 3 && rng.chance(p.branch * 0.3) {
                            let bd = (dir + if rng.chance(0.5) { 2 } else { 6 }) % 8;
                            stack.push((x, y, bd, rem / 2));
                        }
                        x += DIRS[dir].0;
                        y += DIRS[dir].1;
                        rem -= 1;
                    }
                }
            }
        }
        LayerKind::Bricks(p) => {
            let rows = p.rows.max(1);
            let cols = p.cols.max(1);
            let (y0, h) = if region == Region::Face && c.wall {
                (c.cap_h, s - c.cap_h)
            } else {
                (0, s)
            };
            let bh = h as f32 / rows as f32;
            let bw = sf / cols as f32;
            let m = p.mortar as f32;
            for y in y0..(y0 + h) {
                for x in 0..s {
                    let ly_abs = (y - y0) as f32;
                    let row = ((ly_abs / bh).floor() as u32).min(rows - 1);
                    let ly = ly_abs - row as f32 * bh;
                    let off = if p.offset && row % 2 == 1 {
                        bw * 0.5
                    } else {
                        0.0
                    };
                    let xf = (x as f32 + off).rem_euclid(sf);
                    let col = ((xf / bw).floor() as u32).min(cols - 1);
                    let lx = xf - col as f32 * bw;
                    let d = if lx < m || ly < m {
                        p.mortar_level
                    } else {
                        let hsh = hash_combine(hash_combine(seed, row as u64), col as u64);
                        (unit(hsh) - 0.5) * 2.0 * p.variation
                    };
                    c.add(region, x as i32, y as i32, d);
                }
            }
        }
        LayerKind::Grain(p) => {
            let a = p.cells.max(1) as i32;
            let b = (p.cells.max(1) * p.stretch.max(1)) as i32;
            let (px, py) = if p.vertical { (b, a) } else { (a, b) };
            for y in 0..s {
                for x in 0..s {
                    let u = (x as f32 + 0.5) / sf;
                    let v = (y as f32 + 0.5) / sf;
                    let n = value_noise(seed, u * px as f32, v * py as f32, px, py);
                    c.add(region, x as i32, y as i32, (n - 0.5) * 2.0 * p.strength);
                }
            }
        }
        LayerKind::Scatter(p) => {
            let sz = p.size.max(1) as i32;
            for _ in 0..p.count {
                let x = rng.below(s) as i32;
                let y = rng.below(s) as i32;
                match p.kind {
                    ScatterKind::Blades => {
                        let h = ((sz.max(2) as f32) * (0.6 + 0.4 * rng.f32()))
                            .round()
                            .max(2.0) as i32;
                        for i in 0..h {
                            let d = if i == 0 {
                                -0.5 * p.strength
                            } else {
                                p.strength * (0.7 + 0.3 * i as f32 / h as f32)
                            };
                            c.add(region, x, y - i, d);
                        }
                    }
                    ScatterKind::Pebbles => {
                        let w = sz.max(2);
                        let h = (sz * 2 / 3).max(1);
                        for dy in 0..h {
                            for dx in 0..w {
                                let corner =
                                    w > 2 && (dx == 0 || dx == w - 1) && (dy == 0 || dy == h - 1);
                                if !corner {
                                    c.add(region, x + dx, y + dy, 0.4 * p.strength);
                                }
                            }
                        }
                        c.add(region, x + 1, y, p.strength);
                        for dx in 1..=w {
                            c.add(region, x + dx, y + h, -0.8 * p.strength);
                        }
                    }
                    ScatterKind::Tufts => {
                        c.add(region, x, y, p.strength);
                        c.add(region, x - 1, y + 1, p.strength * 0.8);
                        c.add(region, x + 1, y + 1, p.strength * 0.8);
                    }
                }
            }
        }
        LayerKind::EdgeShade(p) => {
            let w = p.width.max(1).min(s);
            for y in 0..s {
                for x in 0..s {
                    let dark = x >= s - w || y >= s - w;
                    let light = x < w || y < w;
                    let d = if dark {
                        -p.amount
                    } else if light {
                        p.amount
                    } else {
                        continue;
                    };
                    c.add(region, x as i32, y as i32, d);
                }
            }
        }
    }
}

/// Three-quarter wall shading: lit cap on top, front face darkening toward the base.
pub fn apply_wall(c: &mut Canvas, w: &WallSpec) {
    let s = c.size;
    let cap = c.cap_h;
    for y in 0..s {
        for x in 0..s {
            let i = (y * s + x) as usize;
            if y < cap {
                c.val[i] += w.cap_lift;
            } else {
                let t = (y - cap) as f32 / (s - cap).max(1) as f32;
                c.val[i] -= w.face_shadow * t;
            }
        }
    }
    if w.lip && cap >= 1 && cap < s {
        for x in 0..s {
            c.val[((cap - 1) * s + x) as usize] += 0.12;
            c.val[(cap * s + x) as usize] -= 0.2;
        }
    }
}
