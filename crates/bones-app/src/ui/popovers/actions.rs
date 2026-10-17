//! Actions, en onglets comme le menu Actions de Procreate : poids
//! automatiques, animation, export (un format par ligne).

use bones_core::io::Format;
use bones_core::{BindMethod, Editor};
use bones_ui::icons::Icon;
use bones_ui::widgets;

use super::{header, note};
use crate::ui::{UiAction, UiState};

const TABS: [(Icon, &str); 3] = [
    (Icon::Weights, "Weights"),
    (Icon::Play, "Animation"),
    (Icon::Export, "Export"),
];

pub fn show(
    ui: &mut egui::Ui,
    editor: &mut Editor,
    state: &mut UiState,
    actions: &mut Vec<UiAction>,
    max_height: f32,
) {
    header(ui, "Actions", None);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        for (i, (icon, label)) in TABS.iter().enumerate() {
            if widgets::tab(ui, *icon, label, state.actions_tab == i).clicked() {
                state.actions_tab = i;
            }
        }
    });
    ui.add_space(6.0);
    egui::ScrollArea::vertical()
        .max_height((max_height - 130.0).max(160.0))
        .show(ui, |ui| match state.actions_tab {
            0 => weights(ui, editor),
            1 => animation(ui, editor),
            _ => export(ui, actions),
        });
}

fn weights(ui: &mut egui::Ui, editor: &mut Editor) {
    widgets::section_title(ui, "Automatic weights");
    let current = editor.bind_method;
    let picked = widgets::group(ui, |g| {
        let mut picked = None;
        for (method, label) in [
            (BindMethod::Heat, "Heat diffusion"),
            (BindMethod::Envelope, "Envelopes"),
        ] {
            if g.check_row(label, current == method).clicked() {
                picked = Some(method);
            }
        }
        picked
    });
    if let Some(method) = picked.filter(|m| *m != current) {
        editor.bind_method = method;
        editor.request_rebind();
    }
    note(
        ui,
        match editor.bind_method {
            BindMethod::Heat => "High quality (Baran & Popović 2007), computed in the background.",
            BindMethod::Envelope => "Instant, based on the distance to the bones.",
        },
    );
    ui.add_space(4.0);
    if widgets::pill_button(ui, Some(Icon::Weights), "Recompute weights", false).clicked() {
        editor.request_rebind();
    }
}

fn animation(ui: &mut egui::Ui, editor: &mut Editor) {
    widgets::section_title(ui, "Frames per second");
    let fps = editor.rig.clip.fps;
    let picked = widgets::group(ui, |g| {
        let mut picked = None;
        for value in [12.0, 24.0, 30.0, 60.0] {
            if g.check_row(&format!("{value} fps"), fps == value).clicked() {
                picked = Some(value);
            }
        }
        let mut custom = fps;
        g.custom(46.0, |ui| {
            ui.label(
                egui::RichText::new("Custom")
                    .size(17.0)
                    .color(bones_ui::theme::TEXT_LIST),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::drag_chip(ui, &mut custom, 1.0..=120.0, 0.2, 0, " fps", 96.0).changed() {
                    picked = Some(custom);
                }
            });
        });
        picked
    });
    if let Some(value) = picked {
        editor.rig.clip_mut().fps = value;
    }
}

/// Un format par ligne, avec ce qu'il emporte.
fn export(ui: &mut egui::Ui, actions: &mut Vec<UiAction>) {
    widgets::section_title(ui, "Export");
    widgets::group(ui, |g| {
        for format in Format::exportable() {
            let mut contents = vec!["mesh"];
            if format.supports_textures() {
                contents.push("textures");
            }
            if format.exports_skeleton() {
                contents.push("skeleton");
            }
            if format.exports_animation() {
                contents.push("animation");
            }
            // « glTF binary (Blender, Unity…) » : titre court, détails dessous.
            let label = format.label();
            let (title, detail) = label.split_once(" (").unwrap_or((label, ""));
            let mut subtitle = contents.join(" · ");
            if !detail.is_empty() {
                subtitle = format!("{} — {subtitle}", detail.trim_end_matches(')'));
            }
            let badge = format!(".{}", format.extension());
            if g.icon_row(Icon::Export, title, Some(&subtitle), Some(&badge))
                .on_hover_text(label)
                .clicked()
            {
                actions.push(UiAction::Export(format));
            }
        }
    });
    note(
        ui,
        "Formats without a skeleton (OBJ, STL, PLY) export the current pose.",
    );
}
