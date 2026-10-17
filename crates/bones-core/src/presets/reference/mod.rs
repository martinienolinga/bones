//! Squelettes de référence de chaque famille : aperçu des préréglages
//! procéduraux dans la galerie, export (BVH, glTF) et tests. Tous suivent
//! les conventions de l'application (Y en haut, face à +Z, gauche en +X,
//! noms à la Auto-Rig Pro), en mètres.

mod animals;
mod plants;

use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::{AutoRigOptions, BoneId, Markers, Skeleton, autorig};

/// Squelette de référence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RefRig {
    Humanoid,
    Quadruped,
    Bird,
    Fish,
    Serpent,
    Spider,
    Insect,
    Tree,
    Grass,
    Flower,
    Palm,
    Rope,
    Tentacle,
    Pendulum,
    Machine,
}

impl RefRig {
    pub const ALL: [RefRig; 15] = [
        RefRig::Humanoid,
        RefRig::Quadruped,
        RefRig::Bird,
        RefRig::Fish,
        RefRig::Serpent,
        RefRig::Spider,
        RefRig::Insect,
        RefRig::Tree,
        RefRig::Grass,
        RefRig::Flower,
        RefRig::Palm,
        RefRig::Rope,
        RefRig::Tentacle,
        RefRig::Pendulum,
        RefRig::Machine,
    ];

    pub fn label(self) -> &'static str {
        match self {
            RefRig::Humanoid => "Human",
            RefRig::Quadruped => "Dog",
            RefRig::Bird => "Bird",
            RefRig::Fish => "Fish",
            RefRig::Serpent => "Snake",
            RefRig::Spider => "Spider",
            RefRig::Insect => "Insect",
            RefRig::Tree => "Tree",
            RefRig::Grass => "Grass blade",
            RefRig::Flower => "Flower",
            RefRig::Palm => "Palm tree",
            RefRig::Rope => "Rope",
            RefRig::Tentacle => "Tentacle",
            RefRig::Pendulum => "Pendulum",
            RefRig::Machine => "Machine",
        }
    }

    /// Construit le squelette.
    pub fn skeleton(self) -> Skeleton {
        match self {
            RefRig::Humanoid => humanoid(),
            RefRig::Quadruped => animals::quadruped(),
            RefRig::Bird => animals::bird(),
            RefRig::Fish => animals::fish(),
            RefRig::Serpent => animals::serpent(),
            RefRig::Spider => animals::many_legs(4),
            RefRig::Insect => animals::many_legs(3),
            RefRig::Tree => plants::tree(),
            RefRig::Grass => plants::grass(),
            RefRig::Flower => plants::flower(),
            RefRig::Palm => plants::palm(),
            RefRig::Rope => plants::rope(),
            RefRig::Tentacle => plants::tentacle(),
            RefRig::Pendulum => plants::pendulum(),
            RefRig::Machine => plants::machine(),
        }
    }
}

/// Humain de 1,80 m en pose T, généré par l'auto-rig (mêmes noms et
/// proportions que les personnages de l'application).
fn humanoid() -> Skeleton {
    let markers = Markers {
        head_top: Vec3::new(0.0, 1.80, 0.0),
        chin: Vec3::new(0.0, 1.57, 0.02),
        neck: Vec3::new(0.0, 1.51, 0.0),
        shoulder: Vec3::new(0.19, 1.45, 0.0),
        wrist: Vec3::new(0.72, 1.45, 0.0),
        root: Vec3::new(0.0, 0.98, 0.0),
        ankle: Vec3::new(0.10, 0.08, 0.0),
    };
    autorig::generate(&markers, &AutoRigOptions::default()).0
}

/// Petit outil de construction : os définis par tête et queue monde.
pub(super) struct Builder(pub Skeleton);

impl Builder {
    pub fn new() -> Self {
        Self(Skeleton::new())
    }

    pub fn bone(&mut self, name: &str, parent: Option<BoneId>, head: [f32; 3], tail: [f32; 3]) -> BoneId {
        self.0
            .add_bone_world(name, parent, Vec3::from_array(head), Vec3::from_array(tail))
    }

    /// Chaîne de points ; retourne les os créés.
    pub fn chain(&mut self, base: &str, parent: Option<BoneId>, points: &[Vec3]) -> Vec<BoneId> {
        let mut parent = parent;
        let mut out = Vec::with_capacity(points.len().saturating_sub(1));
        for (i, pair) in points.windows(2).enumerate() {
            let name = base.replacen('#', &format!("{:02}", i + 1), 1);
            let b = self.0.add_bone_world(name, parent, pair[0], pair[1]);
            out.push(b);
            parent = Some(b);
        }
        out
    }

    /// Côté gauche puis son symétrique (`.l` → `.r`).
    pub fn pair(&mut self, f: impl Fn(&mut Builder, &str, f32)) {
        f(self, "l", 1.0);
        f(self, "r", -1.0);
    }
}
