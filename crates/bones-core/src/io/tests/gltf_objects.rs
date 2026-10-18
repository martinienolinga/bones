//! Import glTF « en objets séparés » : un objet par nœud maillé, hiérarchie
//! des nœuds conservée, maillage rigide sous une articulation rattaché à
//! l'os, maillage skinné gardant ses poids.

use base64::Engine;
use serde_json::{Value, json};

use crate::io::{Format, ImportedParent, SceneImport, import_scene_bytes};
use crate::scene::Parent;

/// Ajoute `data` au tampon (aligné sur 4 octets) et retourne (offset, longueur).
fn push(bin: &mut Vec<u8>, data: &[u8]) -> (usize, usize) {
    while !bin.len().is_multiple_of(4) {
        bin.push(0);
    }
    let offset = bin.len();
    bin.extend_from_slice(data);
    (offset, data.len())
}

fn floats(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|f| f.to_le_bytes()).collect()
}

/// glTF texte minimal : deux articulations, un maillage skinné (triangle lié
/// à l'articulation racine) et un maillage rigide enfant de la seconde
/// articulation.
fn rigged_scene() -> Vec<u8> {
    let mut bin = Vec::new();
    let tri = floats(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0]);
    let (p0, l0) = push(&mut bin, &tri);
    let idx: Vec<u8> = [0u32, 1, 2].iter().flat_map(|i| i.to_le_bytes()).collect();
    let (i0, il) = push(&mut bin, &idx);
    let joints: Vec<u8> = (0..3)
        .flat_map(|_| {
            [0u16, 0, 0, 0]
                .iter()
                .flat_map(|j| j.to_le_bytes())
                .collect::<Vec<_>>()
        })
        .collect();
    let (j0, jl) = push(&mut bin, &joints);
    let weights: Vec<u8> = (0..3).flat_map(|_| floats(&[1.0, 0.0, 0.0, 0.0])).collect();
    let (w0, wl) = push(&mut bin, &weights);
    let tri2 = floats(&[0.0, 0.0, 0.0, 0.5, 0.0, 0.0, 0.0, 0.5, 0.0]);
    let (p1, l1) = push(&mut bin, &tri2);
    let (i1, il1) = push(&mut bin, &idx);
    // Matrices de liaison inverses : identité et translation (0,-1,0).
    let mut ibm = floats(&glam::Mat4::IDENTITY.to_cols_array());
    ibm.extend(floats(
        &glam::Mat4::from_translation(glam::Vec3::new(0.0, -1.0, 0.0)).to_cols_array(),
    ));
    let (m0, ml) = push(&mut bin, &ibm);

    let uri = format!(
        "data:application/octet-stream;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&bin)
    );
    let doc: Value = json!({
        "asset": { "version": "2.0" },
        "scene": 0,
        "scenes": [{ "nodes": [0, 2] }],
        "nodes": [
            { "name": "root", "children": [1] },
            { "name": "tip", "translation": [0.0, 1.0, 0.0], "children": [3] },
            { "name": "SkinnedMesh", "mesh": 0, "skin": 0 },
            { "name": "Prop", "mesh": 1 }
        ],
        "skins": [{ "joints": [0, 1], "inverseBindMatrices": 6 }],
        "meshes": [
            { "primitives": [{ "attributes": { "POSITION": 0, "JOINTS_0": 2, "WEIGHTS_0": 3 }, "indices": 1 }] },
            { "primitives": [{ "attributes": { "POSITION": 4 }, "indices": 5 }] }
        ],
        "accessors": [
            { "bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0.0, 0.0, 0.0], "max": [1.0, 1.0, 0.0] },
            { "bufferView": 1, "componentType": 5125, "count": 3, "type": "SCALAR" },
            { "bufferView": 2, "componentType": 5123, "count": 3, "type": "VEC4" },
            { "bufferView": 3, "componentType": 5126, "count": 3, "type": "VEC4" },
            { "bufferView": 4, "componentType": 5126, "count": 3, "type": "VEC3", "min": [0.0, 0.0, 0.0], "max": [0.5, 0.5, 0.0] },
            { "bufferView": 5, "componentType": 5125, "count": 3, "type": "SCALAR" },
            { "bufferView": 6, "componentType": 5126, "count": 2, "type": "MAT4" }
        ],
        "bufferViews": [
            { "buffer": 0, "byteOffset": p0, "byteLength": l0 },
            { "buffer": 0, "byteOffset": i0, "byteLength": il },
            { "buffer": 0, "byteOffset": j0, "byteLength": jl },
            { "buffer": 0, "byteOffset": w0, "byteLength": wl },
            { "buffer": 0, "byteOffset": p1, "byteLength": l1 },
            { "buffer": 0, "byteOffset": i1, "byteLength": il1 },
            { "buffer": 0, "byteOffset": m0, "byteLength": ml }
        ],
        "buffers": [{ "uri": uri, "byteLength": bin.len() }]
    });
    serde_json::to_vec(&doc).unwrap()
}

fn scene() -> SceneImport {
    import_scene_bytes(Format::Gltf, &rigged_scene(), "Rigged", None).unwrap()
}

#[test]
fn imports_one_object_per_mesh_with_hierarchy() {
    let s = scene();
    assert_eq!(s.objects.len(), 2, "un objet par maillage");
    let skeleton = s.skeleton.as_ref().expect("squelette importé");
    assert_eq!(skeleton.len(), 2);
    // Le maillage rigide « Prop » est rattaché à l'articulation « tip ».
    let prop = s.objects.iter().find(|o| o.name == "Prop").unwrap();
    assert!(matches!(prop.parent, ImportedParent::Joint(_)));
    // Le maillage skinné garde ses poids.
    let skinned = s.objects.iter().find(|o| o.name == "SkinnedMesh").unwrap();
    assert!(
        skinned
            .weights
            .as_ref()
            .is_some_and(|w| w.iter().any(|v| v.weights[0] > 0))
    );
}

#[test]
fn into_rig_makes_body_and_extra_objects() {
    let rig = scene().into_rig();
    assert_eq!(rig.skeleton.len(), 2);
    // Le maillage skinné devient le corps ; le rigide reste un objet attaché
    // à un os.
    assert!(rig.mesh.triangle_count() > 0);
    assert_eq!(rig.objects().len(), 1);
    assert!(matches!(rig.objects()[0].parent(), Parent::Bone(_)));
}

#[test]
fn added_to_a_project_finds_bones_by_name() {
    use crate::{Editor, Rig};
    let mut editor = Editor::new(Rig::demo());
    // Un squelette importé dont un os porte un nom du mannequin : l'objet
    // attaché le retrouve.
    let mut s = scene();
    if let Some(sk) = &mut s.skeleton {
        let head = crate::Skeleton::find(sk, "tip");
        if let Some(h) = head {
            sk.bone_mut(h).name = "head.x".into();
        }
    }
    let before = editor.rig.objects().len();
    let ids = editor.add_imported_objects(s).unwrap();
    assert_eq!(editor.rig.objects().len(), before + ids.len());
    assert!(!ids.is_empty());
}

#[test]
fn default_import_still_merges_into_one_body() {
    // L'import « personnage » classique fusionne tout dans le corps.
    let rig = match crate::io::import_bytes(Format::Gltf, &rigged_scene(), "Merged", None).unwrap() {
        crate::io::Imported::Rig(rig) => rig,
        crate::io::Imported::Mesh(_) => panic!("document attendu"),
    };
    assert!(rig.objects().is_empty(), "import classique : aucun objet séparé");
    assert!(rig.mesh.triangle_count() >= 2, "les maillages sont fusionnés");
}
