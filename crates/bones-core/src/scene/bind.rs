//! Poids des objets déformés : état pour lequel ils ont été calculés,
//! données d'un calcul en tâche de fond, et installation du résultat.
//!
//! Un objet déformé est d'abord placé par sa transformation évaluée au
//! repos (placement de liaison) ; ses poids dépendent du squelette, de son
//! maillage, de ce placement, de la méthode et, pour un transfert, des poids
//! du corps. Le tampon [`BindStamp`] résume tout cela : des poids sont à
//! jour tant que le tampon actuel est celui de leur calcul.

use std::sync::Arc;

use rayon::prelude::*;

use super::{Binding, ObjectId, SkinMethod};
use crate::{BindMethod, Mesh, Rig, Skeleton, Skin, Transform};

/// État pour lequel les poids d'un objet ont été calculés.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BindStamp {
    skeleton: u64,
    mesh: u64,
    /// Révision des poids du corps (transfert), sinon 0.
    source: u64,
    method: SkinMethod,
    /// Placement de liaison, bit à bit.
    placement: [u32; 10],
}

/// Source des poids d'un calcul.
pub(crate) enum WeightSource {
    /// Transfert depuis le corps (maillage et poids de repos).
    Transfer(Arc<Mesh>, Arc<Skin>),
    Compute(BindMethod),
}

/// Données autonomes d'un calcul de poids d'objet (tâche de fond).
pub(crate) struct ObjectBindInput {
    pub id: ObjectId,
    pub mesh: Arc<Mesh>,
    pub placement: Transform,
    pub source: WeightSource,
    pub stamp: BindStamp,
}

impl ObjectBindInput {
    /// Calcule les poids (peut durer : à lancer hors du fil de l'interface).
    pub fn compute(&self, skeleton: &Skeleton) -> Skin {
        let m = self.placement;
        let positions: Vec<glam::Vec3> = self
            .mesh
            .positions
            .par_iter()
            .map(|p| m.transform_point(*p))
            .collect();
        match &self.source {
            WeightSource::Transfer(body, skin) => Skin::transfer(body, skin, &positions),
            WeightSource::Compute(method) => {
                let normals = self
                    .mesh
                    .normals
                    .iter()
                    .map(|n| m.transform_vector(*n).normalize_or(*n))
                    .collect();
                let placed = Mesh {
                    positions,
                    normals,
                    indices: self.mesh.indices.clone(),
                    ..Mesh::default()
                };
                method.compute(&placed, skeleton)
            }
        }
    }
}

fn bits(t: &Transform) -> [u32; 10] {
    let v = [
        t.translation.x,
        t.translation.y,
        t.translation.z,
        t.rotation.x,
        t.rotation.y,
        t.rotation.z,
        t.rotation.w,
        t.scale.x,
        t.scale.y,
        t.scale.z,
    ];
    v.map(f32::to_bits)
}

impl Rig {
    /// Placement de liaison d'un objet : sa transformation monde évaluée
    /// dans la pose de repos.
    pub fn object_bind_placement(&self, id: ObjectId) -> Option<Transform> {
        self.object_world(id, &self.skeleton.rest_world())
    }

    /// Le transfert depuis le corps est possible (corps maillé et lié).
    fn body_transferable(&self) -> bool {
        self.mesh.triangle_count() > 0 && (!self.needs_bind() || self.weights_painted())
    }

    fn stamp_of(&self, index: usize, rest: &[Transform]) -> Option<BindStamp> {
        let o = &self.objects[index];
        let Binding::Skinned(s) = &o.binding else {
            return None;
        };
        let transfer = s.method == SkinMethod::Transfer && self.mesh.triangle_count() > 0;
        Some(BindStamp {
            skeleton: self.revisions.skeleton,
            mesh: o.revisions.mesh,
            source: if transfer { self.revisions.skin } else { 0 },
            method: s.method,
            placement: bits(&self.object_world(o.id, rest).unwrap_or_default()),
        })
    }

    /// Les poids automatiques de l'objet ne correspondent plus à son état.
    pub fn object_needs_bind(&self, id: ObjectId) -> bool {
        let Some(i) = self.object_index(id) else {
            return false;
        };
        self.stale(i, &self.skeleton.rest_world())
    }

