//! The JSON spec model. Everything here is (de)serializable.

use crate::color::{builtin_ramp, parse_hex, Ramp};
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

/// Version of the JSON schema (not of the crate).
pub const SPEC_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Spec {
    pub spec_version: u32,
    pub name: String,
    pub seed: u64,
    /// Cell size in pixels (8..=128). Walls use the same cell size.
    pub tile_size: u32,
    /// Atlas columns; 0 = auto (up to 8).
    pub columns: u32,
    /// Transparent gap between atlas cells, in pixels.
    pub padding: u32,
    /// Named on/off switches. Layers and tiles reference them via "if" / "unless".
    pub switches: BTreeMap<String, bool>,
    pub tiles: Vec<TileSpec>,
}

impl Default for Spec {
    fn default() -> Self {
        Spec {
            spec_version: SPEC_VERSION,
            name: "untitled".to_string(),
            seed: 1,
            tile_size: 32,
            columns: 0,
            padding: 0,
            switches: BTreeMap::new(),
            tiles: Vec::new(),
        }
    }
}

impl Spec {
    pub fn from_json(text: &str) -> Result<Spec> {
        Ok(serde_json::from_str(text)?)
    }

    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Unknown switches count as off.
    pub fn switch(&self, name: &str) -> bool {
        self.switches.get(name).copied().unwrap_or(false)
    }

    pub fn gate(&self, when: &Option<String>, unless: &Option<String>) -> bool {
        let a = match when {
            Some(n) => self.switch(n),
            None => true,
        };
        let b = match unless {
            Some(n) => !self.switch(n),
            None => true,
        };
        a && b
    }

