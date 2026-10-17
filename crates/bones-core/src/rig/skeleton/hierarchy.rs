//! Hiérarchie : changement de parent avec réordonnancement des indices,
//! suppression d'une branche entière.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use super::joints::retarget;
use super::{Bone, BoneId, BoneRemap, Skeleton, SkeletonError, head_tail};
use crate::Transform;

impl Skeleton {
    /// Vrai si `ancestor` est `bone` lui-même ou l'un de ses ancêtres.
    pub fn is_ancestor(&self, ancestor: BoneId, bone: BoneId) -> bool {
        let mut cur = Some(bone);
        while let Some(b) = cur {
            if b == ancestor {
                return true;
            }
            cur = self.bone(b).parent;
        }
        false
    }

    /// L'os et toute sa descendance, par indices croissants.
    pub fn subtree(&self, id: BoneId) -> Vec<BoneId> {
        self.ids().filter(|b| self.is_ancestor(id, *b)).collect()
    }

    /// Rattache `id` à un nouveau parent (ou en fait une racine) sans le
    /// déplacer ; avec `connect`, sa tête rejoint la queue du parent.
    ///
    /// Les indices peuvent changer (un parent doit précéder ses enfants) :
    /// la correspondance retournée sert à mettre à jour pose, animation,
    /// poids, membres et épingles.
    pub fn set_parent(
        &mut self,
        id: BoneId,
        parent: Option<BoneId>,
        connect: bool,
    ) -> Result<BoneRemap, SkeletonError> {
        if !self.contains(id) || parent.is_some_and(|p| !self.contains(p)) {
            return Err(SkeletonError::UnknownBone);
        }
        if parent.is_some_and(|p| self.is_ancestor(id, p)) {
            return Err(SkeletonError::Cycle);
        }
        let mut world = self.rest_world();
        let bone = &mut self.bones[id.index()];
        bone.parent = parent;
        bone.detached = false;
        if let (true, Some(p)) = (connect, parent) {
            let (_, parent_tail) = head_tail(&world[p.index()], self.bones[p.index()].length);
            let (_, tail) = head_tail(&world[id.index()], self.bones[id.index()].length);
            retarget(
                &mut world[id.index()],
                &mut self.bones[id.index()].length,
                parent_tail,
                tail,
            );
        }
        let remap = self.reorder(&mut world);
        self.set_rest_world(&world);
        Ok(remap)
    }

    /// Réordonne les os pour que chaque parent précède ses enfants, en
    /// gardant au mieux l'ordre actuel (tri topologique stable). `world`
    /// est permuté de la même façon. Identité si l'ordre est déjà valide.
    pub(super) fn reorder(&mut self, world: &mut Vec<Transform>) -> BoneRemap {
        let n = self.bones.len();
        let mut children: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut ready = BinaryHeap::new();
        for (i, bone) in self.bones.iter().enumerate() {
            match bone.parent {
                Some(p) => children[p.index()].push(i),
                None => ready.push(Reverse(i)),
            }
        }
        let mut order = Vec::with_capacity(n);
        while let Some(Reverse(i)) = ready.pop() {
            order.push(i);
            ready.extend(children[i].iter().map(|c| Reverse(*c)));
        }
        debug_assert_eq!(order.len(), n, "hiérarchie cyclique");

        let mut map = vec![None; n];
        for (new, &old) in order.iter().enumerate() {
            map[old] = Some(BoneId(new as u32));
        }
        let mut slots: Vec<Option<Bone>> = std::mem::take(&mut self.bones).into_iter().map(Some).collect();
        self.bones = order
            .iter()
            .map(|&i| {
                let mut bone = slots[i].take().expect("os placé une seule fois");
                bone.parent = bone.parent.and_then(|p| map[p.index()]);
                bone
            })
            .collect();
        *world = order.iter().map(|&i| world[i]).collect();
        BoneRemap {
            map,
            removed_parent: None,
        }
    }

    /// Supprime un os et toute sa descendance. Les influences des os
    /// supprimés reviennent au parent de la branche.
    pub fn remove_with_children(&mut self, id: BoneId) -> BoneRemap {
        let removed: Vec<bool> = self.ids().map(|b| self.is_ancestor(id, b)).collect();
        let parent = self.bone(id).parent;
        let mut next = 0;
        let map: Vec<Option<BoneId>> = removed
            .iter()
            .map(|r| {
                (!r).then(|| {
                    next += 1;
                    BoneId(next - 1)
                })
            })
            .collect();
        let mut keep = removed.iter().map(|r| !r);
        self.bones.retain(|_| keep.next().unwrap_or(true));
        for bone in &mut self.bones {
            // Le parent d'un os conservé est forcément conservé.
            bone.parent = bone.parent.and_then(|p| map[p.index()]);
        }
        BoneRemap {
            removed_parent: parent.and_then(|p| map[p.index()]),
            map,
        }
    }
}
