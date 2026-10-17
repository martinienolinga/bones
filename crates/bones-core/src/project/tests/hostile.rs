//! Fichiers abîmés ou malveillants : toujours une erreur ou une réparation,
//! jamais de panique ni d'allocation démesurée.

use std::io::Write;
use std::sync::Arc;

use serde_json::json;

use super::super::container::{Archive, write_atomically};
use super::super::document::{MANIFEST, POSITIONS};
use super::super::{ProjectError, ProjectSnapshot, load, read_info};
use super::{TempDir, edit_document, full_editor, rewrite, save};
use crate::{BoneId, Editor, Rig, VertexWeights};

#[test]
fn failed_save_keeps_the_existing_project() {
    let dir = TempDir::new("atomic");
    let path = save(&full_editor(), &dir, "Safe");
    let before = std::fs::read(&path).unwrap();
    let result = write_atomically(&path, |mut file| {
        file.write_all(b"PK half written")?;
        Err(ProjectError::Invalid("simulated failure".into()))
    });
    assert!(result.is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(dir.files(), vec!["Safe.kbrig".to_string()]);
    assert!(load(&path).is_ok());
}

#[test]
fn saving_into_a_missing_directory_fails_cleanly() {
    let dir = TempDir::new("missing");
    let path = dir.join("nowhere/Project.kbrig");
    let snapshot = ProjectSnapshot::capture(&Editor::new(Rig::demo()));
    assert!(matches!(
        snapshot.save(&path, "Project", 0, None),
        Err(ProjectError::Io(_))
    ));
    assert!(dir.files().is_empty());
}

#[test]
fn oversized_entries_are_refused_at_save_time() {
    let dir = TempDir::new("oversized");
    let path = dir.join("Big.kbrig");
    let snapshot = ProjectSnapshot::capture(&Editor::new(Rig::demo()));
    let huge = vec![0u8; 17 << 20];
    let result = snapshot.save(&path, "Big", 0, Some(&huge));
    assert!(matches!(result, Err(ProjectError::TooLarge(_))));
    assert!(dir.files().is_empty());
}

#[test]
fn truncated_files_are_rejected() {
    let dir = TempDir::new("truncated");
    let path = save(&full_editor(), &dir, "Cut");
    let bytes = std::fs::read(&path).unwrap();
    let cut = dir.join("Cut short.kbrig");
    for len in [
        0,
        3,
        100,
        bytes.len() / 4,
        bytes.len() / 2,
        bytes.len() - 30,
        bytes.len() - 1,
    ] {
        std::fs::write(&cut, &bytes[..len]).unwrap();
        assert!(load(&cut).is_err(), "longueur {len}");
        assert!(read_info(&cut).is_err(), "longueur {len}");
    }
}

#[test]
fn corrupted_bytes_never_panic() {
    let dir = TempDir::new("corrupt");
    let path = save(&full_editor(), &dir, "Noise");
    let bytes = std::fs::read(&path).unwrap();
    let bad = dir.join("Bad.kbrig");
    let mut failures = 0;
    for i in (0..bytes.len()).step_by(bytes.len() / 150 + 1) {
        let mut copy = bytes.clone();
        copy[i] ^= 0x5A;
        std::fs::write(&bad, &copy).unwrap();
        failures += usize::from(load(&bad).is_err());
        let _ = read_info(&bad);
    }
    // Les données compressées sont protégées par leur somme CRC.
    assert!(failures > 50, "{failures} erreurs seulement");
}

#[test]
fn future_versions_are_rejected_with_a_clear_message() {
    let dir = TempDir::new("future");
    let path = save(&full_editor(), &dir, "Future");
    rewrite(&path, MANIFEST, |data| {
        let mut manifest: serde_json::Value = serde_json::from_slice(&data.unwrap()).unwrap();
        manifest["version"] = json!(99);
        Some(serde_json::to_vec(&manifest).unwrap())
    });
    let err = load(&path).err().unwrap();
    assert!(matches!(err, ProjectError::TooNew { version: 99, .. }), "{err}");
    assert!(err.to_string().contains("newer version"));
    assert!(matches!(read_info(&path), Err(ProjectError::TooNew { .. })));
}

#[test]
fn other_files_are_not_projects() {
    let dir = TempDir::new("other");
    let path = dir.join("Other.kbrig");
    std::fs::write(&path, b"just some text").unwrap();
    assert!(matches!(load(&path), Err(ProjectError::Invalid(_))));
    std::fs::write(&path, serde_json::to_vec(&Rig::demo()).unwrap()).unwrap();
    assert!(matches!(read_info(&path), Err(ProjectError::Invalid(_))));

    let path = save(&full_editor(), &dir, "No manifest");
    rewrite(&path, MANIFEST, |_| None);
    assert!(matches!(load(&path), Err(ProjectError::Invalid(_))));
}

#[test]
fn decompression_is_bounded() {
    let dir = TempDir::new("bomb");
    // Manifeste de 2 Mo d'espaces (quelques kilo-octets compressés).
    let path = save(&full_editor(), &dir, "Bomb");
    rewrite(&path, MANIFEST, |_| Some(vec![b' '; 2 << 20]));
    assert!(matches!(read_info(&path), Err(ProjectError::TooLarge(_))));

    // Tailles annoncées démesurées : refusées avant toute allocation.
    let path = save(&full_editor(), &dir, "Huge");
    edit_document(&path, |doc| doc["counts"]["positions"] = json!(u32::MAX));
    assert!(matches!(load(&path), Err(ProjectError::TooLarge(_))));
    edit_document(&path, |doc| doc["counts"]["key_bones"] = json!(u32::MAX));
    assert!(matches!(load(&path), Err(ProjectError::TooLarge(_))));

    // Lecture bornée d'une entrée, quoi qu'annonce le conteneur.
    let mut archive = Archive::open(&path).unwrap();
    assert!(matches!(
        archive.read(POSITIONS, 64),
        Err(ProjectError::TooLarge(_))
    ));
    assert!(archive.read("absent.bin", 64).unwrap().is_none());
}

#[test]
fn entry_sizes_must_match_the_document() {
    let dir = TempDir::new("sizes");
    let path = save(&full_editor(), &dir, "Sizes");
    rewrite(&path, POSITIONS, |data| data.map(|d| d[..d.len() - 12].to_vec()));
    assert!(matches!(load(&path), Err(ProjectError::Invalid(_))));
    rewrite(&path, POSITIONS, |_| None);
    assert!(matches!(load(&path), Err(ProjectError::Invalid(_))));
}

#[test]
fn out_of_range_indices_are_rejected() {
    let dir = TempDir::new("indices");
    let mut editor = Editor::new(Rig::demo());
    let n = editor.rig.mesh.vertex_count() as u32;
    Arc::make_mut(&mut editor.rig.mesh).indices[4] = n + 5;
    let path = save(&editor, &dir, "Indices");
    assert!(matches!(load(&path), Err(ProjectError::Invalid(_))));

    let path = save(&full_editor(), &dir, "Parts");
    edit_document(&path, |doc| {
        doc["rig"]["mesh"]["submeshes"][0]["indices"]["end"] = json!(u32::MAX)
    });
    assert!(matches!(load(&path), Err(ProjectError::Invalid(_))));
}

#[test]
fn invalid_weights_are_replaced_and_rebound() {
    let dir = TempDir::new("weights");
    let mut editor = Editor::new(Rig::demo());
    Arc::make_mut(&mut editor.rig.skin).weights[7] = VertexWeights::single(BoneId(999));
    let path = save(&editor, &dir, "Weights");
    let rig = load(&path).unwrap().snapshot.rig;
    assert!(rig.needs_bind());
    assert_eq!(rig.skin.weights.len(), rig.mesh.vertex_count());
    assert!(
        rig.skin
            .weights
            .iter()
            .all(|w| w.iter().all(|(b, _)| rig.skeleton.contains(b)))
    );
}

#[test]
fn inconsistent_documents_are_repaired() {
    let dir = TempDir::new("repair");
    let path = save(&full_editor(), &dir, "Repair");
    edit_document(&path, |doc| {
        let keys = doc["rig"]["clip"]["keys"].as_array_mut().unwrap();
        keys.reverse();
        doc["rig"]["clip"]["fps"] = json!(-1.0);
        doc["rig"]["pins"][0]["bone"] = json!(9999);
        doc["rig"]["limbs"][0]["lower"] = json!(9999);
        doc["editor"]["frame"] = json!(1e9);
        doc["editor"]["selected"] = json!(9999);
        doc["editor"]["chain_len"] = json!(100_000);
        doc["editor"]["camera"]["distance"] = json!(-3.0);
        doc["editor"]["manipulator"]["size"] = json!(0.0);
    });
    let loaded = load(&path).unwrap();
    let (rig, state) = (&loaded.snapshot.rig, &loaded.snapshot.state);
    let frames: Vec<u32> = rig.clip.keys().iter().map(|k| k.frame).collect();
    assert_eq!(frames, vec![0, 12]);
    assert_eq!(rig.clip.fps, 30.0);
    assert!(rig.pins.is_empty());
    assert_eq!(rig.limbs.len(), full_editor().rig.limbs.len() - 1);
    assert_eq!(state.frame, 47.0);
    assert_eq!(state.selected, None);
    assert_eq!(state.chain_len, 64);
    assert_eq!(state.camera, None);
    assert_eq!(state.manipulator.size, crate::ManipulatorSettings::default().size);
    loaded.snapshot.into_editor();
}

#[test]
fn bad_skeletons_are_rejected() {
    let dir = TempDir::new("skeleton");
    let path = save(&full_editor(), &dir, "Parents");
    edit_document(&path, |doc| doc["rig"]["skeleton"][0]["parent"] = json!(3));
    assert!(load(&path).is_err());

    let path = save(&full_editor(), &dir, "Lengths");
    edit_document(&path, |doc| doc["rig"]["skeleton"][1]["length"] = json!(null));
    assert!(load(&path).is_err());
}
