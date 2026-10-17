//! Édition du squelette de repos : recalcul des transformations locales,
//! déplacement de queue (mode édition) et suppression d'os.

use glam::Vec3;

use super::{BoneId, Skeleton, head_tail};
use crate::Transform;
use crate::math::rotation_arc;

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
        let (head, old_tail) = head_tail(&world[id.index()], self.bone(id).length);
        let new_len = (tail - head).length().max(1e-4);
        let w = &mut world[id.index()];
        w.rotation = (rotation_arc(old_tail - head, tail - head) * w.rotation).normalize();

        let connected: Vec<BoneId> = self.children(id).filter(|c| self.is_connected(*c)).collect();
        for c in connected {
            let (_, child_tail) = head_tail(&world[c.index()], self.bone(c).length);
            let new_head = world[id.index()].transform_point(Vec3::Y * new_len);
            let cw = &mut world[c.index()];
            let old_dir = cw.transform_vector(Vec3::Y);
            cw.rotation = (rotation_arc(old_dir, child_tail - new_head) * cw.rotation).normalize();
            cw.translation = new_head;
            self.bones[c.index()].length = (child_tail - new_head).length().max(1e-4);
        }
        self.bones[id.index()].length = new_len;
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
