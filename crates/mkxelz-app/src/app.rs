use eframe::egui;
use mkxelz_core::color::builtin_names;
use mkxelz_core::preview::compose_preview;
use mkxelz_core::spec::{Autotile, Kind, Layer, LayerKind, RampRef, Region, ScatterKind};
use mkxelz_core::{generate, presets, Image as CoreImage, Output, Spec};
use std::path::PathBuf;

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Preview,
    Controls,
    Layers,
    Files,
}

enum LayerAction {
    Remove,
    Up,
    Down,
}

pub struct MkXelzApp {
    data_dir: PathBuf,
    spec: Spec,
    tab: Tab,
    output: Option<Output>,
    atlas_tex: Option<egui::TextureHandle>,
    preview_tex: Option<egui::TextureHandle>,
    preview_cols: u32,
    preview_rows: u32,
    dirty: bool,
    auto: bool,
    regen_requested: bool,
    status: String,
    spec_files: Vec<PathBuf>,
    json_text: String,
    selected_tile: usize,
}

fn color_image(img: &CoreImage) -> egui::ColorImage {
    egui::ColorImage::from_rgba_unmultiplied([img.width as usize, img.height as usize], &img.rgba)
}

fn show_tex(ui: &mut egui::Ui, tex: &egui::TextureHandle) {
    let size = tex.size_vec2();
    let avail = ui.available_width();
    // Whole-number scaling keeps pixel art crisp; shrink only if it cannot fit.
    let scale = if size.x > avail {
        avail / size.x
    } else {
        (avail / size.x).floor().max(1.0)
    };
    let sized = egui::load::SizedTexture::new(tex.id(), size);
    ui.add(egui::Image::new(sized).fit_to_exact_size(size * scale));
}

fn slider_f32(
    ui: &mut egui::Ui,
    v: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    label: &str,
) -> bool {
    ui.add(egui::Slider::new(v, range).text(label)).changed()
}

fn slider_u32(
    ui: &mut egui::Ui,
    v: &mut u32,
    range: std::ops::RangeInclusive<u32>,
    label: &str,
) -> bool {
    ui.add(egui::Slider::new(v, range).text(label)).changed()
}

fn switch_picker(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut Option<String>,
    names: &[String],
) -> bool {
    let mut changed = false;
    let current = value.clone().unwrap_or_else(|| "-".to_string());
    egui::ComboBox::from_label(label)
        .selected_text(current)
        .show_ui(ui, |ui| {
            if ui.selectable_label(value.is_none(), "-").clicked() {
                *value = None;
                changed = true;
            }
            for n in names {
                let selected = value.as_deref() == Some(n.as_str());
                if ui.selectable_label(selected, n.as_str()).clicked() {
                    *value = Some(n.clone());
                    changed = true;
                }
            }
        });
    changed
}

fn region_picker(ui: &mut egui::Ui, region: &mut Region) -> bool {
    let before = *region;
    egui::ComboBox::from_label("region")
        .selected_text(format!("{:?}", *region))
        .show_ui(ui, |ui| {
            ui.selectable_value(region, Region::All, "All");
            ui.selectable_value(region, Region::Cap, "Cap");
            ui.selectable_value(region, Region::Face, "Face");
        });
    before != *region
}

fn ramp_picker(ui: &mut egui::Ui, label: &str, ramp: &mut RampRef) -> bool {
    let mut changed = false;
    let current = match ramp {
        RampRef::Named(n) => n.clone(),
        RampRef::Colors(_) => "custom".to_string(),
    };
    egui::ComboBox::from_label(label)
        .selected_text(current.clone())
        .show_ui(ui, |ui| {
            for n in builtin_names() {
                if ui.selectable_label(current == n, n).clicked() {
                    *ramp = RampRef::Named(n.to_string());
                    changed = true;
                }
            }
        });
    changed
}

fn opt_ramp_picker(ui: &mut egui::Ui, label: &str, ramp: &mut Option<RampRef>) -> bool {
    let mut changed = false;
    let current = match ramp {
        Some(RampRef::Named(n)) => n.clone(),
        Some(RampRef::Colors(_)) => "custom".to_string(),
        None => "-".to_string(),
    };
    egui::ComboBox::from_label(label)
        .selected_text(current.clone())
        .show_ui(ui, |ui| {
            if ui.selectable_label(ramp.is_none(), "-").clicked() {
                *ramp = None;
                changed = true;
            }
            for n in builtin_names() {
                if ui.selectable_label(current == n, n).clicked() {
                    *ramp = Some(RampRef::Named(n.to_string()));
                    changed = true;
                }
            }
        });
    changed
}

