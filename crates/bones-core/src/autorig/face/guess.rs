//! Estimation des marqueurs du visage sur le maillage : profil (nez, lèvres,
//! menton), globes oculaires (pièces séparées, sinon proportions), oreilles,
//! dents et langue (pièces séparées dans la bouche).

use glam::Vec3;

use super::FaceMarkers;
use super::profile::{Landmarks, Profile};
use crate::autorig::fit::surface::Surface;
use crate::{Markers, Mesh};

impl FaceMarkers {
    /// Estimation sur la tête du maillage (vu de face, regard vers +Z),
    /// d'après les marqueurs du corps. `None` si la tête n'a pas de visage
    /// reconnaissable (pas de nez saillant) : aucun os du visage n'est alors
    /// créé.
    pub fn guess(mesh: &Mesh, markers: &Markers) -> Option<FaceMarkers> {
        guess_on(&Surface::new(mesh), markers)
    }
}

pub(crate) fn guess_on(s: &Surface, m: &Markers) -> Option<FaceMarkers> {
    let cx = m.root.x;
    let (neck, top) = (m.neck.y, m.head_top.y);
    let hh = top - neck;
    if !hh.is_finite() || hh <= 1e-3 {
        return None;
    }
    let lm = Profile::new(s, cx, neck, top, hh / 400.0).landmarks(neck, top)?;
    let temple = half_width(s, cx, lm.brow.y + 0.05 * hh);
    let components = s.components();
    let (eye, eye_radius) = find_eye(s, &components, cx, &lm, hh).unwrap_or_else(|| {
        let x = 0.42 * temple;
        (Vec3::new(cx + x, lm.nasion.y, lm.nasion.z - 0.55 * x), 0.36 * x)
    });
    let x = eye.x - cx;
    if x <= 0.0 {
        return None;
    }
    let skin = |px: f32, py: f32, z: f32| s.front_hit(px, py).unwrap_or(Vec3::new(px, py, z));
    let ear = find_ear(s, cx, temple, &lm, eye);
    let jaw = match ear {
        Some((_, root)) => Vec3::new(cx, root.y - 0.3 * x, root.z + 0.4 * x),
        None => Vec3::new(cx, lm.nose.y, eye.z - 2.5 * x),
    };
    let mut face = FaceMarkers {
        eye,
        eye_radius,
        brow: skin(eye.x, eye.y + 0.72 * x, lm.brow.z),
        nose_tip: Vec3::new(cx, lm.nose.y, lm.nose.z),
        lip_top: Vec3::new(cx, lm.lip_top.y, lm.lip_top.z),
        lip_bottom: Vec3::new(cx, lm.lip_bottom.y, lm.lip_bottom.z),
        mouth_corner: skin(cx + 0.74 * x, lm.slit.y, lm.slit.z - 0.3 * x),
        chin: Vec3::new(cx, lm.chin.y, lm.chin.z),
        jaw,
        ear: ear.map(|(tip, _)| tip),
        teeth_top: None,
        teeth_bottom: None,
        tongue: None,
    };
    mouth_parts(s, &components, cx, &mut face, lm.slit.y);
    plausible(&face, cx).then_some(face)
}

/// Demi-largeur de la tête à la hauteur `y` (tempes).
fn half_width(s: &Surface, cx: f32, y: f32) -> f32 {
    s.slice(Vec3::new(cx, y, 0.0), Vec3::Y)
        .iter()
        .flat_map(|(a, b)| [a.x, b.x])
        .map(|px| (px - cx).abs())
        .fold(0.0, f32::max)
}

/// Boîte englobante d'un ensemble de sommets.
fn bounds(s: &Surface, vertices: &[u32]) -> (Vec3, Vec3) {
    vertices.iter().fold((Vec3::MAX, Vec3::MIN), |(lo, hi), v| {
        let p = s.positions[*v as usize];
        (lo.min(p), hi.max(p))
    })
}

