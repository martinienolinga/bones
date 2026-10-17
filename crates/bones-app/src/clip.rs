//! Découpage des segments au champ de la caméra (Liang-Barsky en
//! coordonnées homogènes). Les lignes envoyées au GPU restent ainsi dans
//! l'écran : certains rastériseurs gèrent mal les extrémités très lointaines
//! ou proches de l'œil, et on évite du travail inutile.

use glam::{Mat4, Vec3, Vec4};

/// Marge autour de l'écran pour que les lignes atteignent bien les bords.
const MARGIN: f32 = 1.02;

/// Partie visible du segment `[a, b]`, ou `None` s'il est hors champ.
pub fn clip_segment(view_proj: &Mat4, a: Vec3, b: Vec3) -> Option<(Vec3, Vec3)> {
    let pa = *view_proj * a.extend(1.0);
    let pb = *view_proj * b.extend(1.0);
    // Plans du volume de vue : chaque test doit être ≥ 0 (Z de 0 à w, wgpu).
    let planes = |p: Vec4| {
        [
            MARGIN * p.w + p.x,
            MARGIN * p.w - p.x,
            MARGIN * p.w + p.y,
            MARGIN * p.w - p.y,
            p.z,
            p.w - p.z,
        ]
    };
    let (fa, fb) = (planes(pa), planes(pb));
    let (mut t0, mut t1) = (0.0f32, 1.0f32);
    for (da, db) in fa.into_iter().zip(fb) {
        match (da < 0.0, db < 0.0) {
            (true, true) => return None,
            (true, false) => t0 = t0.max(da / (da - db)),
            (false, true) => t1 = t1.min(da / (da - db)),
            (false, false) => {}
        }
        if t0 > t1 {
            return None;
        }
    }
    Some((a.lerp(b, t0), a.lerp(b, t1)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bones_core::{OrbitCamera, Viewport};

    #[test]
    fn keeps_visible_part_only() {
        let cam = OrbitCamera::default();
        let vp = Viewport::new(800.0, 600.0);
        let m = cam.view_proj(&vp);
        // Segment entièrement visible : inchangé.
        let (a, b) = (cam.target, cam.target + Vec3::X * 0.1);
        assert_eq!(clip_segment(&m, a, b), Some((a, b)));
        // Segment traversant l'œil : coupé, les extrémités restent visibles.
        let eye = cam.eye();
        let (c, d) = clip_segment(&m, cam.target, eye + (eye - cam.target)).unwrap();
        assert_eq!(c, cam.target);
        let s = cam.project(d, &vp).unwrap();
        assert!(s.x.abs() < 1000.0 && s.y.abs() < 1000.0);
        // Segment derrière la caméra : rejeté.
        let behind = eye + (eye - cam.target);
        assert!(clip_segment(&m, behind, behind + Vec3::X).is_none());
    }
}
