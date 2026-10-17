//! Options d'affichage de la vue 3D, en liste groupée d'interrupteurs.

use bones_core::Editor;
use bones_ui::widgets;

use super::header;

pub fn show(ui: &mut egui::Ui, editor: &mut Editor, max_height: f32) {
    header(ui, "View", None);
    ui.add_space(4.0);
    egui::ScrollArea::vertical()
        .max_height((max_height - 70.0).max(160.0))
        .show(ui, |ui| {
            let v = &mut editor.view;
            widgets::section_title(ui, "Model");
            widgets::group(ui, |g| {
                g.switch_row(&mut v.mesh, "Mesh", Some("The model deformed by the skeleton"));
                g.switch_row(
                    &mut v.textures,
                    "Textures & materials",
                    Some("Otherwise a uniform clay look"),
                );
                g.switch_row(
                    &mut v.weights,
                    "Weight map",
                    Some("Influence of the selected bone"),
                );
            });
            widgets::section_title(ui, "Rig");
            widgets::group(ui, |g| {
                g.switch_row(&mut v.bones, "Bones", Some("X-ray skeleton"));
                g.switch_row(&mut v.grid, "Ground grid", Some("One-metre reference"));
            });
            widgets::section_title(ui, "Motion");
            widgets::group(ui, |g| {
                g.switch_row(&mut v.trajectory, "Trail", Some("Path of the selected bone"));
                g.switch_row(&mut v.ghosts, "Ghosts", Some("Poses of neighbouring keys"));
                g.switch_row(
                    &mut v.center_of_mass,
                    "Center of mass",
                    Some("Balance of the character"),
                );
            });
        });
}
