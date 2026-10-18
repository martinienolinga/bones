//! Lancer de rayon sur un maillage triangulé (sélection d'un objet au
//! doigt) : intersection rayon–triangle de Möller & Trumbore, « Fast,
//! Minimum Storage Ray/Triangle Intersection » (1997), deux faces, en
//! parallèle sur les triangles.

use glam::{Mat4, Vec3};
use rayon::prelude::*;

use crate::Ray;

/// Triangles par tâche parallèle.
const CHUNK: usize = 4096;

/// Paramètre `t ≥ 0` de la première intersection du rayon `origin + t·dir`
/// avec un triangle (`dir` n'a pas besoin d'être unitaire).
pub fn ray_triangle(origin: Vec3, dir: Vec3, a: Vec3, b: Vec3, c: Vec3) -> Option<f32> {
    let (e1, e2) = (b - a, c - a);
    let p = dir.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1.0 / det;
    let s = origin - a;
    let u = s.dot(p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = dir.dot(q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = e2.dot(q) * inv;
    (t >= 0.0 && t.is_finite()).then_some(t)
}

/// Première intersection d'un rayon avec des triangles (`positions` dans
/// le même espace que le rayon) ; les indices invalides sont ignorés.
pub fn ray_mesh(origin: Vec3, dir: Vec3, positions: &[Vec3], indices: &[u32]) -> Option<f32> {
    let n = positions.len();
    let tris = indices.as_chunks::<3>().0;
    tris.par_chunks(CHUNK)
        .filter_map(|chunk| {
            chunk
                .iter()
                .filter(|t| t.iter().all(|i| (*i as usize) < n))
                .filter_map(|t| {
                    let [a, b, c] = t.map(|i| positions[i as usize]);
                    ray_triangle(origin, dir, a, b, c)
                })
                .min_by(f32::total_cmp)
        })
        .min_by(f32::total_cmp)
}

/// Distance le long du rayon (monde) jusqu'à un maillage placé par `model`.
/// Le rayon est ramené dans l'espace du maillage : avec une direction non
/// renormalisée, le paramètre d'intersection est le même dans les deux espaces.
pub fn ray_model(ray: &Ray, model: Mat4, positions: &[Vec3], indices: &[u32]) -> Option<f32> {
    let inv = model.inverse();
    if !inv.is_finite() {
        return None;
    }
    let origin = inv.transform_point3(ray.origin);
    let dir = inv.transform_vector3(ray.dir);
    ray_mesh(origin, dir, positions, indices)
}
