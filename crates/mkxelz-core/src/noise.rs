//! Seamlessly tileable value noise.

use crate::rng::{hash_combine, unit};

fn lattice(seed: u64, x: i32, y: i32) -> f32 {
    unit(hash_combine(
        hash_combine(seed, x as u32 as u64),
        y as u32 as u64,
    ))
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// Value noise in [0, 1] that repeats every `px` / `py` lattice cells.
pub fn value_noise(seed: u64, x: f32, y: f32, px: i32, py: i32) -> f32 {
    let px = px.max(1);
    let py = py.max(1);
    let x0 = x.floor();
    let y0 = y.floor();
    let fx = smooth(x - x0);
    let fy = smooth(y - y0);
    let xi = x0 as i32;
    let yi = y0 as i32;
    let xa = xi.rem_euclid(px);
    let xb = (xi + 1).rem_euclid(px);
    let ya = yi.rem_euclid(py);
    let yb = (yi + 1).rem_euclid(py);
    let a = lattice(seed, xa, ya);
    let b = lattice(seed, xb, ya);
    let c = lattice(seed, xa, yb);
    let d = lattice(seed, xb, yb);
    let top = a + (b - a) * fx;
    let bot = c + (d - c) * fx;
    top + (bot - top) * fy
}

/// Fractal noise in [0, 1], seamless over u, v in [0, 1).
/// `cells` = lattice cells across the unit square at the first octave.
pub fn fbm(seed: u64, u: f32, v: f32, cells: u32, octaves: u32) -> f32 {
    let mut sum = 0.0f32;
    let mut norm = 0.0f32;
    let mut amp = 0.5f32;
    let mut freq = cells.max(1) as i32;
    for o in 0..octaves.max(1) {
        let s = seed.wrapping_add(o as u64 * 0x0100_0193);
        sum += amp * value_noise(s, u * freq as f32, v * freq as f32, freq, freq);
        norm += amp;
        amp *= 0.5;
        freq *= 2;
    }
    sum / norm
}
