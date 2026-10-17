//! Tests de rendu : chaque icône et chaque widget se dessine sans erreur
//! dans un contexte egui sans GPU.

use egui::{Color32, RawInput, Rect, pos2, vec2};

use crate::{icons, theme, widgets};

fn run(mut ui_fn: impl FnMut(&mut egui::Ui)) -> egui::FullOutput {
    let ctx = egui::Context::default();
    theme::apply(&ctx);
    let input = RawInput {
        screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1024.0, 768.0))),
        ..Default::default()
    };
    let mut output = ctx.run_ui(input, |ui| ui_fn(ui));
    // Pas de GPU ici : les textures (atlas de polices) ne sont pas envoyées.
    output.textures_delta.clear();
    output
}

#[test]
fn every_icon_paints_shapes() {
    assert_eq!(icons::all().len(), 45);
    for &icon in icons::all() {
        let output = run(|ui| {
            let rect = Rect::from_min_size(pos2(10.0, 10.0), vec2(24.0, 24.0));
            icons::paint(ui.painter(), rect, icon, Color32::WHITE);
        });
        assert!(!output.shapes.is_empty(), "{icon:?} ne dessine rien");
    }
}

#[test]
fn widgets_render() {
    let mut value = 3.0;
    let mut frame = 12.0;
    let mut mode = 1;
    let output = run(|ui| {
        widgets::tool_button(ui, icons::Icon::Bone, true, "Os");
        widgets::tool_cell(ui, icons::Icon::Ik, "IK", false);
        widgets::text_button(ui, "Galerie", false);
        widgets::separator_dot(ui);
        widgets::segmented(ui, &["Pose", "Édition", "Auto-Rig"], &mut mode);
        widgets::vertical_slider(ui, &mut value, 1.0..=6.0, "Chaîne IK");
        widgets::vertical_slider_with_icon(ui, &mut value, 1.0..=6.0, icons::Icon::Chain, "Chaîne IK");
        widgets::key_strip(ui, &mut frame, 120, &[0, 12, 60], 64.0);
    });
    assert!(!output.shapes.is_empty());
    // Sans interaction, rien ne change.
    assert_eq!((value, frame, mode), (3.0, 12.0, 1));
}
