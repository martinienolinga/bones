//! Ajustement de la disposition au volume du maillage : les articulations
//! placées d'après les marqueurs (proportions génériques) sont recalées sur
//! le modèle réel.
//!
//! Outils : coupes planes du maillage (contours fermés et leur centre) et
//! distance géodésique sur la surface (branches des doigts). Chaque partie
//! est indépendante : une partie non reconnue, ou dont le résultat n'est pas
//! plausible (garde-fous de `sanity`), garde sa position générique.
//! Le personnage est vu de face (regard vers +Z), en pose en T ou en A.

mod arm;
mod branches;
mod hand;
mod leg;
mod probe;
mod sanity;
pub(super) mod surface;
#[cfg(test)]
mod tests;

use glam::Vec3;

use super::layout::generic_fingers;
use crate::Mesh;
use crate::autorig::Layout;

/// Recale `layout` sur `mesh` : colonne et tête dans l'épaisseur du tronc,
/// épaule, coude et poignet suivis dans le bras, doigts détectés sur la
/// main, hanche dans l'axe de la cuisse, genou, cheville et pied.
pub fn fit_layout(layout: &mut Layout, mesh: &Mesh) {
    let Some((lo, _)) = mesh.bounds() else { return };
    if mesh.triangle_count() == 0 {
        return;
    }
    let generic = layout.clone();
    let surface = surface::Surface::new(mesh);

    arm::fit_torso(layout, &surface);
    if !sanity::torso_ok(layout, &generic) {
        layout.restore_torso(&generic);
    }

    if let Some(tip) = arm::fit_arm(layout, &surface) {
        let axis = (layout.wrist - layout.elbow).normalize_or(Vec3::X);
        if !hand::fit_hand(layout, &surface, axis) || !sanity::hand_ok(layout) {
            // Doigts génériques, alignés sur la main réelle.
            let dir = (tip - layout.wrist).normalize_or(axis);
            let hand_len = tip.distance(layout.wrist);
            layout.fingers = generic_fingers(layout.wrist, dir, hand_len);
            layout.hand_tip = layout.wrist + dir * hand_len * 0.45;
        }
        if !sanity::arm_ok(layout, &generic) || !sanity::hand_ok(layout) {
            layout.restore_arm(&generic);
        }
    }

    if leg::fit_leg(layout, &surface, lo.y).is_some() {
        if !sanity::leg_ok(layout, &generic) {
            layout.restore_leg(&generic);
        } else if !sanity::foot_ok(layout) {
            layout.restore_foot(&generic);
        }
    }
}
