//! Symétrie gauche/droite : correspondance des noms d'os, symétrisation du
//! squelette (comme le « Mirror » d'Auto-Rig Pro) et retournement de pose.
//! Le plan de symétrie est le plan YZ (X → −X).

use glam::{Quat, Vec3};

use crate::{BoneId, Pose, Skeleton, Transform};

#[cfg(test)]
mod tests;

const SUFFIXES: [(&str, &str); 6] = [
    (".l", ".r"),
    (".L", ".R"),
    ("_l", "_r"),
    ("_L", "_R"),
    ("Left", "Right"),
    ("left", "right"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

/// Côté d'un os d'après son nom.
pub fn side_of(name: &str) -> Option<Side> {
    SUFFIXES.iter().find_map(|(l, r)| {
        if name.ends_with(l) {
            Some(Side::Left)
        } else if name.ends_with(r) {
            Some(Side::Right)
        } else {
            None
        }
    })
}

/// Nom de l'os symétrique (`arm.l` ↔ `arm.r`), ou `None` pour un os central.
pub fn mirror_name(name: &str) -> Option<String> {
    for (l, r) in SUFFIXES {
        if let Some(stem) = name.strip_suffix(l) {
            return Some(format!("{stem}{r}"));
        }
        if let Some(stem) = name.strip_suffix(r) {
            return Some(format!("{stem}{l}"));
        }
    }
    // Conventions de type « mixamorig:LeftArm ».
    if name.contains("Left") {
        return Some(name.replacen("Left", "Right", 1));
    }
    if name.contains("Right") {
        return Some(name.replacen("Right", "Left", 1));
    }
    None
}

/// Pour chaque os, son symétrique (ou lui-même).
pub fn mirror_map(skeleton: &Skeleton) -> Vec<BoneId> {
    skeleton
        .ids()
        .map(|id| {
            mirror_name(&skeleton.bone(id).name)
                .and_then(|n| skeleton.find(&n))
                .unwrap_or(id)
        })
        .collect()
}

pub fn mirror_point(p: Vec3) -> Vec3 {
    Vec3::new(-p.x, p.y, p.z)
}

/// Symétrique d'une rotation par rapport au plan YZ (S·R·S avec S = diag(−1, 1, 1)).
pub fn mirror_rotation(q: Quat) -> Quat {
    Quat::from_xyzw(q.x, -q.y, -q.z, q.w)
}

pub fn mirror_transform(t: &Transform) -> Transform {
    Transform {
        translation: mirror_point(t.translation),
        rotation: mirror_rotation(t.rotation),
        scale: t.scale,
    }
}

/// Recopie un côté du squelette (pose de repos) sur l'autre. Les os
/// symétriques manquants sont créés. Retourne le nombre d'os créés.
pub fn symmetrize(skeleton: &mut Skeleton, from: Side) -> usize {
    let mut created = 0;
    // Création des os manquants, dans l'ordre (les parents d'abord).
    for id in skeleton.ids().collect::<Vec<_>>() {
        let bone = skeleton.bone(id);
        if side_of(&bone.name) != Some(from) {
            continue;
        }
        let Some(name) = mirror_name(&bone.name) else {
            continue;
        };
        if skeleton.find(&name).is_some() {
            continue;
        }
        let parent = bone.parent().map(|p| {
            mirror_name(&skeleton.bone(p).name)
                .and_then(|n| skeleton.find(&n))
                .unwrap_or(p)
        });
        let world = skeleton.rest_world();
        let target = mirror_transform(&world[id.index()]);
        let local = match parent {
            Some(p) => world[p.index()].inverse().mul(&target),
            None => target,
        };
        let length = bone.length;
        skeleton.add_bone(name, parent, local, length);
        created += 1;
    }

    // Recopie des positions et longueurs.
    let mut world = skeleton.rest_world();
    let map = mirror_map(skeleton);
    for id in skeleton.ids().collect::<Vec<_>>() {
        if side_of(&skeleton.bone(id).name) != Some(from) {
            continue;
        }
        let other = map[id.index()];
        if other == id {
            continue;
        }
        world[other.index()] = mirror_transform(&world[id.index()]);
        skeleton.bone_mut(other).length = skeleton.bone(id).length;
    }
    skeleton.set_rest_world(&world);
    created
}

/// Pose miroir : chaque os prend la pose symétrique de son homologue.
///
/// Fonctionne en espace monde et compense les écarts d'orientation de repos
/// entre les deux côtés (conventions de roulis différentes).
pub fn mirrored_pose(skeleton: &Skeleton, pose: &Pose) -> Pose {
    let map = mirror_map(skeleton);
    let rest = skeleton.rest_world();
    let posed = pose.world(skeleton);
    let mut out = Pose::rest(skeleton);
    let mut new_world: Vec<Transform> = Vec::with_capacity(skeleton.len());

    for id in skeleton.ids() {
        let i = id.index();
        let m = map[i].index();
        let bone = skeleton.bone(id);
        // Écart de repos entre l'os et le symétrique de son homologue.
        let correction = mirror_rotation(rest[m].rotation).inverse() * rest[i].rotation;
        let rotation = (mirror_rotation(posed[m].rotation) * correction).normalize();
        let translation =
            mirror_point(posed[m].translation) + (rest[i].translation - mirror_point(rest[m].translation));

        let parent_world = bone
            .parent()
            .map(|p| new_world[p.index()])
            .unwrap_or(Transform::IDENTITY);
        let basis = parent_world.mul(&bone.rest);
        let delta = Transform {
            translation: bone.rest.rotation.inverse()
                * (parent_world.inverse().transform_point(translation) - bone.rest.translation)
                / bone.rest.scale,
            rotation: (basis.rotation.inverse() * rotation).normalize(),
            scale: pose.get(map[i]).scale,
        };
        out.bones[i] = delta;
        new_world.push(basis.mul(&delta));
    }
    out
}
