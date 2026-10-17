//! Postures debout communes aux actions humaines : pieds posés (décalés,
//! tournés, talons levés), bras relâchés, garde de combat.

use glam::Vec3;

use super::rig::Human;
use crate::presets::procedural::ctx::Body;
use crate::presets::roles::Lr;

/// Pied posé : décalage canonique depuis la place de repos, lacet et
/// tangage (négatif : talon levé, positif : pointe levée).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Foot {
    pub offset: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    /// Hauteur du pied au-dessus du sol (pied levé), en mètres.
    pub lift: f32,
}

impl Foot {
    pub fn at(x: f32, z: f32) -> Self {
        Self {
            offset: Vec3::new(x, 0.0, z),
            ..Self::default()
        }
    }
}

impl Human<'_, '_> {
    /// Pose les deux pieds (IK) ; les orteils restent à plat quand le talon est levé.
    pub fn stand(&self, body: &mut Body, feet: [Foot; 2]) {
        for (side, foot) in Lr::BOTH.into_iter().zip(feet) {
            let leg = self.leg_of(side);
            let flat = leg.ankle + self.rig.w(foot.offset);
            let ankle = self.rolled_ankle(side, flat, foot.pitch, foot.yaw) + self.rig.frame.up * foot.lift;
            let toe = if foot.pitch < 0.0 && foot.lift < 1e-3 {
                1.0
            } else {
                0.0
            };
            self.place_foot(body, side, ankle, foot.pitch, foot.yaw, toe);
        }
    }

    /// Bras relâchés le long du corps, avec un balancement avant/arrière.
    pub fn relax_arms(&self, body: &mut Body, swing: [f32; 2], abduct: f32, elbow: f32) {
        for (side, s) in Lr::BOTH.into_iter().zip(swing) {
            self.swing_arm(body, side, abduct, s, elbow + 0.3 * s.max(0.0), 0.1);
        }
    }

    /// Épaule (tête du bras) en pose.
    pub fn shoulder(&self, body: &Body, side: Lr) -> Vec3 {
        self.arm_of(side)
            .map_or(body.head(self.root), |a| body.head(a.upper))
    }

    /// Point canonique relatif à l'épaule, tourné avec la poitrine.
    pub fn point_from_shoulder(&self, body: &Body, side: Lr, offset: Vec3) -> Vec3 {
        let chest = self.spine.last().copied().unwrap_or(self.root);
        let turn = body.rotation(chest) * self.rig.rest[chest.index()].rotation.inverse();
        self.shoulder(body, side) + turn * self.rig.w(offset)
    }

    /// Main vers un point relatif à l'épaule (canonique, suit la poitrine),
    /// coude vers un pôle relatif lui aussi.
    pub fn reach_from_shoulder(&self, body: &mut Body, side: Lr, hand: Vec3, elbow: Vec3, wrist: f32) {
        let target = self.point_from_shoulder(body, side, hand);
        let pole = self.point_from_shoulder(body, side, elbow);
        self.reach(body, side, target, pole, wrist);
    }

    /// Garde de boxe : poing devant le menton et coude bas, en décalages
    /// canoniques depuis l'épaule (main, pôle du coude).
    pub fn guard_offsets(&self, side: Lr) -> (Vec3, Vec3) {
        let s = side.sign();
        let arm = self.arm_of(side).map_or(0.6, |a| a.length);
        (
            Vec3::new(-s * 0.12, 0.12, 0.42) * arm / 0.6,
            Vec3::new(s * 0.4, -0.8, 0.05),
        )
    }

    /// Garde, ou mélange vers une autre position de main (`toward`, poids `w`).
    pub fn guard(&self, body: &mut Body, side: Lr, toward: Vec3, w: f32) {
        let (hand, elbow) = self.guard_offsets(side);
        self.reach_from_shoulder(body, side, hand.lerp(toward, w), elbow, -0.2 * (1.0 - w));
    }
}
