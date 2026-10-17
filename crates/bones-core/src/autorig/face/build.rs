//! Ajout des os du visage à un squelette humanoïde déjà généré.

use super::{FaceBone, FaceLayout, FaceParent};
use crate::{BoneId, Skeleton};

/// Ajoute les os du visage (enfants de `head.x`, mâchoire, langue, dents…) au
/// squelette, d'après la disposition `layout`. Retourne les os effectivement
/// créés : certains sont absents sur les têtes peu détaillées (pas de langue,
/// pas de dents séparées, pas d'oreilles). Sans `head.x`, rien n'est ajouté.
pub fn build(skeleton: &mut Skeleton, layout: &FaceLayout) -> Vec<FaceBone> {
    let Some(head) = skeleton.find("head.x") else {
        return Vec::new();
    };
    let mut created = Vec::new();
    let mut jaw: Option<BoneId> = None;
    let mut tongue: Option<BoneId> = None;
    for bone in FaceBone::ALL {
        let Some((h, t)) = layout.joint(bone) else {
            continue;
        };
        let parent = match bone.parent() {
            FaceParent::Head => head,
            // La mâchoire et la langue précèdent leurs enfants dans `ALL`.
            FaceParent::Jaw => match jaw {
                Some(j) => j,
                None => continue,
            },
            FaceParent::Tongue => match tongue {
                Some(prev) => prev,
                None => continue,
            },
        };
        let id = skeleton.add_bone_world(bone.name(), Some(parent), h, t);
        match bone {
            FaceBone::Jaw => jaw = Some(id),
            FaceBone::Tongue1 | FaceBone::Tongue2 | FaceBone::Tongue3 => tongue = Some(id),
            _ => {}
        }
        created.push(bone);
    }
    created
}
