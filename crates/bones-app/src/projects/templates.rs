//! Modèles de nouveaux projets (menu « + » de la galerie, premier
//! lancement).

use bones_core::glam::Vec3;
use bones_core::{Mesh, Rig};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Template {
    /// Personnage de démonstration, riggé.
    Mannequin,
    /// Un seul os racine, sans maillage : on importe ou construit ensuite.
    Empty,
}

impl Template {
    /// Nom proposé pour le projet (rendu unique dans le dossier).
    pub fn name(self) -> &'static str {
        match self {
            Template::Mannequin => "Mannequin",
            Template::Empty => "Untitled",
        }
    }

    pub fn rig(self) -> Rig {
        match self {
            Template::Mannequin => mannequin_rig(),
            Template::Empty => empty_rig(),
        }
    }
}

/// Personnage du projet « Mannequin » : humain réaliste riggé et lié
/// (`bones_core::samples`). Coûteux à construire : toujours appelé sur le fil
/// des projets.
pub fn mannequin_rig() -> Rig {
    bones_core::samples::mannequin()
}

/// Projet vide : un os racine d'un mètre, debout à l'origine.
pub fn empty_rig() -> Rig {
    let mut rig = Rig::new(Template::Empty.name(), Mesh::default());
    rig.add_bone_world("root.x", None, Vec3::ZERO, Vec3::Y);
    rig
}
