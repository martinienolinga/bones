//! Opérations du document qui ajoutent des os ou changent leurs indices :
//! pose, animation, poids, membres IK/FK et épingles suivent toujours.

use std::sync::Arc;

use super::SkeletonError;
use super::document::next_revision;
use crate::{BoneId, BoneRemap, Pin, Rig};

impl Rig {
    /// Complète pose et clés d'animation pour les os ajoutés en fin de squelette.
    pub fn sync_bones(&mut self) {
        self.pose.sync_len(&self.skeleton);
        let n = self.skeleton.len();
        if self.clip.keys().iter().any(|k| k.pose.bones.len() != n) {
            self.touch_clip();
            Arc::make_mut(&mut self.clip).sync_len(&self.skeleton);
        }
    }

    /// Applique une correspondance d'indices (suppression, réordonnancement)
    /// à tout ce qui référence des os. Avec `keep_binding`, les poids
    /// remappés restent considérés comme liés (géométrie de repos inchangée).
    pub fn apply_remap(&mut self, remap: &BoneRemap, keep_binding: bool) {
        self.pose.remap(remap);
        if !self.clip.is_empty() {
            self.touch_clip();
            Arc::make_mut(&mut self.clip).remap(remap);
        }
        Arc::make_mut(&mut self.skin).remap(remap);
        self.revisions.skin = next_revision();
        self.limbs = self.limbs.iter().filter_map(|l| l.remap(remap)).collect();
        self.pins = self
            .pins
            .iter()
            .filter_map(|p| {
                Some(Pin {
                    bone: remap.get(p.bone)?,
                    ..*p
                })
            })
            .collect();
        let was_bound = !self.needs_bind();
        self.touch_skeleton();
        if keep_binding && was_bound {
            self.revisions.bound_to = (self.revisions.skeleton, self.revisions.mesh);
        }
    }

    /// Change le parent d'un os (voir `Skeleton::set_parent`).
    pub fn set_bone_parent(
        &mut self,
        id: BoneId,
        parent: Option<BoneId>,
        connect: bool,
    ) -> Result<BoneRemap, SkeletonError> {
        let remap = self.skeleton.set_parent(id, parent, connect)?;
        // Sans connexion, rien ne bouge : les poids restent valables.
        self.apply_remap(&remap, !connect);
        Ok(remap)
    }

    /// Supprime un os et sa descendance.
    pub fn remove_with_children(&mut self, id: BoneId) -> BoneRemap {
        let remap = self.skeleton.remove_with_children(id);
        self.apply_remap(&remap, true);
        remap
    }

    /// Copie un os (et sa descendance) ; retourne les copies.
    pub fn duplicate_bone(&mut self, id: BoneId, with_children: bool) -> Vec<BoneId> {
        let copies = self.skeleton.duplicate(id, with_children);
        self.sync_bones();
        self.touch_skeleton();
        copies
    }

    /// Coupe un os en `parts` morceaux ; retourne les morceaux (indices
    /// définitifs) et la correspondance des anciens indices.
    pub fn subdivide_bone(&mut self, id: BoneId, parts: usize) -> (Vec<BoneId>, BoneRemap) {
        let (pieces, remap) = self.skeleton.subdivide(id, parts);
        // Les morceaux ont été ajoutés en fin de squelette avant le
        // réordonnancement : pose et clés sont complétées puis remappées.
        self.sync_bones();
        self.apply_remap(&remap, false);
        (pieces, remap)
    }
}
