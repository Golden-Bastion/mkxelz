//! Colors and ramps. A ramp is a list of colors ordered dark -> light.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb(pub u8, pub u8, pub u8);

pub fn parse_hex(s: &str) -> Option<Rgb> {
    let t = s.trim().trim_start_matches('#');
    if t.len() != 6 || !t.is_ascii() {
        return None;
    }
    let r = u8::from_str_radix(&t[0..2], 16).ok()?;
    let g = u8::from_str_radix(&t[2..4], 16).ok()?;
    let b = u8::from_str_radix(&t[4..6], 16).ok()?;
    Some(Rgb(r, g, b))
}

#[derive(Clone, Debug)]
pub struct Ramp(pub Vec<Rgb>);

impl Ramp {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Nearest-step lookup: v = 0 is the darkest color, v = 1 the lightest.
    pub fn sample(&self, v: f32) -> Rgb {
        let n = self.0.len();
        if n == 0 {
            return Rgb(255, 0, 255);
        }
        if n == 1 {
            return self.0[0];
        }
        let t = v.clamp(0.0, 1.0);
        let i = (t * (n - 1) as f32).round() as usize;
        self.0[i.min(n - 1)]
    }
}

const RAMPS: &[(&str, [&str; 6])] = &[
    ("stone", ["#23232b", "#3a3a46", "#55556a", "#747488", "#9a9aae", "#c4c4d4"]),
    ("grass", ["#1d3a1f", "#2c5a2a", "#3f7d33", "#5ba342", "#86c85a", "#b5e07a"]),
    ("dirt", ["#2b1d14", "#44301f", "#63452c", "#84603a", "#a98050", "#cfa56e"]),
    ("sand", ["#6b5a38", "#8f7a4b", "#b89d62", "#d4ba7e", "#e8d6a0", "#f6eac4"]),
    ("water", ["#0e2a4a", "#14416b", "#1d5f94", "#2f85bd", "#5db0dd", "#a4dcef"]),
    ("wood", ["#2a1a10", "#422916", "#5e3b1f", "#7c5330", "#9c7044", "#be9262"]),
    ("brick", ["#2d1b1b", "#4a2a26", "#6b3b33", "#8c5143", "#ad6a56", "#cf8c74"]),
    ("snow", ["#6c7a99", "#8d9bb8", "#aebbd4", "#cdd8ea", "#e6edf7", "#ffffff"]),
    ("lava", ["#2a0a05", "#5c1608", "#a3260b", "#e0480f", "#ff8a1f", "#ffd04a"]),
    ("void", ["#07060d", "#0e0c1a", "#171428", "#221d3a", "#302952", "#433a70"]),
];

pub fn builtin_names() -> Vec<&'static str> {
    RAMPS.iter().map(|r| r.0).collect()
}

pub fn builtin_ramp(name: &str) -> Option<Ramp> {
    RAMPS.iter().find(|r| r.0 == name).map(|r| {
        Ramp(r.1.iter().filter_map(|h| parse_hex(h)).collect())
    })
}
