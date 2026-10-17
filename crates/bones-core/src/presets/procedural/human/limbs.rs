//! Gestes des membres humains : pied posé en IK (tangage, orteils à plat),
//! bras en cinématique directe ou en IK, épaules, mains et doigts.

use glam::{Quat, Vec3};

use super::rig::Human;
use crate::presets::procedural::ctx::Body;
use crate::presets::roles::Lr;

impl Human<'_, '_> {
    /// Rotation monde de repos.
    fn rest_rot(&self, b: crate::BoneId) -> Quat {
        self.rig.rest[b.index()].rotation
    }

    /// Transporte un vecteur de repos lié à l'os `b` dans sa pose actuelle.
    fn carried(&self, body: &Body, b: crate::BoneId, v: Vec3) -> Vec3 {
        body.rotation(b) * (self.rest_rot(b).inverse() * v)
    }

    /// Pose le pied : cheville en `ankle` (monde), tangage `pitch` (positif :
    /// pointe relevée), lacet `yaw` autour de la verticale, orteils à plat
    /// au sol selon `toe_flat` (0 : ils suivent le pied, 1 : à plat).
    pub fn place_foot(&self, body: &mut Body, side: Lr, ankle: Vec3, pitch: f32, yaw: f32, toe_flat: f32) {
        let leg = self.leg_of(side);
        let hip = body.head(leg.thigh);
        // Genou : sens de flexion de repos, tourné avec le bassin.
        let bend = self.carried(body, self.root, leg.bend);
        let pole = (hip + ankle) * 0.5 + bend * leg.length;
        body.two_bone(leg.thigh, leg.shin, ankle, pole);
        let rig = self.rig;
        let heading = Quat::from_axis_angle(rig.frame.up, yaw);
        let left = heading * rig.w(Vec3::X);
        let q = Quat::from_axis_angle(left, -pitch) * heading;
        if let Some(f) = leg.foot {
            body.set_rotation(f, q * self.rest_rot(f));
        }
        if let Some(t) = leg.toe {
            let follow = body.rotation(t);
            let flat = heading * self.rest_rot(t);
            body.set_rotation(t, follow.slerp(flat, toe_flat.clamp(0.0, 1.0)));
        }
    }

    /// Point du pied (cheville) quand il pivote d'un tangage `pitch` autour
    /// du talon (pointe relevée) ou de la plante (talon levé), le pied étant
    /// à plat avec sa cheville en `flat_ankle`.
    pub fn rolled_ankle(&self, side: Lr, flat_ankle: Vec3, pitch: f32, yaw: f32) -> Vec3 {
        let leg = self.leg_of(side);
        let rig = self.rig;
        let heading = Quat::from_axis_angle(rig.frame.up, yaw);
        let offset = flat_ankle - leg.ankle;
        let pivot = if pitch >= 0.0 { leg.heel } else { leg.ball };
        let pivot = offset + leg.ankle + heading * (pivot - leg.ankle);
        let left = heading * rig.w(Vec3::X);
        let q = Quat::from_axis_angle(left, -pitch);
        pivot + q * (flat_ankle - pivot)
    }

    /// Bras en cinématique directe : écarté de `abduct` sur le côté (radians,
    /// depuis la verticale), balancé de `swing` vers l'avant autour de l'axe
    /// des épaules, coude et poignet fléchis vers l'avant de l'articulation.
    /// Le bras est d'abord abaissé dans le plan frontal puis balancé : la
    /// torsion reste anatomique quelle que soit la pose de repos (T ou A).
    pub fn swing_arm(&self, body: &mut Body, side: Lr, abduct: f32, swing: f32, elbow: f32, wrist: f32) {
        let Some(arm) = self.arm_of(side) else { return };
        let rig = self.rig;
        let hang = Quat::from_rotation_z(side.sign() * abduct) * -Vec3::Y;
        body.aim(arm.upper, rig.w(hang));
        body.rotate(arm.upper, Quat::from_axis_angle(rig.w(Vec3::X), -swing));
        let up = (body.tail(arm.upper) - body.head(arm.upper)).normalize_or(-rig.frame.up);
        let flex = self.carried(body, arm.upper, arm.flex);
        let fore = up * elbow.cos() + flex * elbow.sin();
        body.aim(arm.fore, fore);
        if let Some(h) = arm.hand {
            let flex = self.carried(body, arm.fore, arm.flex);
            body.aim(h, fore * wrist.cos() + flex * wrist.sin());
        }
    }

    /// Main vers `target` (monde) par IK, coude vers `pole` ; la main
    /// prolonge l'avant-bras, fléchie de `wrist`.
    pub fn reach(&self, body: &mut Body, side: Lr, target: Vec3, pole: Vec3, wrist: f32) {
        let Some(arm) = self.arm_of(side) else { return };
        body.two_bone(arm.upper, arm.fore, target, pole);
        if let Some(h) = arm.hand {
            let fore = (body.tail(arm.fore) - body.head(arm.fore)).normalize_or(Vec3::Y);
            let flex = self.carried(body, arm.fore, arm.flex);
            body.aim(h, fore * wrist.cos() + flex * wrist.sin());
        }
    }

    /// Tourne la main autour de son axe pour que la paume regarde `normal`.
    pub fn palm_towards(&self, body: &mut Body, side: Lr, normal: Vec3) {
        let Some(arm) = self.arm_of(side) else { return };
        let Some(h) = arm.hand else { return };
        let axis = (body.tail(h) - body.head(h)).normalize_or(Vec3::Y);
        let flat = |v: Vec3| (v - axis * v.dot(axis)).try_normalize();
        let (Some(now), Some(want)) = (flat(self.carried(body, h, arm.palm)), flat(normal)) else {
            return;
        };
        let angle = now.cross(want).dot(axis).atan2(now.dot(want));
        // La torsion est partagée entre l'avant-bras et la main.
        body.twist(arm.fore, angle * 0.5);
        body.twist(h, angle * 0.5);
    }

    /// Épaule : élévation et avancée de la clavicule (radians).
    pub fn shrug(&self, body: &mut Body, side: Lr, raise: f32, forward: f32) {
        let Some(c) = self.arm_of(side).and_then(|a| a.clavicle) else { return };
        body.turn(Some(c), Vec3::Z, side.sign() * raise);
        body.turn(Some(c), Vec3::Y, -side.sign() * forward);
    }

    /// Flexion des doigts vers la paume (0 : ouverts, 1 : poing serré).
    /// À appeler avant la cinématique directe.
    pub fn curl(&self, body: &mut Body, side: Lr, amount: f32) {
        let Some(arm) = self.arm_of(side) else { return };
        for (i, finger) in arm.fingers.iter().enumerate() {
            let thumb = i == 0 && arm.fingers.len() == 5;
            let weight = if thumb { 0.45 } else { 1.0 };
            for (k, b) in finger.iter().enumerate() {
                let hinge = self.rig.dir(*b).cross(arm.palm).normalize_or(Vec3::X);
                let angle = amount * weight * if k == 0 { 1.2 } else { 1.5 };
                body.turn_world(*b, Quat::from_axis_angle(hinge, angle));
            }
        }
    }
}
