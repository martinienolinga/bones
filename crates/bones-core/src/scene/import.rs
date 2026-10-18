//! Ajout au document d'objets lus dans un fichier d'échange
//! (`io::SceneImport`) : hiérarchie, attaches aux os et poids.

use std::sync::Arc;

use super::ops::sanitize;
use super::{ObjectId, Parent, SkinMethod};
use crate::io::{ImportedObject, ImportedParent};
use crate::{BoneId, Rig, Skin, Transform, VertexWeights};

impl Rig {
    /// Ajoute les objets importés (sauf l'indice `skip`, devenu le corps).
    /// `bone_map` traduit un os du fichier en os du document ; avec
    /// `same_skeleton`, un objet attaché à un os garde sa transformation
    /// relative à l'os, sinon il garde sa place en monde dans la pose
    /// affichée `bone_world`. Retourne les objets créés.
    pub(crate) fn add_imported(
        &mut self,
        objects: Vec<ImportedObject>,
        skip: Option<usize>,
        bone_map: impl Fn(BoneId) -> Option<BoneId>,
        same_skeleton: bool,
        bone_world: &[Transform],
    ) -> Vec<ObjectId> {
        let mut ids: Vec<Option<ObjectId>> = vec![None; objects.len()];
        let mut created = Vec::new();
        for (i, o) in objects.into_iter().enumerate() {
            if Some(i) == skip || o.mesh.triangle_count() == 0 {
                continue;
            }
            let (parent, local) = match o.parent {
                ImportedParent::World => (Parent::World, o.local),
                ImportedParent::Object(j) => match ids.get(j).copied().flatten().filter(|_| j < i) {
                    Some(id) => (Parent::Object(id), o.local),
                    None => (Parent::World, o.world),
                },
                ImportedParent::Joint(b) => match bone_map(b).filter(|b| self.skeleton.contains(*b)) {
                    Some(nb) if same_skeleton => (Parent::Bone(nb), o.local),
                    Some(nb) => {
                        let bone = bone_world.get(nb.index()).copied().unwrap_or_default();
                        (Parent::Bone(nb), bone.inverse().mul(&o.world))
                    }
                    None => (Parent::World, o.world),
                },
            };
            let count = o.mesh.vertex_count();
            let Ok(id) = self.add_object(&o.name, Arc::new(o.mesh), parent, sanitize(local)) else {
                continue;
            };
            ids[i] = Some(id);
            created.push(id);
            let Some(weights) = o
                .weights
                .filter(|w| w.len() == count && !self.skeleton.is_empty())
            else {
                continue;
            };
            match map_weights(&weights, &bone_map, self.skeleton.len()) {
                Some(mapped) => {
                    let _ = self.set_object_weights(id, Skin { weights: mapped });
                }
                // Os introuvables : poids recalculés (transfert depuis le corps).
                None => {
                    let _ = self.set_object_binding(id, Some(SkinMethod::Transfer));
                }
            }
        }
        created
    }
}

/// Poids traduits vers les os du document ; `None` si un os manque.
fn map_weights(
    weights: &[VertexWeights],
    bone_map: &impl Fn(BoneId) -> Option<BoneId>,
    bones: usize,
) -> Option<Vec<VertexWeights>> {
    weights
        .iter()
        .map(|w| {
            let influences = w
                .iter()
                .map(|(b, x)| Some((bone_map(b).filter(|b| b.index() < bones)?, x)))
                .collect::<Option<Vec<_>>>()?;
            Some(VertexWeights::from_influences(influences))
        })
        .collect()
}