    pub fn validate(&self) -> Result<()> {
        if !(8..=128).contains(&self.tile_size) {
            return Err(Error::Spec("tile_size must be 8..=128".into()));
        }
        let mut seen = HashSet::new();
        let mut total = 0u32;
        for t in &self.tiles {
            if t.name.trim().is_empty() {
                return Err(Error::Spec("tile with empty name".into()));
            }
            if !seen.insert(t.name.clone()) {
                return Err(Error::Spec(format!("duplicate tile name '{}'", t.name)));
            }
            if !(1..=16).contains(&t.variants) {
                return Err(Error::Spec(format!("tile '{}': variants must be 1..=16", t.name)));
            }
            t.ramp.resolve()?;
            if let Some(r) = &t.edge_ramp {
                r.resolve()?;
            }
            if !(0.1..=0.9).contains(&t.wall.face) {
                return Err(Error::Spec(format!("tile '{}': wall.face must be 0.1..=0.9", t.name)));
            }
            if self.gate(&t.when, &t.unless) {
                total += t.variants * t.autotile.mask_count();
            }
        }
        if total == 0 {
            return Err(Error::Spec("no tiles enabled".into()));
        }
        if total > 2048 {
            return Err(Error::Spec(format!("too many cells ({total}); limit is 2048")));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// Flat ground tile.
    #[default]
    Floor,
    /// Three-quarter view wall: a lit top cap above a darker front face.
    Wall,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Autotile {
    #[default]
    #[serde(rename = "none")]
    Off,
    Edges16,
    Blob47,
}

impl Autotile {
    pub fn mask_count(&self) -> u32 {
        match self {
            Autotile::Off => 1,
            Autotile::Edges16 => 16,
            Autotile::Blob47 => 47,
        }
    }
}

/// Either a built-in ramp name ("stone") or an inline list of "#rrggbb" colors, dark -> light.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RampRef {
    Named(String),
    Colors(Vec<String>),
}

impl Default for RampRef {
    fn default() -> Self {
        RampRef::Named("stone".to_string())
    }
}

impl RampRef {
    pub fn resolve(&self) -> Result<Ramp> {
        match self {
            RampRef::Named(n) => {
                builtin_ramp(n).ok_or_else(|| Error::Spec(format!("unknown ramp '{n}'")))
            }
            RampRef::Colors(list) => {
                if list.len() < 2 {
                    return Err(Error::Spec("a custom ramp needs at least 2 colors".into()));
                }
                let mut v = Vec::with_capacity(list.len());
                for c in list {
                    match parse_hex(c) {
                        Some(rgb) => v.push(rgb),
                        None => return Err(Error::Spec(format!("bad color '{c}'"))),
                    }
                }
                Ok(Ramp(v))
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct EdgeSpec {
    /// Edge band width in pixels.
    pub width: f32,
    /// 0..1, how wavy the band is.
    pub wobble: f32,
    /// Added to the value inside the band when there is no edge_ramp.
    pub shade: f32,
    /// Lattice cells for the wobble noise (must tile, so integer).
    pub cells: u32,
    /// Make the band transparent so the tile can overlay another terrain.
    pub cutout: bool,
}

impl Default for EdgeSpec {
    fn default() -> Self {
        EdgeSpec { width: 3.0, wobble: 0.5, shade: -0.2, cells: 4, cutout: false }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct WallSpec {
    /// Fraction of the cell height taken by the front face (rest is the top cap).
    pub face: f32,
    pub cap_lift: f32,
    pub face_shadow: f32,
    /// 1px highlight under the cap edge and a dark line below it.
    pub lip: bool,
}

impl Default for WallSpec {
    fn default() -> Self {
        WallSpec { face: 0.6, cap_lift: 0.2, face_shadow: 0.25, lip: true }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct TileSpec {
    pub name: String,
    pub kind: Kind,
    pub variants: u32,
    pub ramp: RampRef,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub edge_ramp: Option<RampRef>,
    /// 0..1 ordered dithering when quantizing to the ramp.
    pub dither: f32,
    pub autotile: Autotile,
    pub edge: EdgeSpec,
    pub wall: WallSpec,
    #[serde(rename = "if", skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unless: Option<String>,
    pub tags: Vec<String>,
    pub layers: Vec<Layer>,
}

impl Default for TileSpec {
    fn default() -> Self {
        TileSpec {
            name: "tile".to_string(),
            kind: Kind::Floor,
            variants: 1,
            ramp: RampRef::default(),
            edge_ramp: None,
            dither: 0.0,
            autotile: Autotile::Off,
            edge: EdgeSpec::default(),
            wall: WallSpec::default(),
            when: None,
            unless: None,
            tags: Vec::new(),
            layers: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Region {
    #[default]
    All,
    /// Wall top cap only (nothing on floors).
    Cap,
    /// Wall front face only (everything on floors).
    Face,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Layer {
    #[serde(rename = "if", default, skip_serializing_if = "Option::is_none")]
    pub when: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unless: Option<String>,
    #[serde(default)]
    pub region: Region,
    #[serde(flatten)]
    pub kind: LayerKind,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LayerKind {
    Fill(FillParams),
    Noise(NoiseParams),
    Speckle(SpeckleParams),
    Cracks(CracksParams),
    Bricks(BricksParams),
    Grain(GrainParams),
    Scatter(ScatterParams),
    EdgeShade(EdgeShadeParams),
}

impl LayerKind {
    pub fn label(&self) -> &'static str {
        match self {
            LayerKind::Fill(_) => "fill",
            LayerKind::Noise(_) => "noise",
            LayerKind::Speckle(_) => "speckle",
            LayerKind::Cracks(_) => "cracks",
            LayerKind::Bricks(_) => "bricks",
            LayerKind::Grain(_) => "grain",
            LayerKind::Scatter(_) => "scatter",
            LayerKind::EdgeShade(_) => "edge_shade",
        }
    }

    /// One default instance of every layer type (for "add layer" menus).
    pub fn catalog() -> Vec<LayerKind> {
        vec![
            LayerKind::Fill(FillParams::default()),
            LayerKind::Noise(NoiseParams::default()),
            LayerKind::Speckle(SpeckleParams::default()),
            LayerKind::Cracks(CracksParams::default()),
            LayerKind::Bricks(BricksParams::default()),
            LayerKind::Grain(GrainParams::default()),
            LayerKind::Scatter(ScatterParams::default()),
            LayerKind::EdgeShade(EdgeShadeParams::default()),
        ]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct FillParams {
    pub level: f32,
}
impl Default for FillParams {
    fn default() -> Self {
        FillParams { level: 0.5 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct NoiseParams {
    pub cells: u32,
    pub octaves: u32,
    pub strength: f32,
    pub bias: f32,
}
impl Default for NoiseParams {
    fn default() -> Self {
        NoiseParams { cells: 4, octaves: 3, strength: 0.3, bias: 0.0 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SpeckleParams {
    pub density: f32,
    pub strength: f32,
    /// Fraction of specks that are lighter (the rest are darker).
    pub light: f32,
}
impl Default for SpeckleParams {
    fn default() -> Self {
        SpeckleParams { density: 0.05, strength: 0.2, light: 0.5 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct CracksParams {
    pub count: u32,
    pub length: u32,
    pub strength: f32,
    pub branch: f32,
}
impl Default for CracksParams {
    fn default() -> Self {
        CracksParams { count: 3, length: 10, strength: 0.3, branch: 0.15 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct BricksParams {
    pub rows: u32,
    pub cols: u32,
    pub mortar: u32,
    pub offset: bool,
    pub mortar_level: f32,
    pub variation: f32,
}
impl Default for BricksParams {
    fn default() -> Self {
        BricksParams { rows: 4, cols: 2, mortar: 1, offset: true, mortar_level: -0.3, variation: 0.12 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GrainParams {
    pub vertical: bool,
    pub cells: u32,
    pub stretch: u32,
    pub strength: f32,
}
impl Default for GrainParams {
    fn default() -> Self {
        GrainParams { vertical: false, cells: 3, stretch: 6, strength: 0.2 }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScatterKind {
    #[default]
    Blades,
    Pebbles,
    Tufts,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ScatterParams {
    pub kind: ScatterKind,
    pub count: u32,
    pub strength: f32,
    pub size: u32,
}
impl Default for ScatterParams {
    fn default() -> Self {
        ScatterParams { kind: ScatterKind::Blades, count: 12, strength: 0.3, size: 3 }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct EdgeShadeParams {
    pub width: u32,
    pub amount: f32,
}
impl Default for EdgeShadeParams {
    fn default() -> Self {
        EdgeShadeParams { width: 1, amount: 0.2 }
    }
}
