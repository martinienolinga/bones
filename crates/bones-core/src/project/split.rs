//! Découpe d'un instantané pour l'écriture : document JSON allégé d'un côté,
//! données lourdes (géométrie, poids, poses-clés, textures) de l'autre.

use std::sync::Arc;

use super::codec::{PackedTransform, pack};
use super::document::{Counts, Document, TEXTURES_DIR};
use super::snapshot::ProjectSnapshot;
use crate::{Material, Mesh, Skin, Texture, Transform};

/// Données écrites à part du document JSON.
pub(super) struct Heavy {
    /// Textures uniques, avec leur nom d'entrée.
    pub textures: Vec<(String, Texture)>,
    /// Poses des clés, `Counts::key_bones` transformations par clé.
    pub poses: Vec<PackedTransform>,
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
    let (materials, material_textures, textures) = strip_textures(materials);

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
    let key_bones = source.skeleton.len();
    let poses = pack_poses(
        source.clip.keys().iter().map(|k| k.pose.bones.as_slice()),
        key_bones,
    );
    if !rig.clip.is_empty() {
        let clip = Arc::make_mut(&mut rig.clip);
        clip.poses_mut().for_each(|p| p.bones = Vec::new());
    }

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
        editor: snapshot.state.clone(),
    };
    (document, Heavy { textures, poses })
}

/// Matériaux sans les octets de leurs textures, indice de texture de chaque
/// matériau, et textures uniques (une texture partagée n'est écrite qu'une fois).
#[allow(clippy::type_complexity)]
fn strip_textures(materials: &[Material]) -> (Vec<Material>, Vec<Option<u32>>, Vec<(String, Texture)>) {
    let mut textures: Vec<(String, Texture)> = Vec::new();
    let mut indices = Vec::with_capacity(materials.len());
    let stripped = materials
        .iter()
        .map(|m| {
            let mut m = m.clone();
            let index = m.base_color_texture.as_mut().map(|t| {
                let found = textures.iter().position(|(_, u)| Arc::ptr_eq(&u.bytes, &t.bytes));
                let i = found.unwrap_or_else(|| {
                    let name = format!("{TEXTURES_DIR}{}.{}", textures.len(), t.extension());
                    textures.push((name, t.clone()));
                    textures.len() - 1
                });
                t.bytes = Arc::from([]);
                i as u32
            });
            indices.push(index);
            m
        })
        .collect();
    (stripped, indices, textures)
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
