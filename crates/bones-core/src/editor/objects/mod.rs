//! Objets de la scène dans l'éditeur (voir [`crate::scene`]) : sélection,
//! lecture pour l'interface et le rendu, opérations avec historique, gestes
//! du mode Objet, manipulateur et liaisons en tâche de fond.

mod bind;
mod create;
mod gesture;
mod manip;
mod ops;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_gesture;

use glam::{Vec2, Vec3};

use super::Editor;
use crate::scene::{BindStamp, ObjectDraw, ObjectId, ObjectRef, OutlinerRow, Parent, SkinMethod};
use crate::{Transform, Viewport};

/// État de l'éditeur propre aux objets (non enregistré).
#[derive(Default)]
pub(super) struct ObjectState {
    /// Liaisons d'objets demandées et pas encore revenues : (jeton, objet,
    /// état visé). Le jeton est la seconde valeur de la clé du `BindJob`.
    binds: Vec<(u64, ObjectId, BindStamp)>,
    /// Dernier toucher du mode Objet resté sans objet dessous : un simple
    /// toucher sélectionne alors ce qui s'y trouve (le corps, ou rien).
    last_down: Option<(Vec2, Viewport)>,
}

/// Résumé d'un objet pour le panneau de propriétés.
#[derive(Clone, Debug, PartialEq)]
pub struct ObjectInfo {
    pub id: ObjectId,
    pub name: String,
    pub parent: Parent,
    /// Transformation relative au parent.
    pub local: Transform,
    /// Transformation monde affichée.
    pub world: Transform,
    /// `None` : rigide ; sinon méthode des poids.
    pub skin_method: Option<SkinMethod>,
    /// Poids automatiques en attente de calcul.
    pub binding_pending: bool,
    pub visible: bool,
    pub locked: bool,
    pub vertices: usize,
    pub triangles: usize,
    pub children: usize,
}

impl Editor {
    /// Objet supplémentaire sélectionné.
    pub fn selected_extra(&self) -> Option<ObjectId> {
        match self.selected_object {
            Some(ObjectRef::Extra(id)) => Some(id),
            _ => None,
        }
    }

    /// Sélectionne un objet (ou rien) ; un simple choix ne crée pas
    /// d'étape d'annulation.
    pub fn select_object(&mut self, item: Option<ObjectRef>) {
        self.selected_object = item.filter(|r| self.object_exists(*r));
    }

    pub fn object_exists(&self, item: ObjectRef) -> bool {
        match item {
            ObjectRef::Body => true,
            ObjectRef::Extra(id) => self.rig.object(id).is_some(),
        }
    }

    pub(super) fn validate_object_selection(&mut self) {
        if self.selected_object.is_some_and(|r| !self.object_exists(r)) {
            self.selected_object = None;
        }
    }

    /// Parent de l'objet sélectionné (s'il est un objet).
    pub fn select_object_parent(&mut self) {
        let parent = self
            .selected_extra()
            .and_then(|id| self.rig.object(id))
            .map(|o| o.parent());
        if let Some(Parent::Object(p)) = parent {
            self.selected_object = Some(ObjectRef::Extra(p));
        }
    }

    /// Premier enfant de l'objet sélectionné.
    pub fn select_object_child(&mut self) {
        let child = self
            .selected_extra()
            .and_then(|id| self.rig.object_children(Parent::Object(id)).next());
        if let Some(c) = child {
            self.selected_object = Some(ObjectRef::Extra(c));
        }
    }

    /// Frère suivant (même parent), circulaire.
    pub fn select_next_object_sibling(&mut self) {
        let Some(id) = self.selected_extra() else { return };
        let Some(parent) = self.rig.object(id).map(|o| o.parent()) else {
            return;
        };
        let siblings: Vec<ObjectId> = self.rig.object_children(parent).collect();
        if let Some(i) = siblings.iter().position(|s| *s == id) {
            self.selected_object = Some(ObjectRef::Extra(siblings[(i + 1) % siblings.len()]));
        }
    }

    /// Enfants objets directs de `item` (le corps n'en a pas).
    pub fn object_children_of(&self, item: ObjectRef) -> Vec<ObjectId> {
        match item {
            ObjectRef::Body => Vec::new(),
            ObjectRef::Extra(id) => self.rig.object_children(Parent::Object(id)).collect(),
        }
    }

    /// Lignes de la liste des objets (corps puis objets).
    pub fn outliner(&self) -> Vec<OutlinerRow> {
        self.rig.outliner_rows()
    }

    pub fn object_info(&self, id: ObjectId) -> Option<ObjectInfo> {
        let o = self.rig.object(id)?;
        let skin = o.binding().skin();
        Some(ObjectInfo {
            id,
            name: o.name.clone(),
            parent: o.parent(),
            local: o.local(),
            world: self.object_display_world(id)?,
            skin_method: skin.map(|s| s.method),
            binding_pending: self.rig.object_needs_bind(id),
            visible: o.visible,
            locked: o.locked,
            vertices: o.mesh().vertex_count(),
            triangles: o.mesh().triangle_count(),
            children: self.rig.object_children(Parent::Object(id)).count(),
        })
    }

    /// Transformation monde d'un objet telle qu'affichée (os posés ou au
    /// repos selon le mode).
    pub fn object_display_world(&self, id: ObjectId) -> Option<Transform> {
        self.rig.object_world(id, &self.display_world())
    }

    /// Le corps est à dessiner (en plus de `ViewOptions::mesh`).
    pub fn body_visible(&self) -> bool {
        self.rig.body.visible
    }

    /// Appels de dessin des objets visibles (voir [`ObjectDraw`]). Les objets
    /// déformés utilisent la palette [`Editor::skinning_matrices`].
    pub fn object_draws(&self) -> Vec<ObjectDraw> {
        if self.rig.objects().is_empty() {
            return Vec::new();
        }
        let rest = self.rig.skeleton.rest_world();
        self.rig
            .object_draws(&self.display_world(), &rest, self.selected_extra())
    }

    /// Cadre la caméra sur l'élément sélectionné (corps ou objet).
    pub fn frame_selected_object(&mut self) {
        let bounds = match self.selected_object {
            Some(ObjectRef::Extra(id)) => self.object_bounds(id),
            Some(ObjectRef::Body) => self.rig.mesh.bounds(),
            None => None,
        };
        if let Some((lo, hi)) = bounds {
            self.camera.frame(lo, hi);
        }
    }

    /// Boîte englobante monde d'un objet tel qu'affiché (coins transformés).
    pub fn object_bounds(&self, id: ObjectId) -> Option<(Vec3, Vec3)> {
        let (lo, hi) = self.rig.object(id)?.mesh().bounds()?;
        let world = self.object_display_world(id)?;
        let corners = (0..8).map(|i| {
            let pick = |bit: u32, a: f32, b: f32| if i & bit == 0 { a } else { b };
            world.transform_point(Vec3::new(
                pick(1, lo.x, hi.x),
                pick(2, lo.y, hi.y),
                pick(4, lo.z, hi.z),
            ))
        });
        corners.fold(None, |acc: Option<(Vec3, Vec3)>, p| match acc {
            None => Some((p, p)),
            Some((a, b)) => Some((a.min(p), b.max(p))),
        })
    }
}
