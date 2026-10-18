//! Les objets suivent les modifications du squelette : changement
//! d'indices des os (suppression, réordonnancement), remplacement complet
//! (auto-rig, retrouvés par le nom de l'os) et transformation du modèle.

use std::sync::Arc;

use super::ops::sanitize;
use super::{Binding, ObjectId, Parent};
use crate::{BoneId, BoneRemap, Rig, Transform};

/// Places en monde des objets attachés à un os, relevées avant une
/// modification du squelette.
#[derive(Clone, Debug, Default)]
pub(crate) struct ObjectAnchors(Vec<(ObjectId, Transform)>);

impl ObjectAnchors {
    fn get(&self, id: ObjectId) -> Option<Transform> {
        self.0.iter().find(|(o, _)| *o == id).map(|(_, t)| *t)
    }
}

impl Rig {
    fn anchors_with(&self, bone_world: impl FnOnce() -> Vec<Transform>) -> ObjectAnchors {
        if !self.objects.iter().any(|o| matches!(o.parent, Parent::Bone(_))) {
            return ObjectAnchors::default();
        }
        let bones = bone_world();
        ObjectAnchors(
            self.objects
                .iter()
                .filter(|o| matches!(o.parent, Parent::Bone(_)))
                .filter_map(|o| Some((o.id, self.object_world(o.id, &bones)?)))
                .collect(),
        )
    }

    /// Places posées des objets attachés à un os.
    pub(crate) fn object_anchors(&self) -> ObjectAnchors {
        self.anchors_with(|| self.world())
    }

    /// Places de repos des objets attachés à un os.
    pub(crate) fn object_rest_anchors(&self) -> ObjectAnchors {
        self.anchors_with(|| self.skeleton.rest_world())
    }

    /// Applique une correspondance d'indices d'os aux objets : parents et
    /// poids. Un objet dont l'os disparaît passe à l'os qui hérite des
    /// influences (ou au monde) en gardant sa place notée dans `anchors`.
    pub(crate) fn remap_objects(&mut self, remap: &BoneRemap, anchors: &ObjectAnchors) {
        let mut orphans = Vec::new();
        for o in &mut self.objects {
            if let Parent::Bone(b) = o.parent {
                o.parent = match remap.get(b) {
                    Some(new) => Parent::Bone(new),
                    None => {
                        orphans.push(o.id);
                        remap.removed_parent.map_or(Parent::World, Parent::Bone)
                    }
                };
            }
            if let Binding::Skinned(s) = &mut o.binding {
                Arc::make_mut(&mut s.skin).remap(remap);
                o.revisions.skin = crate::next_revision();
            }
        }
        self.keep_places(&orphans, anchors, false);
    }

    /// Après le remplacement du squelette : chaque objet attaché à un os
    /// retrouve l'os de même nom (sinon le monde) en gardant sa place de
    /// repos ; les poids suivent les noms des os.
    pub(crate) fn retarget_objects(&mut self, old_names: &[String], anchors: &ObjectAnchors) {
        if self.objects.is_empty() {
            return;
        }
        let map: Vec<Option<BoneId>> = old_names.iter().map(|n| self.skeleton.find(n)).collect();
        let remap = BoneRemap {
            map,
            removed_parent: None,
        };
        let mut moved = Vec::new();
        for o in &mut self.objects {
            if let Parent::Bone(b) = o.parent {
                o.parent = remap.get(b).map_or(Parent::World, Parent::Bone);
                moved.push(o.id);
            }
            if let Binding::Skinned(s) = &mut o.binding {
                Arc::make_mut(&mut s.skin).remap(&remap);
                o.revisions.skin = crate::next_revision();
            }
        }
        self.keep_places(&moved, anchors, true);
    }

    /// Recalcule la transformation locale des objets `ids` pour qu'ils
    /// retrouvent leur place notée (pose courante, ou repos si `rest`).
    fn keep_places(&mut self, ids: &[ObjectId], anchors: &ObjectAnchors, rest: bool) {
        if ids.is_empty() {
            return;
        }
        let bones = if rest {
            self.skeleton.rest_world()
        } else {
            self.world()
        };
        for &id in ids {
            let Some(world) = anchors.get(id) else { continue };
            let Some(parent) = self.object(id).map(|o| o.parent) else {
                continue;
            };
            let local = self.parent_world(parent, &bones).inverse().mul(&world);
            if let Ok(o) = self.object_mut(id) {
                o.local = sanitize(local);
            }
        }
    }

    /// Transformation du modèle entier (voir `Rig::transform_model_from`) :
    /// les objets posés dans le monde suivent `m` ; ceux attachés à un os
    /// suivent déjà l'os, seule une échelle uniforme leur est appliquée.
    pub(crate) fn transform_objects_from(&mut self, start: &Rig, m: &Transform) {
        self.objects.clone_from(&start.objects);
        let uniform = (m.scale.x - m.scale.y).abs() < 1e-6 && (m.scale.x - m.scale.z).abs() < 1e-6;
        for o in &mut self.objects {
            match o.parent {
                Parent::World => o.local = sanitize(m.mul(&o.local)),
                Parent::Bone(_) if uniform && m.scale.x != 1.0 => {
                    o.local.translation *= m.scale.x;
                    o.local.scale *= m.scale.x;
                }
                _ => {}
            }
        }
    }
}
