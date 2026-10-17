use super::obj::textured_demo;
use crate::Rig;

#[test]
fn bones_document_keeps_uvs_materials_and_textures() {
    let rig = textured_demo();
    let mut bytes = Vec::new();
    rig.save(&mut bytes).unwrap();
    let back = Rig::load(bytes.as_slice()).unwrap();
    assert_eq!(back.mesh.uvs, rig.mesh.uvs);
    assert_eq!(back.mesh.submeshes, rig.mesh.submeshes);
    assert_eq!(back.mesh.materials, rig.mesh.materials);
}

#[test]
fn old_documents_without_materials_still_load() {
    let rig = Rig::demo();
    let mut bytes = Vec::new();
    rig.save(&mut bytes).unwrap();
    let mut json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let mesh = json["rig"]["mesh"].as_object_mut().unwrap();
    for field in ["uvs", "materials", "submeshes"] {
        mesh.remove(field);
    }
    let back = Rig::load(serde_json::to_vec(&json).unwrap().as_slice()).unwrap();
    assert!(back.mesh.materials.is_empty() && back.mesh.uvs.is_empty());
}
