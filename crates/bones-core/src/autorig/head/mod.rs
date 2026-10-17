//! Région de la tête d'un modèle, pour le gros plan de l'étape « Face » de
//! l'auto-rig. Fonctionne sur n'importe quel personnage importé (taille,
//! position et proportions quelconques), avec des indices de moins en moins
//! précis :
//!
//! 1. marqueurs du visage et du corps (sommet de la tête, menton) ;
//! 2. os de la tête du squelette (`head.x`, `Head`, `mixamorig:Head`…) ;
//! 3. haut de la boîte englobante du maillage.
//!
//! L'indice donne un volume de recherche ; la boîte finale est celle des
//! sommets du maillage qui s'y trouvent (oreilles, cheveux, nez compris).

use glam::Vec3;

use super::Markers;
use super::face::{FaceMarkerKind, FaceMarkers};
use crate::{Mesh, Skeleton};

/// Nombre minimal de sommets pour se fier au maillage dans le volume.
const MIN_VERTICES: usize = 8;

/// Boîte englobante (min, max) de la tête, en espace monde (pose de repos).
pub fn head_bounds(
    mesh: &Mesh,
    skeleton: &Skeleton,
    markers: Option<&Markers>,
    face: Option<&FaceMarkers>,
) -> Option<(Vec3, Vec3)> {
    let volume = from_markers(mesh, markers, face)
        .or_else(|| from_skeleton(mesh, skeleton))
        .or_else(|| from_mesh_top(mesh))?;
    let (mut lo, mut hi) = vertices_in(mesh, volume).unwrap_or(volume);
    // Les marqueurs du visage (et leurs symétriques) restent toujours visibles.
    if let Some(face) = face {
        let axis = face.nose_tip.x;
        for p in FaceMarkerKind::ALL.iter().filter_map(|k| face.get(*k)) {
            let mirror = Vec3::new(2.0 * axis - p.x, p.y, p.z);
            (lo, hi) = (lo.min(p).min(mirror), hi.max(p).max(mirror));
        }
    }
    (lo.is_finite() && hi.is_finite() && (hi - lo).max_element() > 0.0).then_some((lo, hi))
}

/// Volume de recherche d'après les marqueurs : du menton au sommet du crâne,
/// sur une largeur proportionnelle à la hauteur de la tête (les mains, même
/// levées, restent en dehors).
fn from_markers(mesh: &Mesh, markers: Option<&Markers>, face: Option<&FaceMarkers>) -> Option<(Vec3, Vec3)> {
    let (cx, bottom, top) = match (markers, face) {
        (Some(m), Some(f)) => (f.nose_tip.x, m.chin.y.min(f.chin.y), m.head_top.y.max(f.brow.y)),
        (Some(m), None) => (m.head_top.x, m.chin.y, m.head_top.y),
        // Sans sommet du crâne : le front est à peu près aussi haut au-dessus
        // des sourcils que les sourcils au-dessus des yeux, deux fois.
        (None, Some(f)) => (
            f.nose_tip.x,
            f.chin.y,
            f.brow.y + 2.5 * (f.brow.y - f.eye.y).abs(),
        ),
        (None, None) => return None,
    };
    let h = top - bottom;
    if !(h.is_finite() && h > 1e-4) {
        return None;
    }
    let (z0, z1) = depth(mesh, h);
    let pad = 0.06 * h;
    Some((
        Vec3::new(cx - 0.75 * h, bottom - pad, z0),
        Vec3::new(cx + 0.75 * h, top + pad, z1),
    ))
}

/// Volume de recherche autour de l'os de la tête.
fn from_skeleton(mesh: &Mesh, skeleton: &Skeleton) -> Option<(Vec3, Vec3)> {
    let id = skeleton
        .find("head.x")
        .or_else(|| skeleton.ids().find(|id| is_head_name(&skeleton.bone(*id).name)))?;
    let (head, tail) = skeleton.rest_head_tail(id);
    // Os de tête parfois minuscule (simple pivot) : longueur minimale
    // relative à la taille du modèle.
    let model = mesh.bounds().map_or(0.0, |(lo, hi)| (hi - lo).max_element());
    let length = (tail - head).length().max(0.06 * model);
    if !(length.is_finite() && length > 0.0) {
        return None;
    }
    // Largeur d'une tête ≈ ¾ de sa hauteur ; on descend peu sous l'os pour
    // ne pas prendre le cou et les épaules.
    let below = Vec3::new(0.5, 0.15, 0.5) * length;
    let above = Vec3::new(0.5, 0.25, 0.5) * length;
    Some((head.min(tail) - below, head.max(tail) + above))
}

/// Os de tête d'après son nom (`Head`, `mixamorig:Head`, `head_jnt`…), en
/// excluant les extrémités (`HeadTop_End`, `head_end`, `head_nub`).
fn is_head_name(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("head") && !["end", "top", "nub", "tip"].iter().any(|w| n.contains(w))
}

/// Dernier recours : le haut du maillage (environ un huitième de sa hauteur),
/// centré sur le sommet le plus haut.
fn from_mesh_top(mesh: &Mesh) -> Option<(Vec3, Vec3)> {
    let (lo, hi) = mesh.bounds()?;
    let h = hi.y - lo.y;
    if !(h.is_finite() && h > 1e-4) {
        return None;
    }
    let crown = mesh.positions.iter().max_by(|a, b| a.y.total_cmp(&b.y))?;
    let half = 0.1 * h;
    Some((
        Vec3::new(crown.x - half, hi.y - 0.14 * h, lo.z),
        Vec3::new(crown.x + half, hi.y, hi.z),
    ))
}

/// Profondeur de recherche : tout le maillage (un nez ou une queue de cheval
/// peut dépasser), ou à défaut une profondeur proportionnelle à la tête.
fn depth(mesh: &Mesh, h: f32) -> (f32, f32) {
    mesh.bounds().map_or((-h, h), |(lo, hi)| (lo.z, hi.z))
}

/// Boîte des sommets du maillage contenus dans le volume, s'il y en a assez.
fn vertices_in(mesh: &Mesh, (lo, hi): (Vec3, Vec3)) -> Option<(Vec3, Vec3)> {
    let (count, min, max) = mesh
        .positions
        .iter()
        .filter(|p| p.cmpge(lo).all() && p.cmple(hi).all())
        .fold((0usize, Vec3::MAX, Vec3::MIN), |(n, a, b), p| {
            (n + 1, a.min(*p), b.max(*p))
        });
    (count >= MIN_VERTICES).then_some((min, max))
}

#[cfg(test)]
mod tests;
