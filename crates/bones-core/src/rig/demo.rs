//! Mannequin de démonstration, riggé automatiquement.

use glam::Vec3;

use crate::autorig;
use crate::{BindMethod, Markers, Mesh, Rig};

impl Rig {
    /// Personnage de démonstration : un mannequin fait de capsules, riggé
    /// automatiquement.
    pub fn demo() -> Self {
        let mut mesh = Mesh::default();
        let c = |a: [f32; 3], b: [f32; 3], r: f32, rings: u32| {
            Mesh::capsule(Vec3::from_array(a), Vec3::from_array(b), r, 20, rings)
        };
        mesh.append(&c([0.0, 0.92, 0.0], [0.0, 1.42, 0.0], 0.16, 12));
        mesh.append(&c([0.0, 1.58, 0.01], [0.0, 1.74, 0.01], 0.1, 4));
        mesh.append(&c([0.0, 1.42, 0.0], [0.0, 1.56, 0.0], 0.05, 3));
        for x in [1.0f32, -1.0] {
            mesh.append(&c([0.14 * x, 1.42, 0.0], [0.82 * x, 1.42, 0.0], 0.045, 24));
            mesh.append(&c([0.1 * x, 0.92, 0.0], [0.1 * x, 0.1, 0.0], 0.065, 24));
            mesh.append(&c([0.1 * x, 0.04, -0.02], [0.1 * x, 0.04, 0.14], 0.045, 4));
        }
        mesh.shrink_to_fit();

        let mut rig = Rig::new("Mannequin", mesh);
        if let Some(markers) = Markers::guess(&rig.mesh) {
            let (skeleton, limbs) = autorig::generate(&markers, &Default::default());
            rig.set_skeleton(skeleton, limbs);
            rig.bind(BindMethod::Envelope);
        }
        rig
    }
}
