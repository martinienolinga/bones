use serde_json::Value;

use crate::Rig;
use crate::io::gltf::export_glb;

#[test]
fn glb_structure_is_valid() {
    let mut rig = Rig::demo();
    let pose = rig.pose.clone();
    rig.clip_mut().set_key(0, pose);
    let mut raised = rig.pose.clone();
    raised.bones[0].translation.y += 0.3;
    rig.clip_mut().set_key(10, raised);

    let glb = export_glb(&rig);
    assert_eq!(&glb[0..4], b"glTF");
    assert_eq!(
        u32::from_le_bytes(glb[8..12].try_into().unwrap()) as usize,
        glb.len()
    );
    let json_len = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
    assert_eq!(&glb[16..20], b"JSON");
    let doc: Value = serde_json::from_slice(&glb[20..20 + json_len]).unwrap();
    assert_eq!(doc["nodes"].as_array().unwrap().len(), rig.skeleton.len() + 1);
    assert_eq!(
        doc["skins"][0]["joints"].as_array().unwrap().len(),
        rig.skeleton.len()
    );
    let channels = doc["animations"][0]["channels"].as_array().unwrap();
    assert_eq!(channels.len(), rig.skeleton.len() * 3);

    // Toutes les vues tiennent dans le tampon binaire.
    let bin_len = doc["buffers"][0]["byteLength"].as_u64().unwrap();
    for v in doc["bufferViews"].as_array().unwrap() {
        let end = v["byteOffset"].as_u64().unwrap() + v["byteLength"].as_u64().unwrap();
        assert!(end <= bin_len);
        assert_eq!(v["byteOffset"].as_u64().unwrap() % 4, 0);
    }
}

fn animated(mut rig: Rig) -> Rig {
    let pose = rig.pose.clone();
    rig.clip_mut().set_key(0, pose);
    let mut moved = rig.pose.clone();
    moved.bones[0].translation.y += 0.25;
    moved.bones[5].rotation = glam::Quat::from_rotation_z(0.7);
    rig.clip_mut().set_key(12, moved);
    rig
}

fn imported(format: crate::io::Format, bytes: &[u8], base: Option<&std::path::Path>) -> Rig {
    match crate::io::import_bytes(format, bytes, "retour", base).unwrap() {
        crate::io::Imported::Rig(rig) => rig,
        crate::io::Imported::Mesh(_) => panic!("document attendu"),
    }
}

/// Têtes des os (par nom) à une image donnée.
fn heads(rig: &Rig, frame: f32) -> Vec<(String, glam::Vec3)> {
    let mut pose = rig.pose.clone();
    rig.clip.sample_into(frame, &mut pose);
    let w = pose.world(&rig.skeleton);
    rig.skeleton
        .bones()
        .iter()
        .zip(&w)
        .map(|(b, w)| (b.name.clone(), w.translation))
        .collect()
}

fn assert_same_motion(a: &Rig, b: &Rig) {
    for frame in [0.0, 6.0, 12.0] {
        let hb = heads(b, frame);
        for (name, p) in heads(a, frame) {
            let q = hb
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, q)| *q)
                .expect("os présent");
            assert!(p.abs_diff_eq(q, 1e-3), "{name} image {frame} : {p:?} / {q:?}");
        }
    }
}

#[test]
fn glb_roundtrip_keeps_rig_textures_and_animation() {
    let rig = animated(super::obj::textured_demo());
    let glb = export_glb(&rig);
    let back = imported(crate::io::Format::Glb, &glb, None);
    assert_eq!(back.skeleton.len(), rig.skeleton.len());
    assert_eq!(back.mesh.vertex_count(), rig.mesh.vertex_count());
    assert!(back.mesh.has_uvs());
    assert!(!back.needs_bind(), "les poids du fichier sont utilisés");
    let tex = back.mesh.materials[0].base_color_texture.as_ref().unwrap();
    assert_eq!(&*tex.bytes, super::tiny_png().as_slice());
    assert_eq!(back.mesh.parts()[0].material, Some(0));
    // Poids identiques (mêmes os, quantification 16 bits conservée).
    for (a, b) in rig.skin.weights.iter().zip(&back.skin.weights) {
        for (bone, w) in a.iter() {
            let name = &rig.skeleton.bone(bone).name;
            let other = back.skeleton.find(name).unwrap();
            assert!((b.weight_of(other) - w).abs() < 1e-3);
        }
    }
    assert_same_motion(&rig, &back);
}

#[test]
fn gltf_text_export_writes_bin_and_images() {
    let dir = super::temp_dir("gltf-text");
    let rig = animated(super::obj::textured_demo());
    let path = dir.join("mannequin.gltf");
    let files = crate::io::export(&rig, crate::io::Format::Gltf, &path).unwrap();
    assert_eq!(files.len(), 3, "{files:?}");
    let back = match crate::io::import(&path).unwrap() {
        crate::io::Imported::Rig(r) => r,
        crate::io::Imported::Mesh(_) => panic!("document attendu"),
    };
    assert!(back.mesh.materials[0].base_color_texture.is_some());
    assert_same_motion(&rig, &back);
    std::fs::remove_dir_all(&dir).unwrap();
}
