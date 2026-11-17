//! Built-in example specs (embedded from /specs at build time).

pub const PRESETS: &[(&str, &str)] = &[
    ("dungeon", include_str!("../../../specs/dungeon.json")),
    ("meadow", include_str!("../../../specs/meadow.json")),
];

pub fn get(name: &str) -> Option<&'static str> {
    PRESETS.iter().find(|p| p.0 == name).map(|p| p.1)
}
