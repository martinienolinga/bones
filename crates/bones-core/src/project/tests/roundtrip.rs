//! Allers-retours : tout ce qui constitue un projet revient à l'identique.

use std::sync::Arc;

use serde_json::Value;

use super::super::state::EditorState;
use super::super::{ProjectSnapshot, decode_thumbnail, encode_thumbnail, load, load_bytes, read_info};
use super::{TempDir, edit_document, full_editor, save, thumbnail};
use crate::editor::Mode;
use crate::{Editor, Mesh, Rig};

/// Chaque champ de `value` (objet JSON) diffère de celui de `default` : le
/// test couvre ainsi tout champ ajouté plus tard, qu'il faudra renseigner ici.
fn assert_all_fields_set(value: &Value, default: &Value, what: &str) {
    let (Value::Object(fields), Value::Object(defaults)) = (value, default) else {
        panic!("{what} : objet attendu");
    };
    for (key, v) in fields {
        assert_ne!(Some(v), defaults.get(key), "{what}.{key} a sa valeur par défaut");
    }
}

#[test]
fn full_project_round_trips_every_field() {
    let dir = TempDir::new("full");
    let editor = full_editor();
    let rig_json = serde_json::to_value(&editor.rig).unwrap();
    let default_rig = Rig::new("", Mesh::default());
    assert_all_fields_set(&rig_json, &serde_json::to_value(&default_rig).unwrap(), "Rig");
    assert_all_fields_set(
        &rig_json["mesh"],
        &serde_json::to_value(Mesh::default()).unwrap(),
        "Mesh",
    );
    let state = EditorState::capture(&editor);
    assert_all_fields_set(
        &serde_json::to_value(&state).unwrap(),
        &serde_json::to_value(EditorState::default()).unwrap(),
        "EditorState",
    );
    assert!(!editor.rig.needs_bind());

    let path = save(&editor, &dir, "Hero walk");
    let loaded = load(&path).unwrap();
    assert_eq!(serde_json::to_value(&loaded.snapshot.rig).unwrap(), rig_json);
    assert_eq!(loaded.snapshot.state, state);
    assert!(!loaded.snapshot.rig.needs_bind());
    assert_eq!(loaded.thumbnail, Some(thumbnail()));

    let info = &loaded.info;
    assert_eq!(info.name, "Hero walk");
    assert_eq!(info.created, 1_700_000_000);
    assert!(info.modified >= info.created);
    assert_eq!(info.bones as usize, editor.rig.skeleton.len());
    assert_eq!(info.vertices as usize, editor.rig.mesh.vertex_count());
    assert_eq!(info.triangles as usize, editor.rig.mesh.triangle_count());
    assert_eq!((info.frames, info.fps, info.textured), (48, 24.0, true));

    // L'éditeur rouvert retrouve tous ses réglages.
    let reopened = loaded.snapshot.into_editor();
    assert_eq!(EditorState::capture(&reopened), state);
    assert_eq!(serde_json::to_value(&reopened.rig).unwrap(), rig_json);
}

#[test]
fn project_loads_from_memory_like_from_disk() {
    let dir = TempDir::new("memory");
    let path = save(&full_editor(), &dir, "On disk");
    let from_disk = load(&path).unwrap();
    let from_memory = load_bytes(&std::fs::read(&path).unwrap(), "Embedded").unwrap();
    assert_eq!(from_memory.info.name, "Embedded");
    let json = |p: &ProjectSnapshot| serde_json::to_value(&p.rig).unwrap();
    assert_eq!(json(&from_memory.snapshot), json(&from_disk.snapshot));
    assert_eq!(from_memory.snapshot.state, from_disk.snapshot.state);
    assert!(load_bytes(b"not a project", "Junk").is_err());
}

#[test]
fn shared_textures_are_written_once_and_stay_shared() {
    let dir = TempDir::new("textures");
    let path = save(&full_editor(), &dir, "Textured");
    let archive = zip::ZipArchive::new(std::fs::File::open(&path).unwrap()).unwrap();
    let textures: Vec<_> = archive
        .file_names()
        .filter_map(|n| {
            n.ok()
                .filter(|n| n.starts_with("textures/"))
                .map(|n| n.into_owned())
        })
        .collect();
    assert_eq!(textures.len(), 2, "{textures:?}");

    let rig = load(&path).unwrap().snapshot.rig;
    let tex = |i: usize| rig.mesh.materials[i].base_color_texture.as_ref().unwrap();
    assert!(Arc::ptr_eq(&tex(0).bytes, &tex(1).bytes));
    assert_eq!(tex(2).name, "eyes.png");
    assert_eq!(tex(2).mime, "image/png");
}

#[test]
fn into_editor_gives_fresh_revisions_and_empty_history() {
    let dir = TempDir::new("fresh");
    let mut editor = full_editor();
    editor.set_mode(Mode::Pose);
    let before = editor.rig.revisions();
    let path = save(&editor, &dir, "Fresh");
    let reopened = load(&path).unwrap().snapshot.into_editor();
    let after = reopened.rig.revisions();
    assert_ne!(after.mesh, before.mesh);
    assert_ne!(after.skin, before.skin);
    assert_ne!(after.skeleton, before.skeleton);
    assert!(!reopened.rig.needs_bind());
    assert!(!reopened.can_undo() && !reopened.can_redo());
    assert_eq!(reopened.edit_count(), 0);
    assert_eq!(reopened.mode, Mode::Pose);
    assert_eq!(reopened.rig.pose.bones.len(), reopened.rig.skeleton.len());
}

