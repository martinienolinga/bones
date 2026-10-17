//! Options d'affichage de la vue 3D, en interrupteurs.

use bones_core::Editor;
use bones_ui::widgets;

pub fn show(ui: &mut egui::Ui, editor: &mut Editor) {
    ui.spacing_mut().item_spacing.y = 6.0;
    let v = &mut editor.view;
    for (value, label, description) in [
        (&mut v.mesh, "Mesh", "the model deformed by the skeleton"),
        (
            &mut v.textures,
            "Textures & materials",
            "otherwise a uniform clay look",
        ),
        (&mut v.bones, "Bones", "X-ray skeleton"),
        (&mut v.grid, "Ground grid", "one-metre reference"),
        (&mut v.weights, "Weight map", "influence of the selected bone"),
        (&mut v.trajectory, "Trail", "path of the selected bone"),
        (&mut v.ghosts, "Ghosts", "poses of neighbouring keys"),
        (
            &mut v.center_of_mass,
            "Center of mass",
            "balance of the character",
        ),
    ] {
        widgets::switch_row(ui, value, label, description);
    }
}
