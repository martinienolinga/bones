//! Petit constructeur d'expressions, pour écrire la bibliothèque comme des
//! données lisibles (les actions faciales nommées sont dans `actions`).
//!
//! Les os de surface (sourcils, joues, nez, lèvres) sont très courts et
//! pivotent juste sous la peau : une rotation n'y déplace presque pas la
//! peau. Les actions les **décalent** donc ([`Expr::shift`], comme les
//! contrôleurs du visage d'Auto-Rig Pro) ; les rotations ne servent qu'aux
//! os à long bras de levier (mâchoire, paupières, langue, oreilles) et à
//! incliner légèrement la peau.
//!
//! Repère local des os de surface au repos (contrat [`crate::autorig::face`]) :
//! X local = +X monde, Y local = +Z monde (vers l'avant), Z local = −Y monde
//! (vers le bas). D'où les signes retenus :
//! - rotation autour de X local : `+` abaisse la pointe (peau vers le bas),
//!   `−` la relève ;
//! - rotation autour de Z local : `+` pousse la pointe vers −X monde (vers
//!   l'intérieur pour un os gauche), `−` vers +X monde (vers l'extérieur).
//!
//! Les paires `.l`/`.r` reçoivent le reflet du delta gauche (quaternion
//! `(w, x, −y, −z)`, translation X opposée), sauf les oreilles dont les bases
//! de repos sont déjà symétriques : elles reçoivent le même delta local.

use glam::{EulerRot, Quat, Vec3};

use super::data::{ExpressionData, compose};
use crate::Transform;
use crate::autorig::face::FaceBone;

/// Un millimètre (les décalages des actions sont écrits en millimètres).
const MM: f32 = 1e-3;

pub(super) struct Expr {
    bones: Vec<(String, Transform)>,
}

fn is_ear(b: FaceBone) -> bool {
    matches!(b, FaceBone::EarL | FaceBone::EarR)
}

impl Expr {
    pub fn new() -> Self {
        Self { bones: Vec::new() }
    }

    /// Cumule un delta sur un os (compose si l'os est déjà posé).
    fn put(&mut self, b: FaceBone, t: Transform) {
        match self.bones.iter_mut().find(|(n, _)| n == b.name()) {
            Some(e) => e.1 = compose(&e.1, &t),
            None => self.bones.push((b.name().to_string(), t)),
        }
    }

    pub(super) fn rot(&mut self, b: FaceBone, x: f32, y: f32, z: f32) {
        let q = Quat::from_euler(EulerRot::XYZ, x, y, z);
        self.put(b, Transform::from_rotation_translation(q, Vec3::ZERO));
    }

    /// Même rotation des deux côtés (reflet, sauf oreilles : delta identique).
    pub(super) fn pair_rot(&mut self, left: FaceBone, x: f32, y: f32, z: f32) {
        self.rot(left, x, y, z);
        if let Some(r) = left.mirrored() {
            let q = Quat::from_euler(EulerRot::XYZ, x, y, z);
            let q = if is_ear(left) {
                q
            } else {
                Quat::from_xyzw(q.x, -q.y, -q.z, q.w)
            };
            self.put(r, Transform::from_rotation_translation(q, Vec3::ZERO));
        }
    }

    /// Décale un os de surface de quelques millimètres (tête de référence ;
    /// [`super::apply::face_pose`] met à l'échelle du visage), dans son repère
    /// local de repos : `side` vers +X monde, `fwd` vers l'avant, `up` vers le
    /// haut. Ajouté directement à la translation, quel que soit l'ordre des
    /// rotations.
    pub(super) fn shift(&mut self, b: FaceBone, side: f32, fwd: f32, up: f32) {
        let t = Vec3::new(side, fwd, -up) * MM;
        match self.bones.iter_mut().find(|(n, _)| n == b.name()) {
            Some(e) => e.1.translation += t,
            None => self
                .bones
                .push((b.name().to_string(), Transform::from_translation(t))),
        }
    }

    /// Même décalage des deux côtés ; `out` éloigne de l'axe du visage.
    pub(super) fn pair_shift(&mut self, left: FaceBone, out: f32, fwd: f32, up: f32) {
        self.shift(left, out, fwd, up);
        if let Some(r) = left.mirrored() {
            self.shift(r, -out, fwd, up);
        }
    }

    pub fn finish(self) -> ExpressionData {
        ExpressionData { bones: self.bones }
    }
}
