//! Toutes les animations d'un fichier glTF/GLB sur son squelette (une
//! animation par préréglage importé). Sans peau, la hiérarchie des nœuds
//! de la scène sert de squelette.

use std::path::Path;
use std::sync::Arc;

use super::{anim, err, load_uri, scene, skin_joints};
use crate::io::IoError;
use crate::io::rigging::{Joint, build_skeleton};
use crate::{Clip, Skeleton};

/// Squelette et animations nommées.
// Utilisé par l'import de préréglages, encore en pause (wip/LISEZMOI.md).
#[allow(dead_code)]
pub(crate) fn animations(
    bytes: &[u8],
    base: Option<&Path>,
) -> Result<(Skeleton, Vec<(String, Clip)>), IoError> {
    let gltf = gltf::Gltf::from_slice(bytes).map_err(err)?;
    let doc = &gltf.document;
    let mut blob = gltf.blob.clone();
    let buffers: Vec<Arc<[u8]>> = doc
        .buffers()
        .map(|b| match b.source() {
            gltf::buffer::Source::Bin => blob
                .take()
                .map(Arc::from)
                .ok_or_else(|| err("missing GLB buffer")),
            gltf::buffer::Source::Uri(uri) => load_uri(uri, base).map(Arc::from),
        })
        .collect::<Result<_, _>>()?;
    let scene = scene(doc);
    let skin = scene
        .order
        .iter()
        .filter_map(|i| doc.nodes().nth(*i)?.skin())
        .next()
        .or_else(|| doc.skins().next());
    let (joints, joint_nodes): (Vec<Joint>, Vec<usize>) = match &skin {
        Some(skin) => (
            skin_joints(skin, &scene, &buffers).joints,
            skin.joints().map(|j| j.index()).collect(),
        ),
        None => {
            // Hiérarchie de nœuds animés sans peau.
            let nodes: Vec<usize> = scene.order.clone();
            let index = |n: usize| nodes.iter().position(|x| *x == n);
            let joints = nodes
                .iter()
                .map(|&n| Joint {
                    name: doc
                        .nodes()
                        .nth(n)
                        .and_then(|node| node.name().map(str::to_owned))
                        .unwrap_or_else(|| format!("node_{n}")),
                    parent: scene.parent[n].and_then(index),
                    world: scene.world[n],
                    leaf_tail: None,
                })
                .collect();
            (joints, nodes)
        }
    };
    if joints.is_empty() {
        return Err(err("no skeleton"));
    }
    let built = build_skeleton(&joints);
    let clips = anim::sample_all(doc, &buffers, &scene, &joint_nodes, &built);
    Ok((built.skeleton, clips))
}
