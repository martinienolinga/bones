//! Géométrie fixe des os et matrices d'instance.

use glam::{Mat4, Vec3};

/// Octaèdre d'os à la Blender : tête en 0, queue en (0, 1, 0), « taille » à
/// 10 % de la longueur. Une normale par face (rendu facetté) ; chaque sommet
/// est `[position, normale]`.
pub(crate) fn octahedron() -> Vec<[f32; 6]> {
    let w = 0.1;
    let head = Vec3::ZERO;
    let tail = Vec3::Y;
    let ring = [
        Vec3::new(w, w, w),
        Vec3::new(w, w, -w),
        Vec3::new(-w, w, -w),
        Vec3::new(-w, w, w),
    ];
    let mut out = Vec::with_capacity(24);
    for i in 0..4 {
        let (a, b) = (ring[i], ring[(i + 1) % 4]);
        // Faces orientées vers l'extérieur (sens trigonométrique).
        for tri in [[head, b, a], [tail, a, b]] {
            let n = (tri[1] - tri[0]).cross(tri[2] - tri[0]).normalize();
            for p in tri {
                out.push([p.x, p.y, p.z, n.x, n.y, n.z]);
            }
        }
    }
    out
}

/// Matrice d'instance d'un os : sa transformation monde, mise à l'échelle de
/// sa longueur.
pub fn bone_matrix(world: Mat4, length: f32) -> Mat4 {
    world * Mat4::from_scale(Vec3::splat(length.max(1e-4)))
}

/// Matrice d'une poignée en losange centrée sur `p`, de taille `size`.
pub fn handle_matrix(p: Vec3, size: f32) -> Mat4 {
    Mat4::from_translation(p - Vec3::Y * size * 0.5) * Mat4::from_scale(Vec3::splat(size))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn octahedron_faces_point_outward() {
        let center = Vec3::new(0.0, 0.1, 0.0);
        let vertices = octahedron();
        assert_eq!(vertices.len(), 24);
        for tri in vertices.chunks(3) {
            let p = |v: &[f32; 6]| Vec3::new(v[0], v[1], v[2]);
            let n = Vec3::new(tri[0][3], tri[0][4], tri[0][5]);
            let c = (p(&tri[0]) + p(&tri[1]) + p(&tri[2])) / 3.0;
            assert!((c - center).dot(n) > 0.0, "face {c:?} {n:?}");
        }
    }

    #[test]
    fn handle_is_centered() {
        let m = handle_matrix(Vec3::new(1.0, 2.0, 3.0), 0.5);
        let mid = m.transform_point3(Vec3::new(0.0, 0.5, 0.0));
        assert!(mid.abs_diff_eq(Vec3::new(1.0, 2.0, 3.0), 1e-6));
    }
}
