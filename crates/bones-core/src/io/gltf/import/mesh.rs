//! Lecture des maillages, matériaux et textures glTF.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use glam::{Mat3, Mat4, Vec2, Vec3};

use super::{Scene, load_uri};
use crate::Mesh;
use crate::io::IoError;
use crate::mesh::{Material, Texture, sniff_mime};

/// Influences brutes par sommet : (indice d'articulation, poids).
pub(super) type RawWeights = Vec<Vec<(usize, f32)>>;

fn texture(image: gltf::Image, buffers: &[Arc<[u8]>], base: Option<&Path>) -> Option<Texture> {
    let (bytes, mime): (Vec<u8>, Option<&str>) = match image.source() {
        gltf::image::Source::View { view, mime_type } => {
            let data = buffers.get(view.buffer().index())?;
            (
                data.get(view.offset()..view.offset() + view.length())?.to_vec(),
                Some(mime_type),
            )
        }
        gltf::image::Source::Uri { uri, mime_type } => (load_uri(uri, base).ok()?, mime_type),
    };
    let mime = mime.unwrap_or_else(|| sniff_mime(&bytes)).to_owned();
    let name = image
        .name()
        .map(str::to_owned)
        .unwrap_or_else(|| format!("image_{}", image.index()));
    Some(Texture {
        name,
        mime,
        bytes: bytes.into(),
    })
}

pub(super) fn materials(doc: &gltf::Document, buffers: &[Arc<[u8]>], base: Option<&Path>) -> Vec<Material> {
    let mut images: HashMap<usize, Option<Texture>> = HashMap::new();
    doc.materials()
        .map(|m| {
            let pbr = m.pbr_metallic_roughness();
            let tex = pbr.base_color_texture().and_then(|info| {
                let image = info.texture().source();
                images
                    .entry(image.index())
                    .or_insert_with(|| texture(image, buffers, base))
                    .clone()
            });
            Material {
                name: m.name().map(str::to_owned).unwrap_or_else(|| "Material".into()),
                base_color: pbr.base_color_factor(),
                base_color_texture: tex,
                metallic: pbr.metallic_factor(),
                roughness: pbr.roughness_factor(),
            }
        })
        .collect()
}

/// Tous les maillages de la scène, fusionnés en un seul (en monde).
pub(super) fn read_meshes(
    doc: &gltf::Document,
    scene: &Scene,
    buffers: &[Arc<[u8]>],
    base: Option<&Path>,
    skin: Option<&gltf::Skin>,
    joint_nodes: &[usize],
    bind: Mat4,
) -> Result<(Mesh, RawWeights), IoError> {
    let nodes: Vec<gltf::Node> = doc.nodes().collect();
    let mut mesh = Mesh {
        materials: materials(doc, buffers, base),
        ..Default::default()
    };
    let mut raw: RawWeights = Vec::new();
    for &n in &scene.order {
        let node = &nodes[n];
        let skinned = skin.is_some() && node.skin().map(|s| s.index()) == skin.map(|s| s.index());
        let matrix = if skinned { bind } else { scene.world[n] };
        // Maillage rigide sous une articulation : lié à 100 % à celle-ci.
        let rigid_joint = (!skinned).then(|| nearest_joint(scene, n, joint_nodes)).flatten();
        read_node(node, buffers, skinned, matrix, rigid_joint, &mut mesh, &mut raw)?;
    }
    if mesh.indices.is_empty() && skin.is_none() {
        return Err(super::err("no triangulated mesh"));
    }
    mesh.shrink_to_fit();
    Ok((mesh, raw))
}

/// Ajoute à `mesh` les primitives triangulées d'un nœud, passées par
/// `matrix`, et leurs influences brutes à `raw` (une par sommet). Les
/// indices de matériaux sont ceux du fichier.
pub(super) fn read_node(
    node: &gltf::Node,
    buffers: &[Arc<[u8]>],
    skinned: bool,
    matrix: Mat4,
    rigid_joint: Option<usize>,
    mesh: &mut Mesh,
    raw: &mut RawWeights,
) -> Result<(), IoError> {
    let Some(gmesh) = node.mesh() else { return Ok(()) };
    for primitive in gmesh.primitives() {
        if primitive.mode() != gltf::mesh::Mode::Triangles {
            continue;
        }
        let reader = primitive.reader(|b| buffers.get(b.index()).map(|d| &d[..]));
        let Some(positions) = reader.read_positions() else {
            continue;
        };
        let mut part = Mesh {
            positions: positions.map(Vec3::from_array).collect(),
            ..Default::default()
        };
        let count = part.positions.len();
        part.indices = match reader.read_indices() {
            Some(i) => i.into_u32().collect(),
            None => (0..count as u32).collect(),
        };
        if part.indices.iter().any(|i| *i as usize >= count) {
            return Err(super::err("vertex index out of range"));
        }
        part.uvs = reader
            .read_tex_coords(0)
            .map(|t| t.into_f32().map(Vec2::from_array).collect())
            .unwrap_or_default();
        if part.uvs.len() != count {
            part.uvs.clear();
        }
        let normals: Vec<Vec3> = reader
            .read_normals()
            .map(|n| n.map(Vec3::from_array).collect())
            .unwrap_or_default();
        transform_part(&mut part, normals, matrix);

        let weights: Vec<Vec<(usize, f32)>> = match (skinned, reader.read_joints(0), reader.read_weights(0)) {
            (true, Some(j), Some(w)) => j
                .into_u16()
                .zip(w.into_f32())
                .map(|(j, w)| {
                    (0..4)
                        .filter(|k| w[*k] > 0.0)
                        .map(|k| (j[k] as usize, w[k]))
                        .collect()
                })
                .collect(),
            _ => vec![rigid_joint.map(|j| vec![(j, 1.0)]).unwrap_or_default(); count],
        };
        raw.extend(
            weights
                .into_iter()
                .chain(std::iter::repeat_with(Vec::new))
                .take(count),
        );
        mesh.append_part(part, primitive.material().index());
    }
    Ok(())
}

pub(super) fn nearest_joint(scene: &Scene, node: usize, joint_nodes: &[usize]) -> Option<usize> {
    let mut cur = scene.parent[node];
    while let Some(p) = cur {
        if let Some(j) = joint_nodes.iter().position(|n| *n == p) {
            return Some(j);
        }
        cur = scene.parent[p];
    }
    None
}

/// Passe la pièce en monde ; une matrice miroir inverse l'ordre des sommets.
fn transform_part(part: &mut Mesh, normals: Vec<Vec3>, m: Mat4) {
    for p in &mut part.positions {
        *p = m.transform_point3(*p);
    }
    if m.determinant() < 0.0 {
        for tri in part.indices.as_chunks_mut::<3>().0 {
            tri.swap(1, 2);
        }
    }
    if normals.len() == part.positions.len() {
        let nm = Mat3::from_mat4(m).inverse().transpose();
        part.normals = normals
            .into_iter()
            .map(|n| (nm * n).normalize_or(Vec3::Y))
            .collect();
    } else {
        part.compute_normals();
    }
}
