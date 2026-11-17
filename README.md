# MkXelz 0.1.0

Procedural (no AI) tile maker for three-quarter-view games. A JSON spec + named switches + a seed
produce a PNG atlas and a metadata JSON. Same version + same spec + same seed = identical pixels.

Standalone: nothing here depends on your game. You use the output files.

## Layout

```
crates/mkxelz-core   generator library (serde, serde_json, png only)
crates/mkxelz-cli    `mkxelz` command line tool
crates/mkxelz-app    egui GUI with live preview (Android first)
specs/                  example specs (also embedded as presets)
```

## Build and run

Desktop GUI (phone-shaped window, for development):

    cargo run -p mkxelz-app --bin mkxelz-desktop

CLI:

    cargo run -p mkxelz-cli -- gen preset:dungeon --out out
    cargo run -p mkxelz-cli -- gen my.json --seed 99 --set moss=true
    cargo run -p mkxelz-cli -- presets

Android (arm64):

    rustup target add aarch64-linux-android
    cargo install cargo-apk
    # needs ANDROID_HOME and ANDROID_NDK_ROOT set
    cd crates/mkxelz-app
    cargo apk run --lib --release

Check the cargo-apk README if flags differ in your version.

### Android notes

- Exports go to the app's own folder: `/sdcard/Android/data/dev.mkxelz.app/files/export/<name>/`.
  Android 11+ hides `Android/data` from most file managers. Use `adb pull`, Android Studio's Device
  Explorer, or a file manager that can open it.
- The GUI is built around sliders, toggles and dropdowns because the soft keyboard is unreliable in
  egui on NativeActivity. Typing (spec name, JSON text) may not work on-device. Edit JSON on a computer,
  put it in the `specs/` folder next to the exports, and load it from the Files tab.
- Planned: Android share sheet export (needs JNI), so you do not have to dig into `Android/data`.

## Output

`tiles.png`: one atlas, row-major, square cells of `tile_size` px, `padding` px apart (not at the outer edge).
`tiles.json`:

    {
      "mkxelz_version": "0.1.0",
      "tile_size": 32, "columns": 8, "rows": 7, "padding": 0, "image": "tiles.png",
      "neighbor_bits": { "N":1, "NE":2, "E":4, "SE":8, "S":16, "SW":32, "W":64, "NW":128 },
      "tiles": [ { "index": 0, "name": "stone_floor", "kind": "floor", "variant": 0, "tags": ["walkable"] },
                 { "index": 8, "name": "dirt_patch", "kind": "floor", "variant": 0, "mask": 255, "tags": [...] } ]
    }

Use `columns`, `rows`, `tile_size`, `padding` to build your atlas layout. For plain tiles, pick any entry
with the same `name` (variants are for visual variety). For autotiled tiles, compute a mask from the 8
neighbors (bit set = neighbor is the same terrain), then look up the entry with that `mask`:
`blob47` masks must be normalized first (a corner bit counts only if both adjacent edge bits are set);
`edges16` ignores corners. `preview.png` shows a sample map.

## Spec reference (spec_version 1)

Top level: `name`, `seed`, `tile_size` (8..128), `columns` (0 = auto), `padding`, `switches` (name -> bool), `tiles`.

Tile: `name` (unique), `kind` (`floor` | `wall`), `variants` (1..16), `ramp`, `edge_ramp`, `dither` (0..1),
`autotile` (`none` | `edges16` | `blob47`), `edge` {`width`, `wobble`, `shade`, `cells`, `cutout`},
`wall` {`face`, `cap_lift`, `face_shadow`, `lip`}, `if`, `unless`, `tags`, `layers`.

`ramp` is a built-in name (stone, grass, dirt, sand, water, wood, brick, snow, lava, void) or a list of
`"#rrggbb"` colors, dark to light.

Every layer has `type`, optional `if` / `unless` (switch names; unknown switches count as off) and
`region` (`all` | `cap` | `face`). Layers run in order on a value buffer that is quantized to the ramp at the end.

| type | parameters |
|---|---|
| fill | level |
| noise | cells, octaves, strength, bias |
| speckle | density, strength, light |
| cracks | count, length, strength, branch |
| bricks | rows, cols, mortar, offset, mortar_level, variation |
| grain | vertical, cells, stretch, strength |
| scatter | kind (blades/pebbles/tufts), count, strength, size |
| edge_shade | width, amount |

Everything wraps at the cell edges, so tiles repeat seamlessly.

## Versioning

SemVer. The `mkxelz_version` is written into every export.
- MAJOR: breaking change to the spec format or to `tiles.json`.
- MINOR: new layers/features. Pre-1.0, a change that alters pixels for existing specs also bumps MINOR.
- PATCH: fixes that do not change pixel output.
See CHANGELOG.md.

## Not in 0.1.0

Character sprites and animations, props/objects, Android share-sheet export, in-app JSON editing that
works without a keyboard.