/// Globe oculaire gauche : pièce séparée, à peu près sphérique, à hauteur
/// des yeux. Centre de sa boîte (la cornée ne le décale pas) et rayon.
fn find_eye(s: &Surface, components: &[Vec<u32>], cx: f32, lm: &Landmarks, hh: f32) -> Option<(Vec3, f32)> {
    components
        .iter()
        .filter_map(|c| {
            let (lo, hi) = bounds(s, c);
            let (center, size) = ((lo + hi) * 0.5, hi - lo);
            let ok = (0.02 * hh..0.3 * hh).contains(&(center.x - cx))
                && (lm.slit.y..lm.brow.y + 0.05 * hh).contains(&center.y)
                && (0.03 * hh..0.2 * hh).contains(&size.max_element())
                && size.max_element() < 1.5 * size.min_element();
            ok.then_some((c.len(), center, (size.x + size.y + size.z) / 6.0))
        })
        .max_by_key(|(count, _, _)| *count)
        .map(|(_, center, radius)| (center, radius))
}

/// Oreille gauche : sommets qui dépassent nettement la largeur des tempes,
/// entre la bouche et les sourcils, en arrière des yeux. (pointe, racine)
fn find_ear(s: &Surface, cx: f32, temple: f32, lm: &Landmarks, eye: Vec3) -> Option<(Vec3, Vec3)> {
    let ear: Vec<Vec3> = s
        .vertices()
        .map(|v| s.positions[v as usize])
        .filter(|p| p.x - cx > 1.08 * temple && (lm.slit.y..lm.brow.y).contains(&p.y) && p.z < eye.z)
        .collect();
    if ear.len() < 8 {
        return None;
    }
    let tip = *ear.iter().max_by(|a, b| a.x.total_cmp(&b.x))?;
    let centroid = ear.iter().copied().sum::<Vec3>() / ear.len() as f32;
    Some((tip, Vec3::new(cx + temple * 0.95, centroid.y, centroid.z)))
}

/// Dents et langue : pièces séparées (autres que les yeux) dans la bouche.
fn mouth_parts(s: &Surface, components: &[Vec<u32>], cx: f32, face: &mut FaceMarkers, slit: f32) {
    let half = (face.mouth_corner.x - cx) * 1.5;
    let mut tongue: Option<(usize, Vec3, Vec3)> = None;
    for c in components {
        let (lo, hi) = bounds(s, c);
        let center = (lo + hi) * 0.5;
        let inside = (center.x - cx).abs() < half
            && (face.chin.y..face.nose_tip.y).contains(&center.y)
            && (face.jaw.z..face.lip_top.z).contains(&center.z)
            && hi.z < face.lip_top.z + face.eye_radius;
        if !inside || (center - face.eye).length() < 2.0 * face.eye_radius {
            continue;
        }
        let deep = hi.z - lo.z > 2.0 * face.eye_radius && hi.y - lo.y < hi.z - lo.z;
        if center.y >= slit {
            face.teeth_top = Some(Vec3::new(cx, center.y, hi.z));
        } else if deep && tongue.is_none_or(|t| c.len() > t.0) {
            let back = Vec3::new(cx, center.y, lo.z + (hi.z - lo.z) * 0.15);
            tongue = Some((c.len(), back, Vec3::new(cx, center.y, hi.z)));
        } else {
            face.teeth_bottom = Some(Vec3::new(cx, center.y, hi.z));
        }
    }
    face.tongue = tongue.map(|(_, base, tip)| [base, tip]);
}

/// Cohérence d'ensemble : visage dans le bon ordre de haut en bas.
fn plausible(f: &FaceMarkers, cx: f32) -> bool {
    f.is_valid()
        && f.brow.y > f.eye.y
        && f.eye.y > f.nose_tip.y
        && f.nose_tip.y > f.lip_top.y
        && f.lip_top.y > f.lip_bottom.y
        && f.lip_bottom.y > f.chin.y
        && f.jaw.y > f.chin.y
        && f.jaw.z < f.chin.z
        && f.mouth_corner.x > cx
}
