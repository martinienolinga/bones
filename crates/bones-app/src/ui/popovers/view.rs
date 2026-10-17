//! Options d'affichage de la vue 3D, en interrupteurs.

use bones_core::Editor;
use bones_ui::widgets;

pub fn show(ui: &mut egui::Ui, editor: &mut Editor) {
    ui.spacing_mut().item_spacing.y = 6.0;
    let v = &mut editor.view;
    for (value, label, description) in [
        (&mut v.mesh, "Maillage", "le modèle déformé par le squelette"),
        (
            &mut v.textures,
            "Textures et matériaux",
            "sinon rendu « argile » uniforme",
        ),
        (&mut v.bones, "Os", "squelette en rayons X"),
        (&mut v.grid, "Grille au sol", "repère d'un mètre"),
        (&mut v.weights, "Carte de poids", "influence de l'os sélectionné"),
        (&mut v.trajectory, "Trajectoire", "chemin de l'os sélectionné"),
        (&mut v.ghosts, "Fantômes", "poses des clés voisines"),
        (
            &mut v.center_of_mass,
            "Centre de masse",
            "équilibre du personnage",
        ),
    ] {
        widgets::switch_row(ui, value, label, description);
    }
}
