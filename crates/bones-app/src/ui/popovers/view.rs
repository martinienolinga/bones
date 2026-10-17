//! Options d'affichage de la vue 3D.

use bones_core::Editor;

pub fn show(ui: &mut egui::Ui, editor: &mut Editor) {
    let v = &mut editor.view;
    for (value, label) in [
        (&mut v.mesh, "Maillage"),
        (&mut v.bones, "Os"),
        (&mut v.grid, "Grille au sol"),
        (&mut v.weights, "Carte de poids de l'os sélectionné"),
        (&mut v.trajectory, "Trajectoire de l'os sélectionné"),
        (&mut v.ghosts, "Fantômes des clés voisines"),
        (&mut v.center_of_mass, "Centre de masse"),
    ] {
        ui.checkbox(value, label);
    }
}
