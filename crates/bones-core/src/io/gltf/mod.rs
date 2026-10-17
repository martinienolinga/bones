//! Export glTF 2.0 binaire (`.glb`) : maillage, squelette, poids et
//! animation, pour Blender, Unity, Unreal, Godot… (équivalent de l'export
//! « moteur de jeu » d'Auto-Rig Pro et de Cascadeur).
//!
//! L'animation est échantillonnée à chaque image (interpolation LINEAR),
//! ce qui reproduit fidèlement toutes les interpolations de l'éditeur.

mod animation;
mod buffer;
#[cfg(test)]
mod tests;

use glam::Mat4;
use serde_json::{Value, json};

use crate::{BoneId, Rig, Transform};
use animation::animation;
use buffer::{Builder, pack_glb};

pub(super) const ARRAY_BUFFER: u32 = 34962;
pub(super) const ELEMENT_ARRAY_BUFFER: u32 = 34963;
pub(super) const FLOAT: u32 = 5126;
pub(super) const UNSIGNED_SHORT: u32 = 5123;
pub(super) const UNSIGNED_INT: u32 = 5125;

/// Produit le fichier `.glb` complet.
pub fn export_glb(rig: &Rig) -> Vec<u8> {
    let mut b = Builder::default();
    let mesh = &rig.mesh;
    let skeleton = &rig.skeleton;
    let skinned = !skeleton.is_empty() && rig.skin.weights.len() == mesh.vertex_count();

    // Maillage.
    let positions = b.vec3s(&mesh.positions, Some(ARRAY_BUFFER), true);
    let normals = b.vec3s(&mesh.normals, Some(ARRAY_BUFFER), false);
    let indices_view = b.view(bytemuck::cast_slice(&mesh.indices), Some(ELEMENT_ARRAY_BUFFER));
    let indices = b.accessor(
        indices_view,
        UNSIGNED_INT,
        mesh.indices.len(),
        "SCALAR",
        json!({}),
    );
    let mut attributes = json!({ "POSITION": positions, "NORMAL": normals });
    if skinned {
        let joints: Vec<[u16; 4]> = rig.skin.weights.iter().map(|w| w.bones).collect();
        let weights: Vec<[u16; 4]> = rig.skin.weights.iter().map(|w| w.weights).collect();
        let jv = b.view(bytemuck::cast_slice(&joints), Some(ARRAY_BUFFER));
        let wv = b.view(bytemuck::cast_slice(&weights), Some(ARRAY_BUFFER));
        attributes["JOINTS_0"] = json!(b.accessor(jv, UNSIGNED_SHORT, joints.len(), "VEC4", json!({})));
        attributes["WEIGHTS_0"] = json!(b.accessor(
            wv,
            UNSIGNED_SHORT,
            weights.len(),
            "VEC4",
            json!({ "normalized": true })
        ));
    }

    // Nœuds : 0 = maillage, puis un nœud par os.
    let bone_node = |i: usize| i + 1;
    let mut nodes = vec![json!({ "name": rig.name, "mesh": 0 })];
    let mut roots = vec![0usize];
    for (i, bone) in skeleton.bones().iter().enumerate() {
        let local = bone.rest.mul(&rig.pose.get(BoneId(i as u32)));
        let children: Vec<usize> = skeleton
            .children(BoneId(i as u32))
            .map(|c| bone_node(c.index()))
            .collect();
        let mut node = trs_json(&local);
        node["name"] = json!(bone.name);
        if !children.is_empty() {
            node["children"] = json!(children);
        }
        nodes.push(node);
        if bone.parent().is_none() {
            roots.push(bone_node(i));
        }
    }

    let mut doc = json!({
        "asset": { "version": "2.0", "generator": "PaintSharp Bones" },
        "scene": 0,
        "scenes": [{ "nodes": roots }],
        "nodes": nodes,
        "meshes": [{
            "name": rig.name,
            "primitives": [{ "attributes": attributes, "indices": indices, "mode": 4 }]
        }],
    });

    if skinned {
        let ibm: Vec<Mat4> = skeleton
            .rest_world()
            .iter()
            .map(|w| w.to_matrix().inverse())
            .collect();
        let view = b.view(bytemuck::cast_slice(&ibm), None);
        let accessor = b.accessor(view, FLOAT, ibm.len(), "MAT4", json!({}));
        let joints: Vec<usize> = (0..skeleton.len()).map(bone_node).collect();
        doc["nodes"][0]["skin"] = json!(0);
        doc["skins"] = json!([{ "inverseBindMatrices": accessor, "joints": joints }]);
    }

    if skinned && rig.clip.keys().len() >= 2 {
        doc["animations"] = json!([animation(&mut b, rig, bone_node)]);
    }

    doc["buffers"] = json!([{ "byteLength": b.bin.len() }]);
    doc["bufferViews"] = json!(b.views);
    doc["accessors"] = json!(b.accessors);
    pack_glb(&doc, b.bin)
}

fn trs_json(t: &Transform) -> Value {
    json!({
        "translation": t.translation.to_array(),
        "rotation": t.rotation.normalize().to_array(),
        "scale": t.scale.to_array(),
    })
}
