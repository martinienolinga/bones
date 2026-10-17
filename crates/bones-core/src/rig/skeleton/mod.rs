//! Squelette : hiérarchie d'os, requêtes et ajout d'os.

mod edit;
#[cfg(test)]
mod tests;

pub use edit::BoneRemap;

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

use crate::Transform;
use crate::math::rotation_arc;

/// Identifiant d'un os : son indice dans [`Skeleton`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BoneId(pub u32);

impl BoneId {
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bone {
    pub name: String,
    parent: Option<BoneId>,
    /// Transformation de repos, relative à l'espace du parent.
    pub rest: Transform,
    pub length: f32,
}

impl Bone {
    pub fn new(name: impl Into<String>, parent: Option<BoneId>, rest: Transform, length: f32) -> Self {
        Self {
            name: name.into(),
            parent,
            rest,
            length,
        }
    }

    pub fn parent(&self) -> Option<BoneId> {
        self.parent
    }
}

/// Position de la tête et de la queue d'un os dont on connaît la
/// transformation monde.
pub fn head_tail(world: &Transform, length: f32) -> (Vec3, Vec3) {
    (world.translation, world.transform_point(Vec3::Y * length))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SkeletonError {
    /// L'os `bone` référence un parent qui n'apparaît pas avant lui.
    InvalidParent { bone: usize },
}

impl std::fmt::Display for SkeletonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkeletonError::InvalidParent { bone } => {
                write!(f, "l'os {bone} référence un parent invalide")
            }
        }
    }
}

impl std::error::Error for SkeletonError {}

/// Hiérarchie d'os.
///
/// Invariant : le parent d'un os a toujours un indice strictement inférieur
/// au sien. Une simple boucle dans l'ordre suffit donc pour la cinématique
/// directe.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Vec<Bone>", into = "Vec<Bone>")]
pub struct Skeleton {
    bones: Vec<Bone>,
}

impl TryFrom<Vec<Bone>> for Skeleton {
    type Error = SkeletonError;

    fn try_from(bones: Vec<Bone>) -> Result<Self, Self::Error> {
        for (i, bone) in bones.iter().enumerate() {
            if let Some(p) = bone.parent
                && p.index() >= i
            {
                return Err(SkeletonError::InvalidParent { bone: i });
            }
        }
        Ok(Self { bones })
    }
}

impl From<Skeleton> for Vec<Bone> {
    fn from(skeleton: Skeleton) -> Self {
        skeleton.bones
    }
}

impl Skeleton {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.bones.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bones.is_empty()
    }

    pub fn bones(&self) -> &[Bone] {
        &self.bones
    }

    pub fn bone(&self, id: BoneId) -> &Bone {
        &self.bones[id.index()]
    }

    pub fn bone_mut(&mut self, id: BoneId) -> &mut Bone {
        &mut self.bones[id.index()]
    }

    pub fn contains(&self, id: BoneId) -> bool {
        id.index() < self.bones.len()
    }

    pub fn ids(&self) -> impl Iterator<Item = BoneId> + use<> {
        (0..self.bones.len() as u32).map(BoneId)
    }

    pub fn children(&self, id: BoneId) -> impl Iterator<Item = BoneId> + '_ {
        self.ids().filter(move |c| self.bone(*c).parent == Some(id))
    }

    pub fn find(&self, name: &str) -> Option<BoneId> {
        self.bones
            .iter()
            .position(|b| b.name == name)
            .map(|i| BoneId(i as u32))
    }

    /// Profondeur dans la hiérarchie (0 pour une racine).
    pub fn depth(&self, id: BoneId) -> usize {
        let mut depth = 0;
        let mut cur = self.bone(id).parent;
        while let Some(p) = cur {
            depth += 1;
            cur = self.bone(p).parent;
        }
        depth
    }

    /// Nom libre dérivé de `base` : `Os`, `Os.001`, `Os.002`…
    pub fn unique_name(&self, base: &str) -> String {
        if self.find(base).is_none() {
            return base.to_owned();
        }
        (1..)
            .map(|i| format!("{base}.{i:03}"))
            .find(|n| self.find(n).is_none())
            .expect("itérateur infini")
    }

    /// Ajoute un os. Le parent doit déjà exister.
    pub fn add_bone(
        &mut self,
        name: impl Into<String>,
        parent: Option<BoneId>,
        rest: Transform,
        length: f32,
    ) -> BoneId {
        if let Some(p) = parent {
            assert!(self.contains(p), "parent inexistant : {p:?}");
        }
        self.bones.push(Bone::new(name, parent, rest, length));
        BoneId(self.bones.len() as u32 - 1)
    }

    /// Ajoute un os défini par sa tête et sa queue en espace monde (pose de repos).
    pub fn add_bone_world(
        &mut self,
        name: impl Into<String>,
        parent: Option<BoneId>,
        head: Vec3,
        tail: Vec3,
    ) -> BoneId {
        let world = Transform::from_rotation_translation(rotation_arc(Vec3::Y, tail - head), head);
        let local = match parent {
            Some(p) => self.rest_world_of(p).inverse().mul(&world),
            None => world,
        };
        self.add_bone(name, parent, local, (tail - head).length())
    }

    /// Chaîne d'os de la racine vers `tip`, limitée à `max_len` os.
    pub fn chain(&self, tip: BoneId, max_len: usize) -> Vec<BoneId> {
        let mut chain = vec![tip];
        let mut cur = self.bone(tip).parent;
        while let Some(p) = cur {
            if chain.len() >= max_len.max(1) {
                break;
            }
            chain.push(p);
            cur = self.bone(p).parent;
        }
        chain.reverse();
        chain
    }

    /// Vrai si l'enfant `child` est collé à la queue de son parent.
    pub fn is_connected(&self, child: BoneId) -> bool {
        let bone = self.bone(child);
        let Some(parent) = bone.parent else {
            return false;
        };
        let len = self.bone(parent).length;
        bone.rest
            .translation
            .abs_diff_eq(Vec3::Y * len, 1e-4 * len.max(1.0))
    }

    /// Transformations monde de la pose de repos.
    pub fn rest_world(&self) -> Vec<Transform> {
        let mut world: Vec<Transform> = Vec::with_capacity(self.bones.len());
        for bone in &self.bones {
            let w = match bone.parent {
                Some(p) => world[p.index()].mul(&bone.rest),
                None => bone.rest,
            };
            world.push(w);
        }
        world
    }

    fn rest_world_of(&self, id: BoneId) -> Transform {
        let bone = self.bone(id);
        match bone.parent {
            Some(p) => self.rest_world_of(p).mul(&bone.rest),
            None => bone.rest,
        }
    }

    /// Rotation monde de repos d'un os (utile pour les outils d'édition).
    pub fn rest_world_rotation(&self, id: BoneId) -> Quat {
        self.rest_world_of(id).rotation
    }
}
