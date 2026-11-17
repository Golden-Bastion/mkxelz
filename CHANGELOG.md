# Changelog

All notable changes to this project are documented here.
Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versioning: [SemVer](https://semver.org/).

## [0.1.0] - 2026-11-17

First release.

### Added
- `mkxelz-core`: deterministic generator (JSON spec + switches + seed -> PNG atlas + `tiles.json`).
- Tile kinds for three-quarter view: `floor` and `wall` (lit top cap + darker front face).
- Layers: `fill`, `noise`, `speckle`, `cracks`, `bricks`, `grain`, `scatter`, `edge_shade`.
- Switches: layers and tiles can be gated with `"if"` / `"unless"`.
- Autotiling: `edges16` and `blob47`, with optional wobbly edge bands and transparent cutouts.
- Built-in ramps (palettes) plus inline custom ramps.
- `mkxelz-cli`: `gen`, `validate`, `presets`.
- `mkxelz-app`: egui GUI with live preview (Android primary target, desktop for development).
- Presets: `dungeon`, `meadow`.
