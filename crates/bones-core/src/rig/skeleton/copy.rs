//! Duplication et subdivision d'os.

use glam::Vec3;

use super::{Bone, BoneId, BoneRemap, Skeleton};
use crate::Transform;

/// Suffixes de côté conservés par les copies (`bras.l` → `bras.001.l`).
const SIDES: [&str; 8] = [".l", ".r", ".L", ".R", "_l", "_r", "_L", "_R"];

/// Retire un numéro de copie final (`.001`).
fn strip_number(stem: &str) -> &str {
    match stem.rsplit_once('.') {
        Some((base, n)) if n.len() == 3 && n.bytes().all(|b| b.is_ascii_digit()) => base,
        _ => stem,
    }
}

impl Skeleton {
    /// Nom libre pour une copie de `name`, qui garde le suffixe de côté pour
    /// que la symétrie continue de fonctionner.
    pub fn copy_name(&self, name: &str) -> String {
        let (stem, side) = SIDES
            .iter()
            .find_map(|s| name.strip_suffix(s).map(|stem| (stem, *s)))
            .unwrap_or((name, ""));
        let stem = strip_number(stem);
        (1..)
            .map(|i| format!("{stem}.{i:03}{side}"))
            .find(|n| self.find(n).is_none())
            .expect("itérateur infini")
    }

    /// Copie un os (avec sa descendance si demandé) à la même place. Les
    /// copies sont ajoutées à la fin ; celle de `id` est la première
    /// retournée et garde le même parent.
    pub fn duplicate(&mut self, id: BoneId, with_children: bool) -> Vec<BoneId> {
        let sources = if with_children { self.subtree(id) } else { vec![id] };
        let mut copies: Vec<(BoneId, BoneId)> = Vec::with_capacity(sources.len());
        for source in sources {
            let bone = self.bone(source).clone();
            // Les parents passent avant leurs enfants : leur copie existe déjà.
            let parent = copies
                .iter()
                .find(|(s, _)| Some(*s) == bone.parent)
                .map(|(_, c)| *c)
                .or(bone.parent);
            let name = self.copy_name(&bone.name);
            self.bones.push(Bone { name, parent, ..bone });
            copies.push((source, BoneId(self.bones.len() as u32 - 1)));
        }
        copies.into_iter().map(|(_, c)| c).collect()
    }

    /// Coupe un os en `parts` os connectés de même direction ; les enfants
    /// passent au dernier morceau, sans bouger. Retourne les morceaux (dans
    /// l'ordre, l'original en premier) et la correspondance des indices
    /// (sur le squelette agrandi, avant réordonnancement).
    pub fn subdivide(&mut self, id: BoneId, parts: usize) -> (Vec<BoneId>, BoneRemap) {
        let parts = parts.max(2);
        let segment = self.bone(id).length / parts as f32;
        let children: Vec<BoneId> = self.children(id).collect();
        let mut world = self.rest_world();
        self.bones[id.index()].length = segment;
        let mut pieces = vec![id];
        for _ in 1..parts {
            let previous = *pieces.last().expect("au moins un morceau");
            let rest = Transform::from_translation(Vec3::Y * segment);
            let name = self.copy_name(&self.bones[id.index()].name);
            let piece = self.add_bone(name, Some(previous), rest, segment);
            self.bones[piece.index()].deform = self.bones[id.index()].deform;
            world.push(world[previous.index()].mul(&rest));
            pieces.push(piece);
        }
        let last = *pieces.last().expect("au moins un morceau");
        for child in children {
            self.bones[child.index()].parent = Some(last);
        }
        let remap = self.reorder(&mut world);
        self.set_rest_world(&world);
        let pieces = pieces.iter().filter_map(|p| remap.get(*p)).collect();
        (pieces, remap)
    }
}
