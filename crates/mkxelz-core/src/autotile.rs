//! Neighbor-mask autotiling (blob47 and edges16).
//!
//! A mask bit is SET when that neighbor is the SAME terrain ("connected").
//! The bit layout is exported in tiles.json so the game side can look tiles up.

pub const N: u8 = 1;
pub const NE: u8 = 2;
pub const E: u8 = 4;
pub const SE: u8 = 8;
pub const S: u8 = 16;
pub const SW: u8 = 32;
pub const W: u8 = 64;
pub const NW: u8 = 128;

/// (bit, dx, dy). Screen coordinates: +y is down.
pub const OFFSETS: [(u8, i32, i32); 8] = [
    (N, 0, -1),
    (NE, 1, -1),
    (E, 1, 0),
    (SE, 1, 1),
    (S, 0, 1),
    (SW, -1, 1),
    (W, -1, 0),
    (NW, -1, -1),
];

pub const BIT_NAMES: [(&str, u8); 8] = [
    ("N", N),
    ("NE", NE),
    ("E", E),
    ("SE", SE),
    ("S", S),
    ("SW", SW),
    ("W", W),
    ("NW", NW),
];

/// A corner only matters when both adjacent edges are connected.
pub fn normalize(mask: u8) -> u8 {
    let mut m = mask;
    if m & N == 0 || m & E == 0 {
        m &= !NE;
    }
    if m & E == 0 || m & S == 0 {
        m &= !SE;
    }
    if m & S == 0 || m & W == 0 {
        m &= !SW;
    }
    if m & W == 0 || m & N == 0 {
        m &= !NW;
    }
    m
}

/// Keep only the four edge bits; corners are treated as connected when both edges are.
pub fn edges16_from(raw: u8) -> u8 {
    let mut m = raw & (N | E | S | W);
    if m & N != 0 && m & E != 0 {
        m |= NE;
    }
    if m & E != 0 && m & S != 0 {
        m |= SE;
    }
    if m & S != 0 && m & W != 0 {
        m |= SW;
    }
    if m & W != 0 && m & N != 0 {
        m |= NW;
    }
    m
}

pub fn blob47_masks() -> Vec<u8> {
    let mut v: Vec<u8> = (0u16..256).map(|m| normalize(m as u8)).collect();
    v.sort_unstable();
    v.dedup();
    v
}

pub fn edges16_masks() -> Vec<u8> {
    (0u8..16)
        .map(|i| {
            let mut raw = 0u8;
            if i & 1 != 0 {
                raw |= N;
            }
            if i & 2 != 0 {
                raw |= E;
            }
            if i & 4 != 0 {
                raw |= S;
            }
            if i & 8 != 0 {
                raw |= W;
            }
            edges16_from(raw)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blob47_has_47_masks() {
        assert_eq!(blob47_masks().len(), 47);
    }

    #[test]
    fn edges16_has_16_unique_masks() {
        let mut v = edges16_masks();
        v.sort_unstable();
        v.dedup();
        assert_eq!(v.len(), 16);
    }

    #[test]
    fn full_mask_is_in_both_sets() {
        assert!(blob47_masks().contains(&255));
        assert!(edges16_masks().contains(&255));
    }
}
