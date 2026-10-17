//! glTF 2.0 : import (`.gltf`, `.glb`) et export binaire ou texte avec
//! maillage, coordonnées de texture, matériaux, textures, squelette, poids
//! et animation (équivalent de l'export « moteur de jeu » d'Auto-Rig Pro et
//! de Cascadeur).
//!
//! L'animation est échantillonnée à chaque image (interpolation LINEAR),
//! ce qui reproduit fidèlement toutes les interpolations de l'éditeur.

mod animation;
mod buffer;
mod import;
mod materials;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use glam::Mat4;
use serde_json::{Value, json};

pub(crate) use import::{animations, import};

use crate::io::{IoError, safe_file_name};
use crate::{BoneId, Rig, Transform};
use animation::animation;
use buffer::{Builder, pack_glb};

pub(super) const ARRAY_BUFFER: u32 = 34962;
pub(super) const ELEMENT_ARRAY_BUFFER: u32 = 34963;
pub(super) const FLOAT: u32 = 5126;
pub(super) const UNSIGNED_SHORT: u32 = 5123;
pub(super) const UNSIGNED_INT: u32 = 5125;

/// Produit le fichier `.glb` complet (textures incluses).
pub fn export_glb(rig: &Rig) -> Vec<u8> {
    let (doc, b, _) = document(rig, None);
    pack_glb(&doc, b.bin)
}

/// Écrit `nom.gltf`, `nom.bin` et les textures à côté.
pub fn export_gltf(rig: &Rig, path: &Path) -> Result<Vec<PathBuf>, IoError> {
    let dir = path.parent().unwrap_or(Path::new("."));
    let stem = path
        .file_stem()
        .map(|s| safe_file_name(&s.to_string_lossy(), "modele"))
        .unwrap_or_else(|| "modele".into());
    let (mut doc, b, images) = document(rig, Some(&stem));
    let bin_name = format!("{stem}.bin");
    doc["buffers"][0]["uri"] = json!(bin_name);
    let mut written = vec![path.to_path_buf(), dir.join(&bin_name)];
    std::fs::write(&written[1], &b.bin)?;
    for (file, bytes) in images {
        let p = dir.join(file);
        std::fs::write(&p, &bytes)?;
        written.push(p);
    }
    let json = serde_json::to_vec_pretty(&doc).map_err(std::io::Error::other)?;
    std::fs::write(path, json)?;
    Ok(written)
}

/// Images à écrire à côté d'un `.gltf` : (nom de fichier, octets).
type ExternalImages = Vec<(String, Arc<[u8]>)>;

/// Document glTF et tampon binaire. Avec `external = Some(nom)`, les images
/// sont des fichiers séparés (retournés) ; sinon elles sont dans le tampon.
fn document(rig: &Rig, external: Option<&str>) -> (Value, Builder, ExternalImages) {
    let mut b = Builder::default();
    let mesh = &rig.mesh;
    let skeleton = &rig.skeleton;
    let skinned = !skeleton.is_empty() && rig.skin.weights.len() == mesh.vertex_count();

    let positions = b.vec3s(&mesh.positions, Some(ARRAY_BUFFER), true);
    let normals = b.vec3s(&mesh.normals, Some(ARRAY_BUFFER), false);
    let mut attributes = json!({ "POSITION": positions, "NORMAL": normals });
    if mesh.has_uvs() {
        let view = b.view(bytemuck::cast_slice(&mesh.uvs), Some(ARRAY_BUFFER));
        attributes["TEXCOORD_0"] = json!(b.accessor(view, FLOAT, mesh.uvs.len(), "VEC2", json!({})));
    }
    if skinned {
        let joints: Vec<[u16; 4]> = rig.skin.weights.iter().map(|w| w.bones).collect();
        let weights: Vec<[u16; 4]> = rig.skin.weights.iter().map(|w| w.weights).collect();
        let jv = b.view(bytemuck::cast_slice(&joints), Some(ARRAY_BUFFER));
        let wv = b.view(bytemuck::cast_slice(&weights), Some(ARRAY_BUFFER));
        attributes["JOINTS_0"] = json!(b.accessor(jv, UNSIGNED_SHORT, joints.len(), "VEC4", json!({})));
        let normalized = json!({ "normalized": true });
        attributes["WEIGHTS_0"] = json!(b.accessor(wv, UNSIGNED_SHORT, weights.len(), "VEC4", normalized));
    }

    // Une primitive par partie : les attributs sont partagés, chaque
    // primitive lit sa plage d'indices dans la même vue.
    let indices_view = b.view(bytemuck::cast_slice(&mesh.indices), Some(ELEMENT_ARRAY_BUFFER));
    let primitives: Vec<Value> = mesh
        .parts()
        .iter()
        .filter(|p| !p.indices.is_empty())
        .map(|part| {
            let count = (part.indices.end - part.indices.start) as usize;
            let offset = json!({ "byteOffset": part.indices.start as usize * 4 });
            let indices = b.accessor(indices_view, UNSIGNED_INT, count, "SCALAR", offset);
            let mut p = json!({ "attributes": attributes, "indices": indices, "mode": 4 });
            if let Some(m) = part.material {
                p["material"] = json!(m);
            }
            p
        })
        .collect();

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
        "meshes": [{ "name": rig.name, "primitives": primitives }],
    });

    let images = materials::write(&mut b, &mut doc, mesh, external);

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
    if !skeleton.is_empty() && rig.clip.keys().len() >= 2 {
        doc["animations"] = json!([animation(&mut b, rig, bone_node)]);
    }

    doc["buffers"] = json!([{ "byteLength": b.bin.len() }]);
    doc["bufferViews"] = json!(b.views);
    doc["accessors"] = json!(b.accessors);
    (doc, b, images)
}

fn trs_json(t: &Transform) -> Value {
    json!({
        "translation": t.translation.to_array(),
        "rotation": t.rotation.normalize().to_array(),
        "scale": t.scale.to_array(),
    })
}
