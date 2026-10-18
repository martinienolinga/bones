//! Opérations de l'utilisateur sur les objets : chacune crée une étape
//! d'annulation (aucune si elle est refusée), et retourne une erreur
//! lisible (en anglais) quand l'opération est impossible.

use crate::editor::Editor;
use crate::scene::{ObjectError, ObjectId, ObjectRef, Parent, SkinMethod};
use crate::{BoneId, Rig, Transform};

/// Code de propriété des retouches de transformation d'objet regroupées
/// (voir `Editor::coalesce` ; les os utilisent les codes 1 à 4).
const OBJECT_TRANSFORM: u8 = 0x80;

impl Editor {
    /// Applique `f` au document avec une étape d'annulation, abandonnée si
    /// `f` échoue (le document revient alors à l'état d'avant).
    pub(super) fn object_edit<T>(
        &mut self,
        f: impl FnOnce(&mut Rig) -> Result<T, ObjectError>,
    ) -> Result<T, ObjectError> {
        self.checkpoint();
        match f(&mut self.rig) {
            Ok(v) => Ok(v),
            Err(e) => {
                self.drop_checkpoint(true);
                Err(e)
            }
        }
    }

    fn unlocked(&self, id: ObjectId) -> Result<(), ObjectError> {
        match self.rig.object(id) {
            None => Err(ObjectError::NotFound),
            Some(o) if o.locked => Err(ObjectError::Locked),
            Some(_) => Ok(()),
        }
    }

    /// Supprime un objet ; ses enfants passent à son parent sans bouger.
    pub fn delete_object(&mut self, id: ObjectId) -> Result<(), ObjectError> {
        let bones = self.display_world();
        self.object_edit(|rig| rig.remove_object(id, &bones))?;
        self.validate_object_selection();
        Ok(())
    }

    /// Supprime l'élément sélectionné (le corps ne peut pas l'être).
    pub fn delete_selected_object(&mut self) -> Result<(), ObjectError> {
        match self.selected_object {
            Some(ObjectRef::Extra(id)) => self.delete_object(id),
            Some(ObjectRef::Body) => Err(ObjectError::Body),
            None => Err(ObjectError::NotFound),
        }
    }

    /// Copie un objet (et ses descendants si demandé) ; la copie est sélectionnée.
    pub fn duplicate_object(&mut self, id: ObjectId, with_children: bool) -> Result<ObjectId, ObjectError> {
        let copies = self.object_edit(|rig| rig.duplicate_object(id, with_children))?;
        let first = copies[0];
        self.selected_object = Some(ObjectRef::Extra(first));
        Ok(first)
    }

    /// Renomme un objet ou le corps (noms d'objets uniques).
    pub fn rename_object(&mut self, item: ObjectRef, name: &str) -> Result<(), ObjectError> {
        let name = name.trim();
        match item {
            ObjectRef::Body => {
                if name.is_empty() || self.rig.body.name == name {
                    return Ok(());
                }
                self.object_edit(|rig| {
                    rig.body.name = name.to_owned();
                    Ok(())
                })
            }
            ObjectRef::Extra(id) => {
                let current = self.rig.object(id).ok_or(ObjectError::NotFound)?;
                if name.is_empty() || current.name == name {
                    return Ok(());
                }
                self.object_edit(|rig| rig.rename_object(id, name))
            }
        }
    }

    pub fn set_object_visible(&mut self, item: ObjectRef, visible: bool) -> Result<(), ObjectError> {
        self.set_flag(item, visible, false)
    }

    /// Verrou : plus de sélection dans la vue 3D ni de transformation.
    pub fn set_object_locked(&mut self, item: ObjectRef, locked: bool) -> Result<(), ObjectError> {
        self.set_flag(item, locked, true)
    }

    fn set_flag(&mut self, item: ObjectRef, value: bool, lock: bool) -> Result<(), ObjectError> {
        if *flag_mut(&mut self.rig, item, lock)? == value {
            return Ok(());
        }
        self.object_edit(|rig| {
            *flag_mut(rig, item, lock)? = value;
            Ok(())
        })
    }

    /// Change le parent ; `keep_transform` garde la place en monde.
    pub fn set_object_parent(
        &mut self,
        id: ObjectId,
        parent: Parent,
        keep_transform: bool,
    ) -> Result<(), ObjectError> {
        if self.rig.object(id).ok_or(ObjectError::NotFound)?.parent() == parent {
            return Ok(());
        }
        let bones = self.display_world();
        self.object_edit(|rig| rig.set_object_parent(id, parent, keep_transform, &bones))
    }

    /// Attache l'objet à l'os sélectionné (« Parent > Bone » de Blender), en
    /// gardant sa place.
    pub fn parent_object_to_selected_bone(&mut self, id: ObjectId) -> Result<(), ObjectError> {
        let bone: BoneId = self.selected.ok_or(ObjectError::NoSuchBone)?;
        self.set_object_parent(id, Parent::Bone(bone), true)
    }

    /// Détache l'objet : il passe au monde (« Clear Parent »).
    pub fn clear_object_parent(&mut self, id: ObjectId, keep_transform: bool) -> Result<(), ObjectError> {
        self.set_object_parent(id, Parent::World, keep_transform)
    }

    /// Rigide (`None`) ou déformé par le squelette avec la méthode donnée ;
    /// les poids automatiques sont calculés en tâche de fond.
    pub fn set_object_binding(
        &mut self,
        id: ObjectId,
        method: Option<SkinMethod>,
    ) -> Result<(), ObjectError> {
        let current = self.rig.object(id).ok_or(ObjectError::NotFound)?;
        if current.binding().skin().map(|s| s.method) == method {
            return Ok(());
        }
        self.object_edit(|rig| rig.set_object_binding(id, method))?;
        self.objects.binds.retain(|(_, o, _)| *o != id);
        Ok(())
    }

    /// Transformation relative au parent (champs numériques) : une suite de
    /// retouches du même objet ne crée qu'une étape d'annulation.
    pub fn set_object_transform(&mut self, id: ObjectId, local: Transform) -> Result<(), ObjectError> {
        self.unlocked(id)?;
        let key = (BoneId(id.0), OBJECT_TRANSFORM);
        if self.coalesce == Some(key) {
            self.count_edit(true);
        } else {
            self.checkpoint();
            self.coalesce = Some(key);
        }
        self.rig.set_object_local(id, local)
    }

    /// Place l'objet à la position `index` parmi ses frères.
    pub fn reorder_object(&mut self, id: ObjectId, index: usize) -> Result<(), ObjectError> {
        self.object_edit(|rig| rig.reorder_object(id, index))
    }
}

/// Visibilité (ou verrou si `lock`) du corps ou d'un objet.
fn flag_mut(rig: &mut Rig, item: ObjectRef, lock: bool) -> Result<&mut bool, ObjectError> {
    match item {
        ObjectRef::Body if lock => Ok(&mut rig.body.locked),
        ObjectRef::Body => Ok(&mut rig.body.visible),
        ObjectRef::Extra(id) => {
            let o = rig.object_mut(id)?;
            Ok(if lock { &mut o.locked } else { &mut o.visible })
        }
    }
}
