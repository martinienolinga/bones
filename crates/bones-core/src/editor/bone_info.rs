//! Informations et navigation dans la hiérarchie pour le panneau des os.

use glam::Vec3;

use super::Editor;
use crate::BoneId;

/// Résumé d'un os pour le panneau de propriétés (valeurs de repos).
#[derive(Clone, Debug, PartialEq)]
pub struct BoneInfo {
    pub id: BoneId,
    pub name: String,
    pub parent: Option<BoneId>,
    pub head: Vec3,
    pub tail: Vec3,
    pub length: f32,
    /// Roulis en radians.
    pub roll: f32,
    pub connected: bool,
    pub deform: bool,
    pub children: usize,
}

impl Editor {
    pub fn bone_info(&self, id: BoneId) -> BoneInfo {
        let s = &self.rig.skeleton;
        let bone = s.bone(id);
        let (head, tail) = s.rest_head_tail(id);
        BoneInfo {
            id,
            name: bone.name.clone(),
            parent: bone.parent(),
            head,
            tail,
            length: bone.length,
            roll: s.roll(id),
            connected: s.is_connected(id),
            deform: bone.deform,
            children: s.children(id).count(),
        }
    }

    /// Os pouvant devenir parent de `id` (ni lui ni ses descendants).
    pub fn parent_candidates(&self, id: BoneId) -> Vec<BoneId> {
        let s = &self.rig.skeleton;
        s.ids().filter(|b| !s.is_ancestor(id, *b)).collect()
    }

    // ------------------------------------------------------------------
    // Sélection

    pub fn select_parent(&mut self) {
        if let Some(p) = self.selected.and_then(|b| self.rig.skeleton.bone(b).parent()) {
            self.selected = Some(p);
        }
    }

    pub fn select_child(&mut self) {
        if let Some(c) = self.selected.and_then(|b| self.rig.skeleton.children(b).next()) {
            self.selected = Some(c);
        }
    }

    /// Frère suivant (circulaire) ; parmi les racines pour une racine.
    pub fn select_next_sibling(&mut self) {
        let Some(bone) = self.selected else { return };
        let s = &self.rig.skeleton;
        let parent = s.bone(bone).parent();
        let siblings: Vec<BoneId> = s.ids().filter(|b| s.bone(*b).parent() == parent).collect();
        if let Some(i) = siblings.iter().position(|b| *b == bone) {
            self.selected = Some(siblings[(i + 1) % siblings.len()]);
        }
    }
}
