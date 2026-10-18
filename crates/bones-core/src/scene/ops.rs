//! Opérations du document sur les objets : création, copie, suppression,
//! parent (avec « Keep Transform »), ordre, transformation et liaison.
//! L'historique d'annulation est géré par l'éditeur.

use std::sync::Arc;

use super::{Binding, ObjectError, ObjectId, Parent, SceneObject, SkinBinding, SkinMethod, unique_name};
use crate::{Mesh, Rig, Transform};

/// Échelle minimale (en valeur absolue) d'un axe : une échelle nulle rendrait
/// la transformation non inversible.
const MIN_SCALE: f32 = 1e-4;

/// Transformation ramenée à des valeurs sûres (finies, échelle non nulle).
pub(crate) fn sanitize(t: Transform) -> Transform {
    let finite = |v: glam::Vec3, d: glam::Vec3| if v.is_finite() { v } else { d };
    let rotation = if t.rotation.is_finite() && t.rotation.length_squared() > 1e-12 {
        t.rotation.normalize()
    } else {
        glam::Quat::IDENTITY
    };
    let scale = finite(t.scale, glam::Vec3::ONE).to_array().map(|s| {
        if s.abs() < MIN_SCALE {
            MIN_SCALE.copysign(s)
        } else {
            s
        }
    });
    Transform {
        translation: finite(t.translation, glam::Vec3::ZERO),
        rotation,
        scale: glam::Vec3::from_array(scale),
    }
}

impl Rig {
    /// Nom d'objet libre (les noms sont uniques, comme dans Blender).
    pub fn unique_object_name(&self, base: &str) -> String {
        unique_name(base, |n| self.objects.iter().any(|o| o.name == n))
    }

    /// Ajoute un objet rigide ; retourne son identifiant.
    pub fn add_object(
        &mut self,
        name: &str,
        mesh: Arc<Mesh>,
        parent: Parent,
        local: Transform,
    ) -> Result<ObjectId, ObjectError> {
        self.check_parent(None, parent)?;
        let id = self.next_object_id();
        let name = self.unique_object_name(name);
        self.objects
            .push(SceneObject::new(id, name, mesh, parent, sanitize(local)));
        Ok(id)
    }

    /// Supprime un objet ; ses enfants passent à son parent en gardant leur
    /// place en monde (`bone_world` : os tels qu'affichés).
    pub fn remove_object(
        &mut self,
        id: ObjectId,
        bone_world: &[crate::Transform],
    ) -> Result<(), ObjectError> {
        let index = self.object_index(id).ok_or(ObjectError::NotFound)?;
        let parent = self.objects[index].parent;
        let children: Vec<ObjectId> = self.object_children(Parent::Object(id)).collect();
        for child in children {
            self.set_object_parent(child, parent, true, bone_world)?;
        }
        self.objects.remove(index);
        Ok(())
    }

    /// Copie un objet (et ses descendants si demandé) au même endroit ; les
    /// maillages et poids sont partagés. La copie de `id` vient juste après
    /// lui dans la liste et est retournée en premier.
    pub fn duplicate_object(
        &mut self,
        id: ObjectId,
        with_children: bool,
    ) -> Result<Vec<ObjectId>, ObjectError> {
        let index = self.object_index(id).ok_or(ObjectError::NotFound)?;
        let sources = if with_children {
            self.object_subtree(id)
        } else {
            vec![id]
        };
        let mut pairs: Vec<(ObjectId, ObjectId)> = Vec::with_capacity(sources.len());
        let mut copies = Vec::with_capacity(sources.len());
        for source in &sources {
            let mut copy = self.object(*source).ok_or(ObjectError::NotFound)?.clone();
            copy.id = ObjectId(self.next_object_id().0.max(pairs.last().map_or(0, |p| p.1.0 + 1)));
            copy.name = unique_name(&copy.name, |n| {
                self.objects
                    .iter()
                    .chain(&copies)
                    .any(|o: &SceneObject| o.name == n)
            });
            if let Parent::Object(p) = copy.parent
                && let Some((_, new)) = pairs.iter().find(|(old, _)| *old == p)
            {
                copy.parent = Parent::Object(*new);
            }
            // Nouvelles révisions : chaque objet a ses propres tampons GPU.
            copy.revisions = Default::default();
            pairs.push((*source, copy.id));
            copies.push(copy);
        }
        let ids = copies.iter().map(|c| c.id).collect();
        self.objects.splice(index + 1..index + 1, copies);
        Ok(ids)
    }

