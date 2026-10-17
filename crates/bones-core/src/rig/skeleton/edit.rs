//! Édition du squelette de repos : recalcul des transformations locales,
//! déplacement de queue (mode édition) et suppression d'un os.

use glam::Vec3;

use super::{BoneId, Skeleton};
use crate::Transform;

/// Correspondance anciens → nouveaux identifiants après la suppression d'un os.
#[derive(Clone, Debug)]
pub struct BoneRemap {
    /// `map[ancien] = nouveau`, `None` pour l'os supprimé.
    pub map: Vec<Option<BoneId>>,
    /// Nouveau identifiant du parent de l'os supprimé, qui hérite de ses influences.
    pub removed_parent: Option<BoneId>,
}

impl BoneRemap {
    pub fn get(&self, old: BoneId) -> Option<BoneId> {
        self.map.get(old.index()).copied().flatten()
    }
}

impl Skeleton {
    /// Recalcule toutes les transformations locales à partir de
    /// transformations monde.
    pub fn set_rest_world(&mut self, world: &[Transform]) {
        assert_eq!(world.len(), self.bones.len());
        for (i, bone) in self.bones.iter_mut().enumerate() {
            bone.rest = match bone.parent {
                Some(p) => world[p.index()].inverse().mul(&world[i]),
                None => world[i],
            };
        }
    }

    /// Déplace la queue d'un os en espace monde (mode édition).
    ///
    /// Comme dans Blender, les enfants connectés suivent par leur tête mais
    /// gardent leur queue en place ; les autres os ne bougent pas.
    pub fn set_tail_world(&mut self, id: BoneId, tail: Vec3) {
        let mut world = self.rest_world();
        self.move_joint(&mut world, id, tail);
        self.set_rest_world(&world);
    }

    /// Supprime un os. Ses enfants sont rattachés à son parent en gardant
    /// leur position monde.
    pub fn remove_bone(&mut self, id: BoneId) -> BoneRemap {
        let mut world = self.rest_world();
        let removed_parent_old = self.bone(id).parent;
        let shift = |old: BoneId| -> BoneId { if old.0 > id.0 { BoneId(old.0 - 1) } else { old } };

        let map: Vec<Option<BoneId>> = self
            .ids()
            .map(|b| if b == id { None } else { Some(shift(b)) })
            .collect();

        for bone in &mut self.bones {
            if bone.parent == Some(id) {
                bone.parent = removed_parent_old;
            }
            bone.parent = bone.parent.map(shift);
        }
        self.bones.remove(id.index());
        world.remove(id.index());
        self.set_rest_world(&world);

        BoneRemap {
            map,
            removed_parent: removed_parent_old.map(shift),
        }
    }
}
