//! Import glTF 2.0 (`.gltf` et `.glb`) : maillages, matériaux et textures,
//! squelette (peau), poids et animation.
//!
//! Espace de liaison : les sommets d'un maillage skinné sont exprimés dans
//! l'espace de la peau ; `B = G(racine) · IBM(racine)` les ramène en monde.
//! Les os sont liés en `B · IBM⁻¹`, et l'animation passe par les matrices
//! monde des nœuds, ce qui reste exact même si la pose de repos des nœuds
//! diffère de la pose de liaison.

mod anim;
mod mesh;

use std::path::Path;
use std::sync::Arc;

use base64::Engine;
use glam::Mat4;

use crate::Rig;
use crate::io::rigging::{Joint, assemble, build_skeleton, weights_from_joints};
use crate::io::{Format, IoError};

pub(super) fn err(message: impl std::fmt::Display) -> IoError {
    IoError::parse(Format::Gltf, message)
}

/// Contenu d'une URI : `data:` (base64) ou fichier relatif au modèle.
pub(super) fn load_uri(uri: &str, base: Option<&Path>) -> Result<Vec<u8>, IoError> {
    if let Some(rest) = uri.strip_prefix("data:") {
        let data = rest
            .split_once(";base64,")
            .map(|(_, d)| d)
            .ok_or_else(|| err("URI data non base64"))?;
        return base64::engine::general_purpose::STANDARD
            .decode(data)
            .map_err(|e| err(format!("base64 : {e}")));
    }
    let base = base.ok_or_else(|| err(format!("fichier externe « {uri} » sans dossier")))?;
    let decoded = percent_decode(uri);
    Ok(std::fs::read(base.join(decoded))?)
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(v) = s.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok())
        {
            out.push(v);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Graphe de scène aplati : parent et matrice monde de chaque nœud.
pub(super) struct Scene {
    pub parent: Vec<Option<usize>>,
    pub world: Vec<Mat4>,
    /// Nœuds de la scène, parents avant enfants.
    pub order: Vec<usize>,
}

fn scene(doc: &gltf::Document) -> Scene {
    let n = doc.nodes().len();
    let mut s = Scene {
        parent: vec![None; n],
        world: vec![Mat4::IDENTITY; n],
        order: Vec::with_capacity(n),
    };
    let roots: Vec<gltf::Node> = match doc.default_scene().or_else(|| doc.scenes().next()) {
        Some(scene) => scene.nodes().collect(),
        None => doc
            .nodes()
            .filter(|n| !doc.nodes().any(|p| p.children().any(|c| c.index() == n.index())))
            .collect(),
    };
    let mut stack: Vec<(gltf::Node, Mat4)> = roots.into_iter().rev().map(|r| (r, Mat4::IDENTITY)).collect();
    while let Some((node, parent_world)) = stack.pop() {
        let world = parent_world * Mat4::from_cols_array_2d(&node.transform().matrix());
        s.world[node.index()] = world;
        s.order.push(node.index());
        for child in node.children() {
            s.parent[child.index()] = Some(node.index());
            stack.push((child, world));
        }
    }
    s
}

pub(crate) fn import(bytes: &[u8], name: &str, base: Option<&Path>) -> Result<Rig, IoError> {
    let gltf = gltf::Gltf::from_slice(bytes).map_err(err)?;
    let doc = &gltf.document;
    let mut blob = gltf.blob.clone();
    let buffers: Vec<Arc<[u8]>> = doc
        .buffers()
        .map(|b| match b.source() {
            gltf::buffer::Source::Bin => blob.take().map(Arc::from).ok_or_else(|| err("tampon GLB absent")),
            gltf::buffer::Source::Uri(uri) => load_uri(uri, base).map(Arc::from),
        })
        .collect::<Result<_, _>>()?;
    let scene = scene(doc);

    // Peau retenue : la première utilisée par un maillage de la scène.
    let skin = scene
        .order
        .iter()
        .filter_map(|i| doc.nodes().nth(*i)?.skin())
        .next()
        .or_else(|| doc.skins().next());

    let rig_data = skin.as_ref().map(|skin| skin_joints(skin, &scene, &buffers));
    let joint_nodes: Vec<usize> = skin.iter().flat_map(|s| s.joints().map(|j| j.index())).collect();
    let bind = rig_data.as_ref().map(|d| d.bind).unwrap_or(Mat4::IDENTITY);

    let (mesh, raw_weights) =
        mesh::read_meshes(doc, &scene, &buffers, base, skin.as_ref(), &joint_nodes, bind)?;
    let Some(data) = rig_data else {
        return Ok(assemble(name, mesh, None, None, None));
    };
    let built = build_skeleton(&data.joints);
    let weights = weights_from_joints(&built, &raw_weights);
    let clip = anim::sample(doc, &buffers, &scene, &joint_nodes, &built);
    Ok(assemble(name, mesh, Some(built), Some(weights), clip))
}

pub(super) struct SkinData {
    pub joints: Vec<Joint>,
    /// Matrice espace de la peau → monde.
    pub bind: Mat4,
}

fn skin_joints(skin: &gltf::Skin, scene: &Scene, buffers: &[Arc<[u8]>]) -> SkinData {
    let nodes: Vec<gltf::Node> = skin.joints().collect();
    let reader = skin.reader(|b| buffers.get(b.index()).map(|d| &d[..]));
    let ibms: Vec<Mat4> = reader
        .read_inverse_bind_matrices()
        .map(|it| it.map(|m| Mat4::from_cols_array_2d(&m)).collect())
        .unwrap_or_else(|| nodes.iter().map(|n| scene.world[n.index()].inverse()).collect());
    let index_of = |node: usize| nodes.iter().position(|n| n.index() == node);
    let parent_joint = |node: usize| {
        let mut cur = scene.parent[node];
        while let Some(p) = cur {
            if let Some(j) = index_of(p) {
                return Some(j);
            }
            cur = scene.parent[p];
        }
        None
    };
    // Racine de la peau : première articulation sans parent articulation.
    let root = (0..nodes.len())
        .find(|j| parent_joint(nodes[*j].index()).is_none())
        .unwrap_or(0);
    let bind = scene.world[nodes[root].index()] * ibms.get(root).copied().unwrap_or(Mat4::IDENTITY);
    let joints = nodes
        .iter()
        .enumerate()
        .map(|(j, node)| Joint {
            name: node
                .name()
                .map(str::to_owned)
                .unwrap_or_else(|| format!("os_{j}")),
            parent: parent_joint(node.index()),
            world: bind * ibms.get(j).copied().unwrap_or(Mat4::IDENTITY).inverse(),
            leaf_tail: None,
        })
        .collect();
    SkinData { joints, bind }
}