    pub fn rename_object(&mut self, id: ObjectId, name: &str) -> Result<(), ObjectError> {
        let name = name.trim();
        let current = &self.object(id).ok_or(ObjectError::NotFound)?.name;
        if name.is_empty() || current == name {
            return Ok(());
        }
        let unique = self.unique_object_name(name);
        self.object_mut(id)?.name = unique;
        Ok(())
    }

    /// Change le parent. Avec `keep_transform`, l'objet garde sa place en
    /// monde (« Keep Transform » de Blender) ; sinon sa transformation
    /// locale est conservée et il saute sous son nouveau parent.
    pub fn set_object_parent(
        &mut self,
        id: ObjectId,
        parent: Parent,
        keep_transform: bool,
        bone_world: &[Transform],
    ) -> Result<(), ObjectError> {
        self.object(id).ok_or(ObjectError::NotFound)?;
        self.check_parent(Some(id), parent)?;
        if keep_transform {
            let world = self.object_world(id, bone_world).unwrap_or_default();
            let local = self.parent_world(parent, bone_world).inverse().mul(&world);
            self.object_mut(id)?.local = sanitize(local);
        }
        self.object_mut(id)?.parent = parent;
        Ok(())
    }

    pub fn set_object_local(&mut self, id: ObjectId, local: Transform) -> Result<(), ObjectError> {
        self.object_mut(id)?.local = sanitize(local);
        Ok(())
    }

    /// Place l'objet à la position `index` parmi ses frères (même parent),
    /// bornée ; l'ordre de la liste est celui de l'interface.
    pub fn reorder_object(&mut self, id: ObjectId, index: usize) -> Result<(), ObjectError> {
        let from = self.object_index(id).ok_or(ObjectError::NotFound)?;
        let parent = self.objects[from].parent;
        let object = self.objects.remove(from);
        let siblings: Vec<usize> = (0..self.objects.len())
            .filter(|i| self.objects[*i].parent == parent)
            .collect();
        let at = match siblings.get(index) {
            Some(&i) => i,
            None => siblings.last().map_or(self.objects.len(), |i| i + 1),
        };
        self.objects.insert(at, object);
        Ok(())
    }

    /// Rigide, ou déformé avec la méthode de poids donnée. Les poids
    /// automatiques sont calculés en tâche de fond (voir `Editor::bind_job`) ;
    /// `Manual` garde les poids actuels (vides s'il n'y en avait pas).
    pub fn set_object_binding(
        &mut self,
        id: ObjectId,
        method: Option<SkinMethod>,
    ) -> Result<(), ObjectError> {
        if method.is_some() && self.skeleton.is_empty() {
            return Err(ObjectError::NoBones);
        }
        let object = self.object_mut(id)?;
        object.binding = match (method, &object.binding) {
            (None, _) => Binding::Rigid,
            (Some(method), Binding::Skinned(s)) => Binding::Skinned(SkinBinding {
                skin: s.skin.clone(),
                method,
                bound_to: None,
            }),
            (Some(method), Binding::Rigid) => Binding::Skinned(SkinBinding {
                skin: object.empty_skin(),
                method,
                bound_to: None,
            }),
        };
        object.bump_skin();
        Ok(())
    }

    /// Installe des poids fournis (import) : méthode `Manual`.
    pub fn set_object_weights(&mut self, id: ObjectId, skin: crate::Skin) -> Result<(), ObjectError> {
        let object = self.object_mut(id)?;
        if skin.weights.len() != object.mesh.vertex_count() {
            return Err(ObjectError::InvalidWeights);
        }
        object.binding = Binding::Skinned(SkinBinding {
            skin: Arc::new(skin),
            method: SkinMethod::Manual,
            bound_to: None,
        });
        object.bump_skin();
        Ok(())
    }
}