fn autotile_picker(ui: &mut egui::Ui, mode: &mut Autotile) -> bool {
    let before = *mode;
    egui::ComboBox::from_label("autotile")
        .selected_text(format!("{:?}", *mode))
        .show_ui(ui, |ui| {
            ui.selectable_value(mode, Autotile::Off, "Off");
            ui.selectable_value(mode, Autotile::Edges16, "Edges16");
            ui.selectable_value(mode, Autotile::Blob47, "Blob47");
        });
    before != *mode
}

fn edit_layer(ui: &mut egui::Ui, layer: &mut Layer, switches: &[String]) -> bool {
    let mut ch = false;
    ch |= switch_picker(ui, "if", &mut layer.when, switches);
    ch |= switch_picker(ui, "unless", &mut layer.unless, switches);
    ch |= region_picker(ui, &mut layer.region);
    match &mut layer.kind {
        LayerKind::Fill(p) => {
            ch |= slider_f32(ui, &mut p.level, 0.0..=1.0, "level");
        }
        LayerKind::Noise(p) => {
            ch |= slider_u32(ui, &mut p.cells, 1..=16, "cells");
            ch |= slider_u32(ui, &mut p.octaves, 1..=6, "octaves");
            ch |= slider_f32(ui, &mut p.strength, 0.0..=1.0, "strength");
            ch |= slider_f32(ui, &mut p.bias, -0.5..=0.5, "bias");
        }
        LayerKind::Speckle(p) => {
            ch |= slider_f32(ui, &mut p.density, 0.0..=0.5, "density");
            ch |= slider_f32(ui, &mut p.strength, 0.0..=1.0, "strength");
            ch |= slider_f32(ui, &mut p.light, 0.0..=1.0, "light share");
        }
        LayerKind::Cracks(p) => {
            ch |= slider_u32(ui, &mut p.count, 0..=12, "count");
            ch |= slider_u32(ui, &mut p.length, 2..=40, "length");
            ch |= slider_f32(ui, &mut p.strength, 0.0..=1.0, "strength");
            ch |= slider_f32(ui, &mut p.branch, 0.0..=1.0, "branch");
        }
        LayerKind::Bricks(p) => {
            ch |= slider_u32(ui, &mut p.rows, 1..=16, "rows");
            ch |= slider_u32(ui, &mut p.cols, 1..=8, "cols");
            ch |= slider_u32(ui, &mut p.mortar, 0..=3, "mortar");
            ch |= ui.checkbox(&mut p.offset, "offset rows").changed();
            ch |= slider_f32(ui, &mut p.mortar_level, -1.0..=0.5, "mortar level");
            ch |= slider_f32(ui, &mut p.variation, 0.0..=0.5, "variation");
        }
        LayerKind::Grain(p) => {
            ch |= ui.checkbox(&mut p.vertical, "vertical").changed();
            ch |= slider_u32(ui, &mut p.cells, 1..=8, "cells");
            ch |= slider_u32(ui, &mut p.stretch, 1..=16, "stretch");
            ch |= slider_f32(ui, &mut p.strength, 0.0..=1.0, "strength");
        }
        LayerKind::Scatter(p) => {
            let before = p.kind;
            egui::ComboBox::from_label("kind")
                .selected_text(format!("{:?}", p.kind))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut p.kind, ScatterKind::Blades, "Blades");
                    ui.selectable_value(&mut p.kind, ScatterKind::Pebbles, "Pebbles");
                    ui.selectable_value(&mut p.kind, ScatterKind::Tufts, "Tufts");
                });
            ch |= before != p.kind;
            ch |= slider_u32(ui, &mut p.count, 0..=64, "count");
            ch |= slider_u32(ui, &mut p.size, 1..=8, "size");
            ch |= slider_f32(ui, &mut p.strength, 0.0..=1.0, "strength");
        }
        LayerKind::EdgeShade(p) => {
            ch |= slider_u32(ui, &mut p.width, 1..=6, "width");
            ch |= slider_f32(ui, &mut p.amount, 0.0..=0.6, "amount");
        }
    }
    ch
}

