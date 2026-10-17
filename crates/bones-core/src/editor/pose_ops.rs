//! Opérations sur la pose : remise à zéro, miroir, épingles, IK/FK.

use super::Editor;
use crate::rig::mirror;
use crate::{LimbMode, Pose, Transform};

impl Editor {
    /// Si une clé existe à l'image courante, elle reçoit la pose modifiée.
    pub(super) fn store_key_if_any(&mut self) {
        let frame = self.current_frame();
        if self.rig.clip.key_at(frame).is_some() {
            let pose = self.rig.pose.clone();
            self.rig.clip_mut().set_key(frame, pose);
        }
    }

    pub fn reset_pose(&mut self, selected_only: bool) {
        self.checkpoint();
        match (selected_only, self.selected) {
            (true, Some(b)) => self.rig.pose.set(b, Transform::IDENTITY),
            _ => self.rig.pose = Pose::rest(&self.rig.skeleton),
        }
        self.snap_limbs();
        self.store_key_if_any();
    }

    pub fn mirror_pose(&mut self) {
        self.checkpoint();
        self.rig.pose = mirror::mirrored_pose(&self.rig.skeleton, &self.rig.pose);
        self.snap_limbs();
        self.store_key_if_any();
    }

    pub fn clear_pins(&mut self) {
        self.checkpoint();
        self.rig.pins.clear();
    }

    /// Bascule IK/FK d'un membre, avec recalage (snap) comme Auto-Rig Pro.
    pub fn toggle_limb_mode(&mut self, limb: usize) {
        self.checkpoint();
        self.snap_limbs();
        if let Some(l) = self.rig.limbs.get_mut(limb) {
            l.mode = match l.mode {
                LimbMode::Ik => LimbMode::Fk,
                LimbMode::Fk => LimbMode::Ik,
            };
        }
    }
}
