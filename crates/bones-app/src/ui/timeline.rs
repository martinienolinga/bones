//! Frise d'animation en bas de l'écran (mode Pose), façon Cascadeur :
//! lecture, navigation entre clés, interpolation par intervalle, saut
//! balistique.

use bones_core::{Editor, Interpolation};
use bones_ui::icons::Icon;
use bones_ui::{theme, widgets};

use super::{EDGE, block};

const INTERPOLATIONS: [Interpolation; 3] =
    [Interpolation::Step, Interpolation::Linear, Interpolation::Smooth];
/// Largeur réservée aux commandes de droite (clés, interpolation…).
const RIGHT: f32 = 360.0;

pub fn show(ctx: &egui::Context, editor: &mut Editor) -> egui::Rect {
    let width = (ctx.content_rect().width() - 2.0 * EDGE - 160.0).max(420.0);
    block(
        ctx,
        "timeline",
        egui::Align2::CENTER_BOTTOM,
        [0.0, -EDGE],
        theme::bar_frame(),
        |ui| {
            ui.set_width(width);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                transport(ui, editor);
                ui.add_space(6.0);
                let keys: Vec<u32> = editor.rig.clip.keys().iter().map(|k| k.frame).collect();
                let mut frame = editor.frame;
                let strip_width = (ui.available_width() - RIGHT).max(140.0);
                ui.allocate_ui(egui::vec2(strip_width, 50.0), |ui| {
                    if widgets::key_strip(ui, &mut frame, editor.rig.clip.frame_count, &keys, 50.0).changed()
                    {
                        editor.playing = false;
                        editor.set_frame(frame);
                    }
                });
                ui.add_space(6.0);
                keys_and_interpolation(ui, editor);
            });
        },
    )
}

fn transport(ui: &mut egui::Ui, editor: &mut Editor) {
    if widgets::tool_button(ui, Icon::PrevKey, false, "Previous key").clicked() {
        editor.jump_key(false);
    }
    let (icon, hint) = if editor.playing {
        (Icon::Pause, "Pause")
    } else {
        (Icon::Play, "Play")
    };
    if widgets::tool_button(ui, icon, editor.playing, hint).clicked() {
        editor.toggle_play();
    }
    if widgets::tool_button(ui, Icon::NextKey, false, "Next key").clicked() {
        editor.jump_key(true);
    }
    // Compteur d'images, chiffres à chasse fixe pour ne pas trembler.
    let text = format!("{:>3}/{}", editor.current_frame(), editor.rig.clip.frame_count);
    ui.add_sized(
        [64.0, 44.0],
        egui::Label::new(egui::RichText::new(text).monospace().color(theme::TEXT_DIM)),
    );
}

fn keys_and_interpolation(ui: &mut egui::Ui, editor: &mut Editor) {
    let on_key = editor.rig.clip.key_at(editor.current_frame()).is_some();
    if widgets::tool_button(ui, Icon::Key, on_key, "Add key").clicked() {
        editor.add_key();
    }
    if ui
        .add_enabled_ui(on_key, |ui| {
            widgets::tool_button(ui, Icon::KeyRemove, false, "Delete key")
        })
        .inner
        .clicked()
    {
        editor.delete_key();
    }
    if let Some(current) = editor.interpolation() {
        let mut index = INTERPOLATIONS.iter().position(|i| *i == current).unwrap_or(2);
        ui.vertical(|ui| {
            ui.add_space(4.0);
            if widgets::segmented(ui, &["Step", "Linear", "Smooth"], &mut index).changed() {
                editor.set_interpolation(INTERPOLATIONS[index]);
            }
        });
    }
    if widgets::tool_button(
        ui,
        Icon::Ballistic,
        false,
        "Ballistic jump over the current interval",
    )
    .clicked()
    {
        editor.bake_ballistic();
    }
    let mut count = editor.rig.clip.frame_count;
    ui.vertical(|ui| {
        ui.add_space(6.0);
        let response = ui
            .add_sized(
                [72.0, 32.0],
                egui::DragValue::new(&mut count).range(2..=100_000).suffix(" fr"),
            )
            .on_hover_text("Frame count");
        if response.changed() {
            editor.set_frame_count(count);
        }
    });
}