impl MkXelzApp {
    pub fn new(cc: &eframe::CreationContext<'_>, data_dir: PathBuf) -> Self {
        // Bigger touch targets.
        let mut style = (*cc.egui_ctx.style()).clone();
        style.spacing.interact_size.y = 34.0;
        style.spacing.slider_width = 150.0;
        style.spacing.button_padding = egui::vec2(10.0, 6.0);
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        cc.egui_ctx.set_style(style);

        // Seed the specs folder with the built-in presets (never overwrites).
        let specs_dir = data_dir.join("specs");
        let _ = std::fs::create_dir_all(&specs_dir);
        for (name, json) in presets::PRESETS {
            let path = specs_dir.join(format!("{name}.json"));
            if !path.exists() {
                let _ = std::fs::write(&path, *json);
            }
        }

        let spec = presets::get("dungeon")
            .and_then(|j| Spec::from_json(j).ok())
            .unwrap_or_default();

        let mut app = MkXelzApp {
            data_dir,
            spec,
            tab: Tab::Preview,
            output: None,
            atlas_tex: None,
            preview_tex: None,
            preview_cols: 10,
            preview_rows: 8,
            dirty: false,
            auto: true,
            regen_requested: false,
            status: String::new(),
            spec_files: Vec::new(),
            json_text: String::new(),
            selected_tile: 0,
        };
        app.refresh_files();
        app.json_text = app.spec.to_json().unwrap_or_default();
        app.regenerate(&cc.egui_ctx);
        app
    }

    fn regenerate(&mut self, ctx: &egui::Context) {
        match generate(&self.spec) {
            Ok(out) => {
                let atlas_img = color_image(&out.atlas);
                self.atlas_tex =
                    Some(ctx.load_texture("atlas", atlas_img, egui::TextureOptions::NEAREST));
                let prev =
                    compose_preview(&out, self.preview_cols, self.preview_rows, self.spec.seed);
                self.preview_tex = Some(ctx.load_texture(
                    "preview",
                    color_image(&prev),
                    egui::TextureOptions::NEAREST,
                ));
                self.status = format!(
                    "{} cells, atlas {}x{}",
                    out.entries.len(),
                    out.atlas.width,
                    out.atlas.height
                );
                self.output = Some(out);
            }
            Err(e) => {
                self.status = format!("Error: {e}");
            }
        }
        self.dirty = false;
        self.regen_requested = false;
    }

    fn refresh_files(&mut self) {
        let mut files = Vec::new();
        if let Ok(rd) = std::fs::read_dir(self.data_dir.join("specs")) {
            for entry in rd.flatten() {
                let p = entry.path();
                if p.extension().and_then(|e| e.to_str()) == Some("json") {
                    files.push(p);
                }
            }
        }
        files.sort();
        self.spec_files = files;
    }