#[test]
fn stale_weights_ask_for_a_rebind_after_loading() {
    let dir = TempDir::new("stale");
    let mut fresh = Editor::new(Rig::demo());
    let path = save(&fresh, &dir, "Bound");
    let mut bound = load(&path).unwrap().snapshot.into_editor();
    assert!(!bound.rig.needs_bind());
    assert!(bound.bind_job().is_none());

    fresh.rig.touch_skeleton();
    let path = save(&fresh, &dir, "Stale");
    let loaded = load(&path).unwrap();
    assert!(loaded.snapshot.rig.needs_bind());
    let mut stale = loaded.snapshot.into_editor();
    assert!(stale.rig.needs_bind());
    let job = stale.bind_job().expect("liaison demandée");
    let (skin, key) = job.run();
    stale.finish_bind(skin, key);
    assert!(!stale.rig.needs_bind());
}

#[test]
fn empty_project_round_trips() {
    let dir = TempDir::new("empty");
    let editor = Editor::new(Rig::new("Empty", Mesh::default()));
    let path = dir.join("Empty.kbrig");
    let snapshot = ProjectSnapshot::capture(&editor);
    let info = snapshot.save(&path, "Empty", 5, None).unwrap();
    assert_eq!((info.bones, info.vertices, info.textured), (0, 0, false));
    let loaded = load(&path).unwrap();
    assert_eq!(loaded.thumbnail, None);
    assert_eq!(
        loaded.info,
        ProjectSnapshot::capture(&editor).info("Empty", 5, info.modified)
    );
    let reopened = loaded.snapshot.into_editor();
    assert_eq!(reopened.rig.mesh.vertex_count(), 0);
    assert!(reopened.rig.skeleton.is_empty());
}

#[test]
fn read_info_reads_only_manifest_and_thumbnail() {
    let dir = TempDir::new("info");
    let path = save(&full_editor(), &dir, "Gallery");
    // Document illisible : la galerie affiche quand même le projet.
    edit_document(&path, |doc| *doc = Value::String("broken".into()));
    let (info, thumb) = read_info(&path).unwrap();
    assert_eq!(info.name, "Gallery");
    assert_eq!((info.frames, info.fps, info.textured), (48, 24.0, true));
    assert_eq!(thumb, Some(thumbnail()));
    assert!(load(&path).is_err());
}

#[test]
fn saving_again_replaces_the_file_without_leftovers() {
    let dir = TempDir::new("resave");
    let mut editor = full_editor();
    let path = save(&editor, &dir, "Again");
    editor.rig.name = "Renamed rig".into();
    save(&editor, &dir, "Again");
    assert_eq!(dir.files(), vec!["Again.kbrig".to_string()]);
    assert_eq!(load(&path).unwrap().snapshot.rig.name, "Renamed rig");
}

#[test]
fn thumbnails_encode_and_decode() {
    let (w, h) = (40, 30);
    let rgba: Vec<u8> = (0..w * h)
        .flat_map(|i| [(i % w * 6) as u8, 128, 64, 255])
        .collect();
    let jpeg = encode_thumbnail(w, h, &rgba).unwrap();
    assert_eq!(&jpeg[..3], &[0xFF, 0xD8, 0xFF]);
    let (dw, dh, pixels) = decode_thumbnail(&jpeg).unwrap();
    assert_eq!((dw, dh, pixels.len()), (w, h, (w * h * 4) as usize));
    let close = pixels
        .iter()
        .zip(&rgba)
        .all(|(a, b)| (*a as i32 - *b as i32).abs() <= 24);
    assert!(close, "JPEG trop éloigné de l'original");

    assert!(encode_thumbnail(w, h, &rgba[4..]).is_err());
    assert!(encode_thumbnail(0, 0, &[]).is_err());
    assert!(decode_thumbnail(b"not an image").is_err());
    assert!(decode_thumbnail(&jpeg[..jpeg.len() / 3]).is_err());
}

#[test]
fn non_finite_values_never_make_a_project_unreadable() {
    let dir = TempDir::new("nan");
    let mut editor = full_editor();
    editor.rig.pose.bones[1].rotation.x = f32::NAN;
    editor.rig.limbs[0].pole.y = f32::INFINITY;
    editor.rig.pins[0].target.z = f32::NAN;
    Arc::make_mut(&mut editor.rig.mesh).materials[0].roughness = f32::NAN;
    Arc::make_mut(&mut editor.rig.mesh).positions[3].x = f32::NAN;
    editor.camera.distance = f32::NAN;
    editor.frame = f32::NAN;
    let path = save(&editor, &dir, "NaN");
    let loaded = load(&path).unwrap();
    let rig = &loaded.snapshot.rig;
    assert_eq!(rig.pose.bones[1], crate::Transform::IDENTITY);
    assert!(rig.limbs[0].pole.is_finite());
    assert!(rig.pins.is_empty());
    assert!(rig.mesh.materials[0].roughness.is_finite());
    assert!(rig.mesh.positions.iter().all(|p| p.is_finite()));
    assert_eq!(loaded.snapshot.state.camera, None);
    assert_eq!(loaded.snapshot.state.frame, 0.0);
    loaded.snapshot.into_editor();
}
