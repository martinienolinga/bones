//! Gestes de corps communs à tous les générateurs : bassin, courbure d'une
//! chaîne, orientation de la tête, ondulation d'une queue.

use glam::{Quat, Vec3};

use super::ctx::Body;
use crate::BoneId;

/// Rotation canonique tangage (X), lacet (Y), roulis (Z).
pub(crate) fn euler(pitch: f32, yaw: f32, roll: f32) -> Quat {
    Quat::from_rotation_y(yaw) * Quat::from_rotation_z(roll) * Quat::from_rotation_x(pitch)
}

impl Body<'_, '_> {
    /// Bassin : déplacement canonique et rotations (lacet autour de Y,
    /// tangage autour de X — positif penche vers l'avant —, roulis autour de Z).
    pub fn pelvis(&mut self, root: BoneId, offset: Vec3, yaw: f32, pitch: f32, roll: f32) {
        self.shift(Some(root), offset);
        self.turn(Some(root), Vec3::X, pitch);
        self.turn(Some(root), Vec3::Z, roll);
        self.turn(Some(root), Vec3::Y, yaw);
    }

    /// Répartit une rotation canonique sur une chaîne (rotations relatives).
    pub fn spread(&mut self, bones: &[BoneId], pitch: f32, yaw: f32, roll: f32) {
        let n = bones.len().max(1) as f32;
        for b in bones {
            self.turn(Some(*b), Vec3::X, pitch / n);
            self.turn(Some(*b), Vec3::Z, roll / n);
            self.turn(Some(*b), Vec3::Y, yaw / n);
        }
    }

    /// Orientation monde de la tête relative au repère du corps (après la
    /// cinématique directe) ; le cou fait une partie du chemin.
    pub fn look(&mut self, neck: &[BoneId], head: Option<BoneId>, pitch: f32, yaw: f32, roll: f32) {
        let basis = self.rig.basis;
        let q = basis * euler(pitch, yaw, roll) * basis.inverse();
        for (i, b) in neck.iter().enumerate() {
            let want = q * self.rig.rest[b.index()].rotation;
            let t = (i + 1) as f32 / (neck.len() + 1) as f32 * 0.5;
            let r = self.rotation(*b).slerp(want, t);
            self.set_rotation(*b, r);
        }
        if let Some(h) = head {
            self.set_rotation(h, q * self.rig.rest[h.index()].rotation);
        }
    }

    /// Onde qui parcourt une chaîne (queue, tentacule) : chaque os tourne
    /// autour de l'axe canonique `axis`, avec un retard de phase croissant
    /// et une amplitude qui grandit vers le bout. `phase` en tours.
    pub fn ripple(&mut self, bones: &[BoneId], axis: Vec3, amplitude: f32, phase: f32, lag: f32) {
        let n = bones.len().max(1) as f32;
        for (i, b) in bones.iter().enumerate() {
            let s = (i as f32 + 1.0) / n;
            let a = amplitude * (0.5 + s) / n * 2.0;
            self.turn(Some(*b), axis, a * (std::f32::consts::TAU * (phase - lag * s)).sin());
        }
    }
}