    fn random_seed() -> u64 {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1);
        mkxelz_core::rng::mix(nanos) % 1_000_000
    }

    fn ui_preview(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Regenerate").clicked() {
                self.regen_requested = true;
            }
            ui.checkbox(&mut self.auto, "Auto");
        });
        ui.horizontal(|ui| {
            ui.label("Seed");
            if ui.add(egui::DragValue::new(&mut self.spec.seed)).changed() {
                self.dirty = true;
            }
            if ui.button("Random").clicked() {
                self.spec.seed = Self::random_seed();
                self.dirty = true;
            }
        });
        ui.label(&self.status);
        if let Some(tex) = &self.preview_tex {
            show_tex(ui, tex);
        }
        ui.separator();
        ui.label("Atlas");
        if let Some(tex) = &self.atlas_tex {
            show_tex(ui, tex);
        }
    }

    fn ui_controls(&mut self, ui: &mut egui::Ui) {
        let mut changed = false;

        ui.label("Name");
        ui.text_edit_singleline(&mut self.spec.name);

        egui::ComboBox::from_label("Tile size")
            .selected_text(self.spec.tile_size.to_string())
            .show_ui(ui, |ui| {
                for s in [16u32, 24, 32, 48, 64] {
                    if ui
                        .selectable_value(&mut self.spec.tile_size, s, s.to_string())
                        .changed()
                    {
                        changed = true;
                    }
                }
            });

        changed |= slider_u32(ui, &mut self.spec.padding, 0..=4, "atlas padding");
        changed |= slider_u32(
            ui,
            &mut self.spec.columns,
            0..=16,
            "atlas columns (0 = auto)",
        );

        ui.separator();
        ui.heading("Switches");
        if self.spec.switches.is_empty() {
            ui.label("This spec defines no switches.");
        }
        for (name, value) in self.spec.switches.iter_mut() {
            if ui.checkbox(value, name.as_str()).changed() {
                changed = true;
            }
        }

        ui.separator();
        ui.heading("Preview map");
        let mut map_changed = false;
        map_changed |= slider_u32(ui, &mut self.preview_cols, 6..=20, "width");
        map_changed |= slider_u32(ui, &mut self.preview_rows, 6..=16, "height");

        if changed || map_changed {
            self.dirty = true;
        }
    }

    fn ui_layers(&mut self, ui: &mut egui::Ui) {
        if self.spec.tiles.is_empty() {
            ui.label("No tiles in this spec.");
            return;
        }
        if self.selected_tile >= self.spec.tiles.len() {
            self.selected_tile = 0;
        }
        let names: Vec<String> = self.spec.tiles.iter().map(|t| t.name.clone()).collect();
        let switch_names: Vec<String> = self.spec.switches.keys().cloned().collect();

        egui::ComboBox::from_label("Tile")
            .selected_text(names[self.selected_tile].clone())
            .show_ui(ui, |ui| {
                for (i, n) in names.iter().enumerate() {
                    ui.selectable_value(&mut self.selected_tile, i, n.as_str());
                }
            });

        let idx = self.selected_tile;
        let mut changed = false;
        let tile = &mut self.spec.tiles[idx];

        ui.label(format!("kind: {:?}", tile.kind));
        changed |= slider_u32(ui, &mut tile.variants, 1..=8, "variants");
        changed |= ramp_picker(ui, "ramp", &mut tile.ramp);
        changed |= slider_f32(ui, &mut tile.dither, 0.0..=1.0, "dither");
        changed |= autotile_picker(ui, &mut tile.autotile);
        if tile.autotile != Autotile::Off {
            changed |= opt_ramp_picker(ui, "edge ramp", &mut tile.edge_ramp);
            changed |= slider_f32(ui, &mut tile.edge.width, 0.0..=8.0, "edge width");
            changed |= slider_f32(ui, &mut tile.edge.wobble, 0.0..=1.0, "edge wobble");
            changed |= slider_f32(ui, &mut tile.edge.shade, -0.6..=0.6, "edge shade");
            changed |= ui
                .checkbox(&mut tile.edge.cutout, "edge cutout (transparent)")
                .changed();
        }
        if tile.kind == Kind::Wall {
            changed |= slider_f32(ui, &mut tile.wall.face, 0.2..=0.85, "face height");
            changed |= slider_f32(ui, &mut tile.wall.cap_lift, 0.0..=0.5, "cap lift");
            changed |= slider_f32(ui, &mut tile.wall.face_shadow, 0.0..=0.6, "face shadow");
            changed |= ui.checkbox(&mut tile.wall.lip, "cap lip").changed();
        }

        ui.separator();
        ui.heading("Layers");

        let mut action: Option<(usize, LayerAction)> = None;
        let count = tile.layers.len();
        for (i, layer) in tile.layers.iter_mut().enumerate() {
            let title = format!("{}. {}", i + 1, layer.kind.label());
            egui::CollapsingHeader::new(title)
                .id_salt((idx, i))
                .show(ui, |ui| {
                    ui.push_id((idx, i), |ui| {
                        changed |= edit_layer(ui, layer, &switch_names);
                        ui.horizontal(|ui| {
                            if i > 0 && ui.button("Up").clicked() {
                                action = Some((i, LayerAction::Up));
                            }
                            if i + 1 < count && ui.button("Down").clicked() {
                                action = Some((i, LayerAction::Down));
                            }
                            if ui.button("Remove").clicked() {
                                action = Some((i, LayerAction::Remove));
                            }
                        });
                    });
                });
        }

        if let Some((i, act)) = action {
            match act {
                LayerAction::Remove => {
                    tile.layers.remove(i);
                }
                LayerAction::Up => tile.layers.swap(i, i - 1),
                LayerAction::Down => tile.layers.swap(i, i + 1),
            }
            changed = true;
        }

        egui::ComboBox::from_label("Add layer")
            .selected_text("choose...")
            .show_ui(ui, |ui| {
                for kind in LayerKind::catalog() {
                    if ui.selectable_label(false, kind.label()).clicked() {
                        tile.layers.push(Layer {
                            when: None,
                            unless: None,
                            region: Region::All,
                            kind,
                        });
                        changed = true;
                    }
                }
            });

        if changed {
            self.dirty = true;
        }
    }

    fn ui_files(&mut self, ui: &mut egui::Ui) {
        ui.heading("Export");
        ui.label("Writes tiles.png, tiles.json, spec.json and preview.png.");
        if ui.button("Export now").clicked() {
            self.export();
        }
        ui.label(format!("Folder: {}", self.data_dir.display()));

        ui.separator();
        ui.heading("Specs");
        ui.horizontal(|ui| {
            if ui.button("Save spec").clicked() {
                self.save_spec();
            }
            if ui.button("Refresh list").clicked() {
                self.refresh_files();
            }
        });
        let mut to_load: Option<PathBuf> = None;
        for p in &self.spec_files {
            let label = p
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("?")
                .to_string();
            if ui.button(format!("Load {label}")).clicked() {
                to_load = Some(p.clone());
            }
        }
        if let Some(p) = to_load {
            self.load_spec(&p);
        }

        ui.separator();
        ui.heading("Spec JSON");
        ui.horizontal(|ui| {
            if ui.button("From current").clicked() {
                self.json_text = self.spec.to_json().unwrap_or_default();
            }
            if ui.button("Apply").clicked() {
                match Spec::from_json(&self.json_text) {
                    Ok(s) => {
                        self.spec = s;
                        self.selected_tile = 0;
                        self.regen_requested = true;
                        self.status = "Applied JSON".to_string();
                    }
                    Err(e) => self.status = format!("JSON error: {e}"),
                }
            }
            if ui.button("Copy").clicked() {
                ui.ctx().copy_text(self.json_text.clone());
            }
        });
        ui.add(
            egui::TextEdit::multiline(&mut self.json_text)
                .code_editor()
                .desired_rows(14)
                .desired_width(f32::INFINITY),
        );
        ui.label(&self.status);
    }

    fn export(&mut self) {
        let Some(out) = &self.output else {
            self.status = "Nothing to export yet".to_string();
            return;
        };
        let folder: String = self
            .spec
            .name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let dir = self.data_dir.join("export").join(if folder.is_empty() {
            "untitled".to_string()
        } else {
            folder
        });
        self.status = match out.export(&dir) {
            Ok(files) => format!("Exported {} files to {}", files.len(), dir.display()),
            Err(e) => format!("Export failed: {e}"),
        };
    }

    fn save_spec(&mut self) {
        let dir = self.data_dir.join("specs");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join(format!("{}.json", self.spec.name));
        self.status = match self
            .spec
            .to_json()
            .map_err(|e| e.to_string())
            .and_then(|j| std::fs::write(&path, j).map_err(|e| e.to_string()))
        {
            Ok(()) => format!("Saved {}", path.display()),
            Err(e) => format!("Save failed: {e}"),
        };
        self.refresh_files();
    }

    fn load_spec(&mut self, path: &PathBuf) {
        match std::fs::read_to_string(path) {
            Ok(text) => match Spec::from_json(&text) {
                Ok(s) => {
                    self.spec = s;
                    self.selected_tile = 0;
                    self.json_text = self.spec.to_json().unwrap_or_default();
                    self.regen_requested = true;
                    self.status = format!("Loaded {}", path.display());
                }
                Err(e) => self.status = format!("Parse error: {e}"),
            },
            Err(e) => self.status = format!("Read error: {e}"),
        }
    }
}

impl eframe::App for MkXelzApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("tabs").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(&mut self.tab, Tab::Preview, "Preview");
                ui.selectable_value(&mut self.tab, Tab::Controls, "Controls");
                ui.selectable_value(&mut self.tab, Tab::Layers, "Layers");
                ui.selectable_value(&mut self.tab, Tab::Files, "Files");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| match self.tab {
                    Tab::Preview => self.ui_preview(ui),
                    Tab::Controls => self.ui_controls(ui),
                    Tab::Layers => self.ui_layers(ui),
                    Tab::Files => self.ui_files(ui),
                });
        });

        if self.regen_requested || (self.dirty && self.auto) {
            self.regenerate(ctx);
        }
    }
}
