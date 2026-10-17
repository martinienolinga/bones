//! Squelette : hiérarchie d'os, requêtes et ajout d'os.

mod copy;
mod edit;
mod error;
mod hierarchy;
mod joints;
mod rest;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_edit;

pub use edit::BoneRemap;
pub use error::SkeletonError;

use glam::Vec3;
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
    /// Os déformant : participe aux poids automatiques (faux pour les os de
    /// contrôle, comme dans Blender).
    #[serde(default = "default_true")]
    pub deform: bool,
    /// Déconnecté explicitement de son parent, même si sa tête touche la
    /// queue de celui-ci.
    #[serde(default)]
    detached: bool,
}

fn default_true() -> bool {
    true
}

impl Bone {
    pub fn new(name: impl Into<String>, parent: Option<BoneId>, rest: Transform, length: f32) -> Self {
        Self {
            name: name.into(),
            parent,
            rest,
            length,
            deform: true,
            detached: false,
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
}
