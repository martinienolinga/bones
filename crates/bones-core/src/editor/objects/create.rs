//! Création d'objets : primitives, maillage importé, ou objets d'un
//! fichier lus par `io::import_scene`. Chaque ajout est une étape
//! d'annulation et sélectionne le nouvel objet.

use std::sync::Arc;

use glam::Vec3;

use crate::editor::Editor;
use crate::io::SceneImport;
use crate::mesh::Primitive;
use crate::scene::{ObjectError, ObjectId, ObjectRef, Parent};
use crate::{Mesh, Transform};

impl Editor {
    /// Taille d'une nouvelle primitive : à l'échelle du corps (15 % de sa
    /// hauteur), 30 cm sans corps.
    fn primitive_size(&self) -> f32 {
        self.rig
            .mesh
            .bounds()
            .map_or(0.3, |(lo, hi)| ((hi.y - lo.y) * 0.15).clamp(0.02, 10.0))
    }

    /// Ajoute une primitive au centre de la vue (point visé par la caméra).
    pub fn add_primitive(&mut self, primitive: Primitive) -> ObjectId {
        let local = Transform {
            translation: self.camera.target,
            scale: Vec3::splat(self.primitive_size()),
            ..Transform::IDENTITY
        };
        let mesh = Arc::new(primitive.mesh());
        let id = self
            .object_edit(|rig| rig.add_object(primitive.label(), mesh, Parent::World, local))
            .expect("parent monde toujours valide");
        self.selected_object = Some(ObjectRef::Extra(id));
        id
    }

    /// Ajoute un maillage exprimé en monde (OBJ, STL, PLY…) comme objet
    /// rigide ; son origine est placée au centre de sa base.
    pub fn add_object_mesh(&mut self, name: &str, mut mesh: Mesh) -> Result<ObjectId, ObjectError> {
        let (lo, hi) = mesh.bounds().ok_or(ObjectError::NotFound)?;
        let c = (lo + hi) * 0.5;
        let pivot = Vec3::new(c.x, lo.y, c.z);
        mesh.transform_by(&Transform::from_translation(-pivot));
        mesh.shrink_to_fit();
        let local = Transform::from_translation(pivot);
        let id = self.object_edit(|rig| rig.add_object(name, Arc::new(mesh), Parent::World, local))?;
        self.selected_object = Some(ObjectRef::Extra(id));
        Ok(id)
    }

    /// Ajoute tous les objets d'un fichier (hiérarchie conservée). Les os du
    /// fichier sont retrouvés par leur nom : un objet attaché à une
    /// articulation suit l'os de même nom, et des poids dont tous les os
    /// existent sont gardés (sinon ils sont transférés depuis le corps).
    /// Une seule étape d'annulation ; le premier objet est sélectionné.
    pub fn add_imported_objects(&mut self, scene: SceneImport) -> Result<Vec<ObjectId>, ObjectError> {
        if !scene.objects.iter().any(|o| o.mesh.triangle_count() > 0) {
            return Err(ObjectError::NotFound);
        }
        let names: Vec<Option<String>> = match &scene.skeleton {
            Some(s) => s.bones().iter().map(|b| Some(b.name.clone())).collect(),
            None => Vec::new(),
        };
        let bones = self.display_world();
        let skeleton = self.rig.skeleton.clone();
        let map = |b: crate::BoneId| names.get(b.index())?.as_deref().and_then(|n| skeleton.find(n));
        let ids = self.object_edit(|rig| Ok(rig.add_imported(scene.objects, None, map, false, &bones)))?;
        self.selected_object = ids.first().map(|id| ObjectRef::Extra(*id));
        Ok(ids)
    }
}
