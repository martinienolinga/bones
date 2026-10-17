use super::*;

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
