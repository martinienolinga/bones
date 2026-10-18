//! Primitives à ajouter comme objets (menu « Add » de Blender) : cube,
//! sphère UV, cylindre, cône, plan et tore. Toutes tiennent dans un cube
//! d'un mètre centré sur l'origine (le plan est horizontal, à y = 0), avec
//! normales et coordonnées de texture ; triangles dans le sens direct vus de
//! l'extérieur.

use std::f32::consts::{PI, TAU};

use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

use crate::Mesh;

/// Segments autour de l'axe des primitives de révolution.
const SEGMENTS: u32 = 32;
/// Anneaux de la sphère et du tube du tore.
const RINGS: u32 = 16;

/// Primitive géométrique.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Primitive {
    Cube,
    Sphere,
    Cylinder,
    Cone,
    Plane,
    Torus,
}

impl Primitive {
    pub const ALL: [Primitive; 6] = [
        Primitive::Cube,
        Primitive::Sphere,
        Primitive::Cylinder,
        Primitive::Cone,
        Primitive::Plane,
        Primitive::Torus,
    ];

    /// Nom affiché (et nom par défaut de l'objet).
    pub fn label(self) -> &'static str {
        match self {
            Primitive::Cube => "Cube",
            Primitive::Sphere => "Sphere",
            Primitive::Cylinder => "Cylinder",
            Primitive::Cone => "Cone",
            Primitive::Plane => "Plane",
            Primitive::Torus => "Torus",
        }
    }

    pub fn mesh(self) -> Mesh {
        let mut m = Mesh::default();
        match self {
            Primitive::Cube => cube(&mut m),
            Primitive::Sphere => {
                let profile: Vec<_> = (0..=RINGS)
                    .map(|i| {
                        let phi = -PI / 2.0 + PI * i as f32 / RINGS as f32;
                        (0.5 * phi.cos(), 0.5 * phi.sin(), Vec2::new(phi.cos(), phi.sin()))
                    })
                    .collect();
                lathe(&mut m, &profile);
            }
            Primitive::Cylinder => {
                cap(&mut m, 0.5, -0.5, false);
                lathe(&mut m, &[(0.5, -0.5, Vec2::X), (0.5, 0.5, Vec2::X)]);
                cap(&mut m, 0.5, 0.5, true);
            }
            Primitive::Cone => {
                cap(&mut m, 0.5, -0.5, false);
                let n = Vec2::new(1.0, 0.5).normalize();
                lathe(&mut m, &[(0.5, -0.5, n), (0.0, 0.5, n)]);
            }
            Primitive::Plane => {
                let corners = [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.5), (-0.5, 0.5)];
                for (x, z) in corners {
                    m.positions.push(Vec3::new(x, 0.0, z));
                    m.normals.push(Vec3::Y);
                    m.uvs.push(Vec2::new(x + 0.5, z + 0.5));
                }
                m.indices.extend_from_slice(&[0, 3, 2, 0, 2, 1]);
            }
            Primitive::Torus => {
                let (major, minor) = (0.375, 0.125);
                let profile: Vec<_> = (0..=RINGS)
                    .map(|j| {
                        let phi = TAU * j as f32 / RINGS as f32;
                        let n = Vec2::new(phi.cos(), phi.sin());
                        (major + minor * n.x, minor * n.y, n)
                    })
                    .collect();
                lathe(&mut m, &profile);
            }
        }
        m
    }
}

/// Surface de révolution autour de Y : profil `(rayon, hauteur, normale
/// (radiale, verticale))` parcouru vers le haut côté extérieur ; la couture
/// est dupliquée pour les coordonnées de texture.
fn lathe(m: &mut Mesh, profile: &[(f32, f32, Vec2)]) {
    let base = m.positions.len() as u32;
    let row = SEGMENTS + 1;
    let last = profile.len().saturating_sub(1).max(1) as f32;
    for (i, (r, y, n)) in profile.iter().enumerate() {
        for s in 0..row {
            let theta = TAU * s as f32 / SEGMENTS as f32;
            let (sin, cos) = theta.sin_cos();
            m.positions.push(Vec3::new(r * sin, *y, r * cos));
            m.normals
                .push(Vec3::new(n.x * sin, n.y, n.x * cos).normalize_or(Vec3::Y));
            m.uvs
                .push(Vec2::new(s as f32 / SEGMENTS as f32, 1.0 - i as f32 / last));
        }
    }
    for i in 0..profile.len().saturating_sub(1) as u32 {
        for s in 0..SEGMENTS {
            let a = base + i * row + s;
            let (b, c) = (a + 1, a + row);
            m.indices.extend_from_slice(&[a, b, c + 1, a, c + 1, c]);
        }
    }
}

/// Disque horizontal (éventail) de rayon `r` à la hauteur `y`, tourné vers
/// le haut ou vers le bas.
fn cap(m: &mut Mesh, r: f32, y: f32, up: bool) {
    let normal = if up { Vec3::Y } else { Vec3::NEG_Y };
    let center = m.positions.len() as u32;
    m.positions.push(Vec3::new(0.0, y, 0.0));
    m.normals.push(normal);
    m.uvs.push(Vec2::splat(0.5));
    for s in 0..SEGMENTS {
        let (sin, cos) = (TAU * s as f32 / SEGMENTS as f32).sin_cos();
        m.positions.push(Vec3::new(r * sin, y, r * cos));
        m.normals.push(normal);
        m.uvs.push(Vec2::new(0.5 + 0.5 * sin, 0.5 + 0.5 * cos));
    }
    for s in 0..SEGMENTS {
        let (p, q) = (center + 1 + s, center + 1 + (s + 1) % SEGMENTS);
        let tri = if up { [center, p, q] } else { [center, q, p] };
        m.indices.extend_from_slice(&tri);
    }
}

/// Cube à faces plates (24 sommets).
fn cube(m: &mut Mesh) {
    // (normale, axe u, axe v) avec u × v = normale.
    let faces = [
        (Vec3::X, Vec3::NEG_Z, Vec3::Y),
        (Vec3::NEG_X, Vec3::Z, Vec3::Y),
        (Vec3::Y, Vec3::X, Vec3::NEG_Z),
        (Vec3::NEG_Y, Vec3::X, Vec3::Z),
        (Vec3::Z, Vec3::X, Vec3::Y),
        (Vec3::NEG_Z, Vec3::NEG_X, Vec3::Y),
    ];
    for (n, u, v) in faces {
        let base = m.positions.len() as u32;
        for (a, b) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
            m.positions.push((n + u * a + v * b) * 0.5);
            m.normals.push(n);
            m.uvs.push(Vec2::new((a + 1.0) * 0.5, (1.0 - b) * 0.5));
        }
        m.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
}
