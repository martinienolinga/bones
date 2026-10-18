//! Découpe d'un instantané pour l'écriture : document JSON allégé d'un côté,
//! données lourdes (géométrie, poids, poses-clés, textures) de l'autre.

use std::sync::Arc;

use super::codec::{PackedTransform, pack};
use super::document::{Counts, Document};
use super::finite::make_finite;
use super::objects::{Stripped, TexturePool, strip_objects};
use super::snapshot::ProjectSnapshot;
use crate::{Mesh, Skin, Texture, Transform};

/// Données écrites à part du document JSON.
pub(super) struct Heavy {
    /// Textures uniques (corps et objets), avec leur nom d'entrée.
    pub textures: Vec<(String, Texture)>,
    /// Poses des clés, `Counts::key_bones` transformations par clé.
    pub poses: Vec<PackedTransform>,
    /// Données lourdes des objets supplémentaires.
    pub objects: Stripped,
}

pub(super) fn split(snapshot: &ProjectSnapshot) -> (Document, Heavy) {
    let source = &snapshot.rig;
    // Déstructuration exhaustive : un champ ajouté au maillage ou aux poids
    // ne compile plus tant qu'il n'est pas pris en compte ici et à la lecture.
    let Mesh {
        positions,
        normals,
        indices,
        uvs,
        materials,
        submeshes,
    } = &*source.mesh;
    let Skin { weights } = &*source.skin;
    let mut pool = TexturePool::default();
    let (materials, material_textures) = pool.strip(materials);

    // Copie du document (les données lourdes y sont partagées, pas copiées),
    // dont on retire ce qui est écrit à part : tout autre champ, présent ou
    // futur, passe par serde.
    let mut rig = source.clone();
    rig.mesh = Arc::new(Mesh {
        materials,
        submeshes: submeshes.clone(),
        ..Mesh::default()
    });
    rig.skin = Arc::new(Skin::default());
    // Objets allégés dans la copie ; textures mises au même pool que le corps.
    let objects = strip_objects(rig.objects_mut(), &mut pool);
    let textures = pool.into_entries();
    let key_bones = source.skeleton.len();
    let poses = pack_poses(
        source.clip.keys().iter().map(|k| k.pose.bones.as_slice()),
        key_bones,
    );
    if !rig.clip.is_empty() {
        let clip = Arc::make_mut(&mut rig.clip);
        clip.poses_mut().for_each(|p| p.bones = Vec::new());
    }

    make_finite(&mut rig);
    let mut state = snapshot.state.clone();
    state.sanitize(&rig);

    let len = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    let document = Document {
        weights_bound: !source.needs_bind(),
        rig,
        counts: Counts {
            positions: len(positions.len()),
            normals: len(normals.len()),
            uvs: len(uvs.len()),
            indices: len(indices.len()),
            weights: len(weights.len()),
            key_bones: len(key_bones),
        },
        textures: textures.iter().map(|(n, _)| n.clone()).collect(),
        material_textures,
        objects: objects.counts.clone(),
        editor: state,
    };
    (
        document,
        Heavy {
            textures,
            poses,
            objects,
        },
    )
}

/// Poses des clés, chacune ramenée à exactement `bones` transformations.
fn pack_poses<'a>(
    poses: impl ExactSizeIterator<Item = &'a [Transform]>,
    bones: usize,
) -> Vec<PackedTransform> {
    let mut out = Vec::with_capacity(poses.len() * bones);
    for pose in poses {
        out.extend((0..bones).map(|i| pack(pose.get(i).unwrap_or(&Transform::IDENTITY))));
    }
    out
}