    fn stale(&self, index: usize, rest: &[Transform]) -> bool {
        match &self.objects[index].binding {
            Binding::Skinned(s) if s.method.is_automatic() && !self.skeleton.is_empty() => {
                s.bound_to != self.stamp_of(index, rest)
            }
            _ => false,
        }
    }

    /// Objets dont les poids automatiques sont à recalculer.
    pub(crate) fn stale_objects(&self) -> Vec<ObjectId> {
        if !self.objects.iter().any(|o| o.binding.is_skinned()) {
            return Vec::new();
        }
        let rest = self.skeleton.rest_world();
        (0..self.objects.len())
            .filter(|i| self.stale(*i, &rest))
            .map(|i| self.objects[i].id)
            .collect()
    }

    /// Considère à jour les poids des objets déformés dont les poids sont
    /// utilisables (chargement d'un projet) ; les autres seront recalculés.
    pub(crate) fn settle_object_bindings(&mut self) {
        let rest = self.skeleton.rest_world();
        for i in 0..self.objects.len() {
            let usable = match &self.objects[i].binding {
                Binding::Skinned(s) => s.skin.weights.iter().any(|w| w.weights[0] > 0),
                Binding::Rigid => false,
            };
            let stamp = usable.then(|| self.stamp_of(i, &rest)).flatten();
            if usable && let Binding::Skinned(s) = &mut self.objects[i].binding {
                s.bound_to = stamp;
            }
        }
    }

    /// Données du prochain calcul de poids d'objet à lancer (premier objet
    /// périmé qui n'est pas dans `skip`). Un transfert attend que le corps
    /// soit lié ; sans corps, `fallback` est utilisée.
    pub(crate) fn object_bind_input(
        &self,
        fallback: BindMethod,
        skip: &[(ObjectId, BindStamp)],
    ) -> Option<ObjectBindInput> {
        if self.skeleton.is_empty() || !self.objects.iter().any(|o| o.binding.is_skinned()) {
            return None;
        }
        let rest = self.skeleton.rest_world();
        (0..self.objects.len()).find_map(|i| {
            if !self.stale(i, &rest) {
                return None;
            }
            let o = &self.objects[i];
            let stamp = self.stamp_of(i, &rest)?;
            if skip.contains(&(o.id, stamp)) {
                return None;
            }
            let method = o.binding.skin()?.method;
            let source = match method.bind_method() {
                Some(m) => WeightSource::Compute(m),
                None if self.mesh.triangle_count() == 0 => WeightSource::Compute(fallback),
                // Transfert : le corps doit d'abord être lié.
                None if !self.body_transferable() => return None,
                None => WeightSource::Transfer(self.mesh.clone(), self.skin.clone()),
            };
            Some(ObjectBindInput {
                id: o.id,
                mesh: o.mesh.clone(),
                placement: self.object_world(o.id, &rest).unwrap_or_default(),
                source,
                stamp,
            })
        })
    }

    /// Oublie l'état de calcul des poids d'un objet : ils seront recalculés.
    pub(crate) fn invalidate_object_bind(&mut self, id: ObjectId) {
        if let Ok(o) = self.object_mut(id)
            && let Binding::Skinned(s) = &mut o.binding
        {
            s.bound_to = None;
        }
    }

    /// Installe des poids calculés pour `stamp` ; ignoré si l'objet a
    /// changé entre-temps (résultat périmé).
    pub(crate) fn apply_object_skin(&mut self, id: ObjectId, skin: Skin, stamp: BindStamp) -> bool {
        let Some(i) = self.object_index(id) else {
            return false;
        };
        let current = self.stamp_of(i, &self.skeleton.rest_world());
        let o = &mut self.objects[i];
        let Binding::Skinned(s) = &mut o.binding else {
            return false;
        };
        if current != Some(stamp) || skin.weights.len() != o.mesh.vertex_count() {
            return false;
        }
        s.skin = Arc::new(skin);
        s.bound_to = Some(stamp);
        o.bump_skin();
        true
    }
}
