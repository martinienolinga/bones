//! Opérations sur le squelette : édition, auto-rig et liaison en tâche de fond.

use std::sync::Arc;

use super::{Editor, Mode};
use crate::autorig;
use crate::rig::mirror;
use crate::{BindMethod, Markers, Mesh, Side, Skeleton, Skin};

/// Calcul de poids à lancer en tâche de fond.
pub struct BindJob {
    pub mesh: Arc<Mesh>,
    pub skeleton: Skeleton,
    pub method: BindMethod,
    pub key: (u64, u64),
}

impl BindJob {
    pub fn run(self) -> (Skin, (u64, u64)) {
        (self.method.compute(&self.mesh, &self.skeleton), self.key)
    }
}

impl Editor {
    pub fn delete_selected(&mut self) {
        let Some(bone) = self.selected else { return };
        self.checkpoint();
        self.rig.remove_bone(bone);
        self.selected = None;
    }

    pub fn rename_selected(&mut self, name: &str) {
        let Some(bone) = self.selected else { return };
        let name = name.trim();
        if name.is_empty() || self.rig.skeleton.bone(bone).name == name {
            return;
        }
        self.checkpoint();
        let unique = self.rig.skeleton.unique_name(name);
        self.rig.skeleton.bone_mut(bone).name = unique;
    }

    pub fn symmetrize(&mut self, from: Side) {
        self.checkpoint();
        let created = mirror::symmetrize(&mut self.rig.skeleton, from);
        self.rig.pose.sync_len(&self.rig.skeleton);
        if created > 0 && !self.rig.clip.is_empty() {
            let skeleton = self.rig.skeleton.clone();
            self.rig.clip_mut().sync_len(&skeleton);
        }
        self.rig.touch_skeleton();
        self.status = Some(format!("Mirrored: {created} bones created"));
    }

    pub fn guess_markers(&mut self) {
        self.markers = Markers::guess(&self.rig.mesh);
    }

    pub fn generate_rig(&mut self) {
        let Some(markers) = self.markers else { return };
        self.checkpoint();
        let (skeleton, limbs) = autorig::generate(&markers, &self.autorig_options);
        let count = skeleton.len();
        self.rig.set_skeleton(skeleton, limbs);
        self.selected = None;
        self.frame = 0.0;
        self.set_mode(Mode::Pose);
        self.status = Some(format!("Rig generated: {count} bones, binding…"));
    }

    /// Tâche de liaison à lancer, s'il y en a une de nouvelle.
    pub fn bind_job(&mut self) -> Option<BindJob> {
        if !self.rig.needs_bind() || self.drag.is_some() {
            return None;
        }
        let revs = self.rig.revisions();
        let key = (revs.skeleton, revs.mesh);
        if self.bind_requested == Some(key) {
            return None;
        }
        self.bind_requested = Some(key);
        Some(BindJob {
            mesh: self.rig.mesh.clone(),
            skeleton: self.rig.skeleton.clone(),
            method: self.bind_method,
            key,
        })
    }

    pub fn finish_bind(&mut self, skin: Skin, key: (u64, u64)) {
        if self.rig.apply_skin(skin, key) {
            self.status = Some("Weights ready".into());
        }
    }

    /// Force un nouveau calcul des poids (changement de méthode).
    pub fn request_rebind(&mut self) {
        self.rig.touch_skeleton();
        self.bind_requested = None;
    }
}
