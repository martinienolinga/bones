//! Application d'un glissement du manipulateur, toujours à partir de l'état
//! du début du geste : delta de pose (mode Pose), os de repos (mode
//! Édition) ou modèle entier.

use glam::Vec2;

use super::manipulate::{Follow, ManipGesture, Target};
use super::{Editor, Mode};
use crate::ik::{IkSettings, solve_fabrik};
use crate::manipulator::{Frame, ManipDelta};
use crate::{BoneId, Transform};

impl Editor {
    pub(super) fn manipulator_move(&mut self, gesture: &mut ManipGesture, pos: Vec2) {
        let delta = gesture.drag.update(pos, self.manipulator.active_snap());
        match (gesture.target, self.mode) {
            (Target::Bone(bone), Mode::Pose) => self.manip_pose(bone, gesture, &delta),
            (Target::Bone(bone), _) => self.manip_rest(bone, gesture, &delta),
            (Target::Model, _) => {
                let m = model_transform(&gesture.drag.frame, &delta);
                if let Some(start) = self.undo.last() {
                    self.rig.transform_model_from(start, &m);
                }
            }
        }
    }

    /// Mode Pose : rotation autour de la tête, échelle uniforme du delta,
    /// translation selon la façon dont l'os suit (voir [`Follow`]).
    fn manip_pose(&mut self, bone: BoneId, g: &ManipGesture, delta: &ManipDelta) {
        self.rig.pose.set(bone, g.start_pose);
        let skeleton = &self.rig.skeleton;
        let parent = skeleton
            .bone(bone)
            .parent()
            .map(|p| self.rig.pose.world(skeleton)[p.index()])
            .unwrap_or(Transform::IDENTITY);
        let part = g.drag.part;
        if part.is_rotation() {
            self.rig.pose.rotate_world(skeleton, bone, &parent, delta.rotate);
            let limb = self.rig.limbs.iter().position(|l| l.contains(bone));
            self.hold_constraints(&[bone], limb);
            return;
        }
        if part.is_scale() {
            let mut t = g.start_pose;
            t.scale *= delta.scale;
            self.rig.pose.set(bone, t);
            self.hold_constraints(&[bone], None);
            return;
        }
        match &g.follow {
            Follow::Delta => {
                let rest = skeleton.bone(bone).rest;
                let basis = parent.rotation * rest.rotation;
                let mut t = g.start_pose;
                t.translation += basis.inverse() * delta.translate / (parent.scale * rest.scale);
                self.rig.pose.set(bone, t);
                self.hold_constraints(&[bone], None);
            }
            Follow::Limb(i, target) => {
                let limb = &mut self.rig.limbs[*i];
                limb.target = *target + delta.translate;
                limb.solve(skeleton, &mut self.rig.pose);
                let locked = [limb.upper, limb.lower];
                self.hold_constraints(&locked, Some(*i));
            }
            Follow::Chain(chain) => {
                let target = g.start_head + delta.translate;
                solve_fabrik(skeleton, &mut self.rig.pose, chain, target, IkSettings::default());
                self.hold_constraints(chain, None);
            }
        }
    }

    /// Mode Édition : déplacement de l'os entier, rotation autour de sa
    /// tête (roulis compris) ou longueur ; édition symétrique respectée.
    fn manip_rest(&mut self, bone: BoneId, g: &ManipGesture, delta: &ManipDelta) {
        let part = g.drag.part;
        let skeleton = &mut self.rig.skeleton;
        if part.is_rotation() {
            let current = skeleton.rest_world_rotation(bone);
            let wanted = delta.rotate * g.start_rot;
            skeleton.rotate_bone_world(bone, (wanted * current.inverse()).normalize());
        } else if part.is_scale() {
            skeleton.set_length(bone, g.start_len * delta.scale.max_element());
        } else {
            let head = skeleton.rest_head_tail(bone).0;
            skeleton.translate_world(bone, g.start_head + delta.translate - head);
        }
        self.after_bone_edit(bone);
    }
}

/// Transformation monde du modèle : échelle et rotation autour du pivot,
/// puis translation (`p ↦ o + t + R·S·(p − o)`). L'échelle suit les axes du
/// monde (le repère du modèle n'est jamais tourné).
pub(super) fn model_transform(frame: &Frame, delta: &ManipDelta) -> Transform {
    let o = frame.origin;
    Transform {
        translation: o + delta.translate - delta.rotate * (delta.scale * o),
        rotation: delta.rotate,
        scale: delta.scale,
    }
}
