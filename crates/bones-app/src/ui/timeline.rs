//! Frise d'animation en bas de l'écran (mode Pose), façon Cascadeur :
//! lecture, navigation entre clés, interpolation par intervalle, saut
//! balistique.

use bones_core::{Editor, Interpolation};
use bones_ui::icons::Icon;
use bones_ui::{theme, widgets};

use super::block;

const INTERPOLATIONS: [Interpolation; 3] =
    [Interpolation::Step, Interpolation::Linear, Interpolation::Smooth];

pub fn show(ctx: &egui::Context, editor: &mut Editor) -> egui::Rect {
    let width = (ctx.content_rect().width() - 140.0).max(320.0);
    block(
        ctx,
        "timeline",
        egui::Align2::CENTER_BOTTOM,
        [0.0, -8.0],
        theme::bar_frame(),
        |ui| {
            ui.set_width(width);
            ui.horizontal(|ui| {
                transport(ui, editor);
                ui.add_space(8.0);
                let keys: Vec<u32> = editor.rig.clip.keys().iter().map(|k| k.frame).collect();
                let mut frame = editor.frame;
                let strip_width = (ui.available_width() - 330.0).max(120.0);
                ui.allocate_ui(egui::vec2(strip_width, 48.0), |ui| {
                    if widgets::key_strip(ui, &mut frame, editor.rig.clip.frame_count, &keys, 44.0).changed()
                    {
                        editor.playing = false;
                        editor.set_frame(frame);
                    }
                });
                ui.add_space(8.0);
                keys_and_interpolation(ui, editor);
            });
        },
    )
}

fn transport(ui: &mut egui::Ui, editor: &mut Editor) {
    if widgets::tool_button(ui, Icon::PrevKey, false, "Clé précédente").clicked() {
        editor.jump_key(false);
    }
    let (icon, hint) = if editor.playing {
        (Icon::Pause, "Pause")
    } else {
        (Icon::Play, "Lecture")
    };
    if widgets::tool_button(ui, icon, editor.playing, hint).clicked() {
        editor.toggle_play();
    }
    if widgets::tool_button(ui, Icon::NextKey, false, "Clé suivante").clicked() {
        editor.jump_key(true);
    }
    ui.label(
        egui::RichText::new(format!(
            "{:>3} / {}",
            editor.current_frame(),
            editor.rig.clip.frame_count
        ))
        .monospace()
        .color(theme::TEXT_DIM),
    );
}

fn keys_and_interpolation(ui: &mut egui::Ui, editor: &mut Editor) {
    let on_key = editor.rig.clip.key_at(editor.current_frame()).is_some();
    if widgets::tool_button(ui, Icon::Key, on_key, "Poser une clé").clicked() {
        editor.add_key();
    }
    if ui
        .add_enabled_ui(on_key, |ui| {
            widgets::tool_button(ui, Icon::KeyRemove, false, "Supprimer la clé")
        })
        .inner
        .clicked()
    {
        editor.delete_key();
    }
    if let Some(current) = editor.interpolation() {
        let mut index = INTERPOLATIONS.iter().position(|i| *i == current).unwrap_or(2);
        if widgets::segmented(ui, &["Palier", "Linéaire", "Doux"], &mut index).changed() {
            editor.set_interpolation(INTERPOLATIONS[index]);
        }
    }
    if widgets::tool_button(
        ui,
        Icon::Ballistic,
        false,
        "Saut balistique sur l'intervalle courant",
    )
    .clicked()
    {
        editor.bake_ballistic();
    }
    let mut count = editor.rig.clip.frame_count;
    let response = ui.add(egui::DragValue::new(&mut count).range(2..=100_000).suffix(" im."));
    if response.changed() {
        editor.set_frame_count(count);
    }
}
