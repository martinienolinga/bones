//! Primitives géométriques (capsules du mannequin de démonstration).

use glam::Vec3;
use std::f32::consts::{FRAC_PI_2, TAU};

use crate::Mesh;
use crate::math::rotation_arc;

impl Mesh {
    /// Capsule (cylindre fermé par deux demi-sphères) entre `a` et `b`.
    ///
    /// `radial` : segments autour de l'axe ; `rings` : anneaux le long du
    /// cylindre (plus il y en a, plus la flexion est douce).
    pub fn capsule(a: Vec3, b: Vec3, radius: f32, radial: u32, rings: u32) -> Mesh {
        let radial = radial.max(3);
        let rings = rings.max(1);
        let cap = (radial / 4).max(2);
        let length = (b - a).length();

        // Profil (hauteur, rayon) du pôle bas au pôle haut.
        let mut profile = Vec::new();
        for k in 0..=cap {
            let t = -FRAC_PI_2 + FRAC_PI_2 * k as f32 / cap as f32;
            profile.push((radius * t.sin(), radius * t.cos()));
        }
        for k in 1..rings {
            profile.push((length * k as f32 / rings as f32, radius));
        }
        for k in 0..=cap {
            let t = FRAC_PI_2 * k as f32 / cap as f32;
            profile.push((length + radius * t.sin(), radius * t.cos()));
        }

        let rotation = rotation_arc(Vec3::Y, b - a);
        let mut mesh = Mesh::default();
        for &(y, r) in &profile {
            for s in 0..radial {
                let angle = TAU * s as f32 / radial as f32;
                let local = Vec3::new(r * angle.cos(), y, -r * angle.sin());
                mesh.positions.push(a + rotation * local);
            }
        }
        for ring in 0..profile.len() as u32 - 1 {
            for s in 0..radial {
                let i0 = ring * radial + s;
                let i1 = ring * radial + (s + 1) % radial;
                let i2 = i0 + radial;
                let i3 = i1 + radial;
                mesh.indices.extend_from_slice(&[i0, i1, i3, i0, i3, i2]);
            }
        }
        mesh.compute_normals();
        mesh
    }
}
