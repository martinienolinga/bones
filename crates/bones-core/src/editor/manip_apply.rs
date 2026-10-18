//! Geste sur le manipulateur : préparation (comment l'os suit), application
//! de chaque glissement, toujours à partir de l'état du début du geste
//! (delta de pose en mode Pose, os de repos en mode Édition, ou modèle
//! entier), puis fin du geste.

use glam::{Quat, Vec2};

use super::manipulate::{Follow, ManipGesture, Target};
use super::{Editor, Mode};
use crate::ik::{IkSettings, solve_fabrik};
use crate::manipulator::{Frame, ManipDelta, ManipDrag};
use crate::{BoneId, LimbMode, Transform, Viewport};

impl Editor {
    pub(super) fn start_gesture(&self, drag: ManipDrag, target: Target, viewport: Viewport) -> ManipGesture {
        let skeleton = &self.rig.skeleton;
        let mut gesture = ManipGesture {
            drag,
            target,
            start_pose: Transform::IDENTITY,
            start_head: drag.frame.origin,
            start_len: 0.0,
            start_rot: Quat::IDENTITY,
            follow: Follow::Delta,
            viewport,
        };
        if let Target::Object(id) = target {
            // Transformation monde de l'objet au début du geste.
            gesture.start_pose = self.object_display_world(id).unwrap_or_default();
            return gesture;
        }
        let Target::Bone(bone) = target else {
            return gesture;
        };
        gesture.start_pose = self.rig.pose.get(bone);
        gesture.start_len = skeleton.bone(bone).length;
        gesture.start_rot = skeleton.rest_world_rotation(bone);
        if self.mode == Mode::Pose && drag.part.is_translation() {
            // Main ou pied d'un membre IK : sa tête est la cible du membre,
            // qui la suit exactement.
            let limb = self
                .rig
                .limbs
                .iter()
                .position(|l| l.mode == LimbMode::Ik && l.end == Some(bone) && l.is_valid(skeleton));
            gesture.follow = match (limb, skeleton.bone(bone).parent()) {
                (Some(i), _) => Follow::Limb(i, self.rig.limbs[i].target),
                (None, Some(parent)) if skeleton.is_connected(bone) => {
                    Follow::Chain(skeleton.chain(parent, self.chain_len))
                }
                _ => Follow::Delta,
            };
        }
        gesture
    }

    /// Fin de geste. Un simple toucher sans effet ne crée pas d'étape
    /// d'annulation et sélectionne l'os éventuellement situé dessous.
    pub(super) fn manipulator_up(&mut self, gesture: &ManipGesture) {
        let tap = gesture.drag.travel() < 6.0 * self.pixels_per_point.max(0.25);
        if !self.drag_moved || (tap && gesture.drag.last.is_identity()) {
            self.drop_checkpoint(true);
            let (at, radius) = (gesture.drag.last_pos, 12.0 * self.pixels_per_point);
            if self.mode == Mode::Object {
                self.select_object(self.pick_object(at, &gesture.viewport));
            } else if let Some(bone) = self.pick(at, &gesture.viewport, radius) {
                self.selected = Some(bone);
            }
            return;
        }
        match (self.mode, gesture.target) {
            (Mode::Pose, _) => {
                self.snap_limbs();
                self.store_key_if_any();
            }
            // Les marqueurs d'auto-rig ne correspondent plus au modèle : ils
            // seront de nouveau estimés.
            (_, Target::Model) => self.markers = None,
            _ => {}
        }
    }

    pub(super) fn manipulator_move(&mut self, gesture: &mut ManipGesture, pos: Vec2) {
        let delta = gesture.drag.update(pos, self.manipulator.active_snap());
        match (gesture.target, self.mode) {
            (Target::Bone(bone), Mode::Pose) => self.manip_pose(bone, gesture, &delta),
            (Target::Bone(bone), _) => self.manip_rest(bone, gesture, &delta),
            (Target::Object(id), _) => self.manip_object(id, gesture, &delta),
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
