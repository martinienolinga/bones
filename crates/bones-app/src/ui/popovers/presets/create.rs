//! Feuille de création d'un préréglage à partir de l'animation du projet :
//! nom, catégorie, groupe et étiquettes, intervalle d'images, boucle, puis
//! enregistrement dans le dossier `Presets` de l'utilisateur.

use std::path::Path;

use bones_core::Editor;
use bones_core::presets::{self, Category, CreateOptions};
use bones_ui::{theme, widgets};

use super::{PresetsState, View, presets_dir};

/// Catégories proposées (libellés courts) et leur valeur.
const CATEGORIES: [(&str, Category); 4] = [
    ("Human", Category::Human),
    ("Animals", Category::Animals),
    ("Plants", Category::Vegetation),
    ("Misc", Category::Misc),
];

pub struct CreateForm {
    name: String,
    group: String,
    tags: String,
    category: usize,
    first: u32,
    last: u32,
    looping: bool,
    /// L'intervalle a été initialisé sur la longueur du clip.
    ready: bool,
}

impl Default for CreateForm {
    fn default() -> Self {
        Self {
            name: "My Preset".to_owned(),
            group: String::new(),
            tags: String::new(),
            category: 0,
            first: 0,
            last: 0,
            looping: false,
            ready: false,
        }
    }
}

pub fn show(ui: &mut egui::Ui, editor: &mut Editor, p: &mut PresetsState, docs: &Path, body: f32) {
    let last_frame = editor.rig.clip.frame_count.saturating_sub(1);
    if !p.create.ready {
        p.create.last = last_frame;
        p.create.ready = true;
    }
    p.create.last = p.create.last.min(last_frame);
    let mut save = false;
    egui::ScrollArea::vertical()
        .id_salt("preset-create")
        .auto_shrink([false, false])
        .max_height(body)
        .show(ui, |ui| {
            let f = &mut p.create;
            field(ui, "Name");
            text_field(ui, &mut f.name);
            ui.add_space(12.0);
            field(ui, "Category");
            let labels: Vec<&str> = CATEGORIES.iter().map(|(l, _)| *l).collect();
            widgets::segmented(ui, &labels, &mut f.category);
            ui.add_space(12.0);
            field(ui, "Group (optional)");
            text_field(ui, &mut f.group);
            ui.add_space(12.0);
            field(ui, "Tags (comma separated)");
            text_field(ui, &mut f.tags);
            ui.add_space(12.0);
            field(ui, "Frame range");
            range_row(ui, f, last_frame);
            ui.add_space(8.0);
            widgets::switch_row(ui, &mut f.looping, "Loop", "Mark as a looping cycle");
            ui.add_space(16.0);
            if widgets::pill_button(ui, Some(bones_ui::icons::Icon::Save), "Save preset", true).clicked() {
                save = true;
            }
        });
    if save {
        run(editor, p, docs);
    }
}

/// Prépare l'intervalle et enregistre (chemin direct pour les tests).
#[cfg(test)]
pub(super) fn test_save(editor: &mut Editor, p: &mut PresetsState, docs: &Path) {
    p.create.ready = true;
    p.create.last = editor.rig.clip.frame_count.saturating_sub(1);
    run(editor, p, docs);
}

fn run(editor: &mut Editor, p: &mut PresetsState, docs: &Path) {
    let f = &p.create;
    let options = CreateOptions {
        name: f.name.clone(),
        category: CATEGORIES[f.category].1,
        group: f.group.clone(),
        tags: f
            .tags
            .split(',')
            .map(|t| t.trim().to_owned())
            .filter(|t| !t.is_empty())
            .collect(),
        author: String::new(),
        license: "CC0-1.0".to_owned(),
        first: f.first.min(f.last),
        last: f.last,
        looping: f.looping,
        ..CreateOptions::default()
    };
    match editor.create_preset(&options) {
        Ok(preset) => match presets::user::save(&preset, &presets_dir(docs)) {
            Ok(_) => {
                let name = preset.info.name.clone();
                p.refresh(docs);
                p.create.ready = false;
                p.view_slot = Some(View::Grid);
                p.notify(format!("Saved “{name}”"));
            }
            Err(e) => p.notify(format!("Error: {e}")),
        },
        Err(e) => p.notify(format!("Error: {e}")),
    }
}

/// Libellé d'un champ (gris, au-dessus du contrôle).
fn field(ui: &mut egui::Ui, label: &str) {
    ui.label(egui::RichText::new(label).size(13.0).color(theme::TEXT_DIM));
    ui.add_space(4.0);
}

/// Champ texte dans une pastille arrondie.
fn text_field(ui: &mut egui::Ui, text: &mut String) {
    egui::Frame::new()
        .fill(theme::WIDGET)
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.add(
                egui::TextEdit::singleline(text)
                    .frame(egui::Frame::NONE)
                    .desired_width(f32::INFINITY),
            );
        });
}

/// Intervalle d'images : deux pastilles « From » / « To ».
fn range_row(ui: &mut egui::Ui, f: &mut CreateForm, last_frame: u32) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        let mut from = f.first as f32;
        if widgets::drag_chip(ui, &mut from, 0.0..=last_frame as f32, 0.3, 0, "", 86.0).changed() {
            f.first = (from.round() as u32).min(last_frame);
        }
        ui.label(egui::RichText::new("to").size(14.0).color(theme::TEXT_DIM));
        let mut to = f.last as f32;
        if widgets::drag_chip(ui, &mut to, 0.0..=last_frame as f32, 0.3, 0, "", 86.0).changed() {
            f.last = (to.round() as u32).min(last_frame);
        }
        ui.label(
            egui::RichText::new(format!("of {last_frame}"))
                .size(13.0)
                .color(theme::TEXT_FAINT),
        );
    });
}
