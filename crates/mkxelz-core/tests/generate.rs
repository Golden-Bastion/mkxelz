use mkxelz_core::{generate, presets, Spec};

fn load(name: &str) -> Spec {
    Spec::from_json(presets::get(name).expect("preset exists")).expect("preset parses")
}

#[test]
fn presets_generate() {
    for (name, _) in presets::PRESETS {
        let out = generate(&load(name)).expect("generates");
        assert!(!out.entries.is_empty());
        assert_eq!(out.atlas.rgba.len(), (out.atlas.width * out.atlas.height * 4) as usize);
    }
}

#[test]
fn same_seed_same_pixels() {
    let a = generate(&load("dungeon")).unwrap();
    let b = generate(&load("dungeon")).unwrap();
    assert_eq!(a.atlas.rgba, b.atlas.rgba);
}

#[test]
fn different_seed_different_pixels() {
    let a = generate(&load("dungeon")).unwrap();
    let mut spec = load("dungeon");
    spec.seed += 1;
    let b = generate(&spec).unwrap();
    assert_ne!(a.atlas.rgba, b.atlas.rgba);
}

#[test]
fn switches_change_output() {
    let mut on = load("dungeon");
    on.switches.insert("moss".into(), true);
    let mut off = load("dungeon");
    off.switches.insert("moss".into(), false);
    let a = generate(&on).unwrap();
    let b = generate(&off).unwrap();
    assert_ne!(a.atlas.rgba, b.atlas.rgba);
}

#[test]
fn blob47_emits_47_cells_per_variant() {
    let out = generate(&load("dungeon")).unwrap();
    let n = out.entries.iter().filter(|e| e.name == "dirt_patch").count();
    assert_eq!(n, 47);
}

#[test]
fn png_encodes() {
    let out = generate(&load("meadow")).unwrap();
    let bytes = out.atlas.encode_png().unwrap();
    assert_eq!(&bytes[1..4], b"PNG");
}
