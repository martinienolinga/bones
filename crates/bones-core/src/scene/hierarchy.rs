//! Hiérarchie des objets : recherche, enfants, ascendance et évaluation des
//! transformations monde (objet → objet → os → monde).

use super::{ObjectError, ObjectId, Parent, SceneObject};
use crate::{BoneId, Rig, Transform};

impl Rig {
    /// Objets supplémentaires, dans l'ordre de la liste.
    pub fn objects(&self) -> &[SceneObject] {
        &self.objects
    }

    pub(crate) fn objects_mut(&mut self) -> &mut Vec<SceneObject> {
        &mut self.objects
    }

    pub fn object_index(&self, id: ObjectId) -> Option<usize> {
        self.objects.iter().position(|o| o.id == id)
    }

    pub fn object(&self, id: ObjectId) -> Option<&SceneObject> {
        self.objects.iter().find(|o| o.id == id)
    }

    pub(crate) fn object_mut(&mut self, id: ObjectId) -> Result<&mut SceneObject, ObjectError> {
        self.objects
            .iter_mut()
            .find(|o| o.id == id)
            .ok_or(ObjectError::NotFound)
    }

    /// Identifiant libre (jamais celui d'un objet existant).
    pub(crate) fn next_object_id(&self) -> ObjectId {
        ObjectId(self.objects.iter().map(|o| o.id.0 + 1).max().unwrap_or(1).max(1))
    }

    /// Enfants directs de `parent`, dans l'ordre de la liste.
    pub fn object_children(&self, parent: Parent) -> impl Iterator<Item = ObjectId> + '_ {
        self.objects
            .iter()
            .filter(move |o| o.parent == parent)
            .map(|o| o.id)
    }

    /// Vrai si `ancestor` est `id` lui-même ou l'un de ses ascendants objets.
    pub fn is_object_ancestor(&self, ancestor: ObjectId, id: ObjectId) -> bool {
        let mut cur = Some(id);
        // Borné par le nombre d'objets : jamais de boucle infinie, même sur
        // un document incohérent.
        for _ in 0..=self.objects.len() {
            match cur {
                Some(c) if c == ancestor => return true,
                Some(c) => {
                    cur = match self.object(c).map(|o| o.parent) {
                        Some(Parent::Object(p)) => Some(p),
                        _ => None,
                    }
                }
                None => return false,
            }
        }
        false
    }

    /// `id` puis tous ses descendants (parcours en profondeur, ordre de la liste).
    pub fn object_subtree(&self, id: ObjectId) -> Vec<ObjectId> {
        let mut out = Vec::new();
        let mut stack = vec![id];
        while let Some(cur) = stack.pop() {
            if out.contains(&cur) {
                continue;
            }
            out.push(cur);
            let children: Vec<ObjectId> = self.object_children(Parent::Object(cur)).collect();
            stack.extend(children.into_iter().rev());
        }
        out
    }

    /// Vérifie qu'un parent est possible pour `id` (existe, sans cycle).
    pub(crate) fn check_parent(&self, id: Option<ObjectId>, parent: Parent) -> Result<(), ObjectError> {
        match parent {
            Parent::World => Ok(()),
            Parent::Bone(b) if self.skeleton.contains(b) => Ok(()),
            Parent::Bone(_) => Err(ObjectError::NoSuchBone),
            Parent::Object(p) if self.object(p).is_none() => Err(ObjectError::NotFound),
            Parent::Object(p) => match id {
                Some(id) if self.is_object_ancestor(id, p) => Err(ObjectError::Cycle),
                _ => Ok(()),
            },
        }
    }

    /// Transformations monde de tous les objets (indices de `objects()`),
    /// `bone_world` étant celles des os (posées ou de repos).
    pub fn object_worlds(&self, bone_world: &[Transform]) -> Vec<Transform> {
        let n = self.objects.len();
        let mut worlds: Vec<Option<Transform>> = vec![None; n];
        for i in 0..n {
            self.resolve(i, bone_world, &mut worlds);
        }
        worlds.into_iter().map(|w| w.unwrap_or_default()).collect()
    }

    /// Transformation monde d'un seul objet (remonte seulement sa chaîne).
    pub fn object_world(&self, id: ObjectId, bone_world: &[Transform]) -> Option<Transform> {
        let mut chain = Vec::new();
        let mut cur = Some(id);
        while let Some(c) = cur {
            let o = self.object(c)?;
            if chain.len() > self.objects.len() {
                break;
            }
            chain.push(o.local);
            cur = match o.parent {
                Parent::Object(p) => Some(p),
                Parent::Bone(b) => {
                    chain.push(bone_transform(bone_world, b));
                    None
                }
                Parent::World => None,
            };
        }
        Some(chain.iter().rev().fold(Transform::IDENTITY, |acc, t| acc.mul(t)))
    }

    /// Transformation monde d'un parent (objet manquant : le monde).
    pub fn parent_world(&self, parent: Parent, bone_world: &[Transform]) -> Transform {
        match parent {
            Parent::World => Transform::IDENTITY,
            Parent::Bone(b) => bone_transform(bone_world, b),
            Parent::Object(p) => self.object_world(p, bone_world).unwrap_or_default(),
        }
    }

    /// Calcule (et mémorise) le monde de l'objet `i` et de ses ascendants,
    /// sans récursion : la chaîne est remontée puis redescendue.
    fn resolve(&self, i: usize, bone_world: &[Transform], worlds: &mut [Option<Transform>]) {
        let mut chain = vec![i];
        let mut base = Transform::IDENTITY;
        loop {
            let cur = self.objects[*chain.last().expect("chaîne non vide")].parent;
            match cur {
                Parent::World => break,
                Parent::Bone(b) => {
                    base = bone_transform(bone_world, b);
                    break;
                }
                Parent::Object(p) => match self.object_index(p) {
                    Some(j) if worlds[j].is_some() => {
                        base = worlds[j].unwrap_or_default();
                        break;
                    }
                    // Cycle (document incohérent) ou parent absent : monde.
                    Some(j) if !chain.contains(&j) => chain.push(j),
                    _ => break,
                },
            }
        }
        for &k in chain.iter().rev() {
            base = base.mul(&self.objects[k].local);
            worlds[k] = Some(base);
        }
    }
}

fn bone_transform(bone_world: &[Transform], b: BoneId) -> Transform {
    bone_world.get(b.index()).copied().unwrap_or_default()
}
