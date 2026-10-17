//! Tests des fichiers de préréglages : format natif `.kb3da` (aller-retour,
//! fiche, refus des fichiers abîmés ou trop récents), bibliothèque de
//! l'utilisateur, et échange BVH / glTF (aller-retour des animations).

use std::path::PathBuf;

use glam::Vec3;

use crate::presets::model::{Preset, PresetKind, PresetSource};
use crate::presets::{self, ApplyOptions, CreateOptions, PresetError, RigType, file, user};
use crate::{Clip, Editor, Pose, Rig, Skeleton, head_tail};

/// Dossier temporaire propre à un test.
fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bones-presets-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Préréglage à clés : marche intégrée appliquée au mannequin, puis capturée.
fn made_preset(name: &str) -> Preset {
    let mut e = Editor::new(Rig::demo());
    let walk = presets::find_builtin("builtin.human.walk").unwrap().clone();
    e.apply_preset(&walk, &ApplyOptions::default()).unwrap();
    e.create_preset(&CreateOptions {
        name: name.into(),
        looping: true,
        ..CreateOptions::default()
    })
    .unwrap()
}

fn source(preset: &Preset) -> (&Skeleton, &Clip) {
    match &preset.kind {
        PresetKind::Keyframes(d) => (&d.skeleton, &d.clip),
        PresetKind::Procedural(_) | PresetKind::Expression(_) => panic!("préréglage à clés attendu"),
    }
}

/// Têtes monde des os (par nom) à une image donnée.
fn heads(skeleton: &Skeleton, clip: &Clip, frame: f32) -> Vec<(String, Vec3)> {
    let mut pose = Pose::rest(skeleton);
    clip.sample_into(frame, &mut pose);
    let world = pose.world(skeleton);
    skeleton
        .bones()
        .iter()
        .zip(&world)
        .map(|(b, w)| (b.name.clone(), head_tail(w, b.length).0))
        .collect()
}

/// Même mouvement (os appariés par nom) à trois images entières.
fn assert_same_motion(a: &Preset, b: &Preset, tol: f32) {
    let ((sa, ca), (sb, cb)) = (source(a), source(b));
    let last = ca.frame_count.min(cb.frame_count).saturating_sub(1);
    for f in [0, last / 2, last] {
        let hb = heads(sb, cb, f as f32);
        for (name, p) in heads(sa, ca, f as f32) {
            if let Some((_, q)) = hb.iter().find(|(n, _)| *n == name) {
                assert!(p.abs_diff_eq(*q, tol), "{name} @ {f}: {p:?} / {q:?}");
            }
        }
    }
}

#[test]
fn create_save_load_apply_roundtrip() {
    let dir = temp_dir("native");
    let made = made_preset("My Walk");
    assert!(matches!(made.kind, PresetKind::Keyframes(_)));

    let path = user::save(&made, &dir).unwrap();
    assert_eq!(path.extension().unwrap(), "kb3da");

    // Fiche lisible sans décompresser le contenu (galerie).
    let info = presets::read_info(&path).unwrap();
    assert_eq!(info.name, made.info.name);
    assert_eq!(info.rig_type, RigType::Humanoid);

    // Chargement complet identique après l'aller-retour JSON.
    let loaded = presets::load(&path).unwrap();
    assert_eq!(loaded, made);

    // Application identique sur deux projets neufs.
    let (mut a, mut b) = (Editor::new(Rig::demo()), Editor::new(Rig::demo()));
    let ra = a.apply_preset(&made, &ApplyOptions::default()).unwrap();
    let rb = b.apply_preset(&loaded, &ApplyOptions::default()).unwrap();
    assert_eq!(ra.frames, rb.frames);
    assert_eq!(*a.rig.clip, *b.rig.clip);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn user_library_crud() {
    let dir = temp_dir("library");
    let preset = made_preset("Walk");
    let a = user::save(&preset, &dir).unwrap();
    let b = user::save(&preset, &dir).unwrap();
    assert_ne!(a, b);
    let listed = user::list(&dir);
    assert_eq!(listed.len(), 2);
    assert!(listed.iter().all(|l| l.info.name == "Walk"));

    let renamed = user::rename(&a, "Jog").unwrap();
    assert_eq!(renamed.file_stem().unwrap(), "Jog");
    let copy = user::duplicate(&renamed).unwrap();
    assert_eq!(user::list(&dir).len(), 3);
    assert_eq!(user::load(&copy).unwrap().info.name, "Walk");

    for path in [renamed, copy, b] {
        user::delete(&path).unwrap();
    }
    assert!(user::list(&dir).is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn rejects_corrupt_and_future_presets() {
    assert!(file::load_bytes(b"this is not a zip container").is_err());
    let dir = temp_dir("bad");
    let path = dir.join("future.kb3da");
    file::save_future(&made_preset("Future"), &path).unwrap();
    match presets::load(&path) {
        Err(PresetError::TooNew { supported, .. }) => assert_eq!(supported, 1),
        other => panic!("TooNew attendu, obtenu {other:?}"),
    }
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn bvh_animation_roundtrip() {
    let dir = temp_dir("bvh");
    let made = made_preset("Walk");
    let path = dir.join("walk.bvh");
    assert_eq!(presets::export_file(&made, &path).unwrap().len(), 1);
    let back = presets::import_file(&path).unwrap();
    assert_eq!(back.len(), 1);
    assert_eq!(back[0].info.name, "walk");
    assert!(matches!(back[0].info.source, PresetSource::Imported { .. }));
    assert_eq!(back[0].info.rig_type, RigType::Humanoid);
    assert_same_motion(&made, &back[0], 0.02);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn gltf_animation_roundtrip() {
    let dir = temp_dir("gltf");
    let made = made_preset("Walk");
    let path = dir.join("walk.glb");
    presets::export_file(&made, &path).unwrap();
    let back = presets::import_file(&path).unwrap();
    assert_eq!(back.len(), 1);
    assert_eq!(back[0].info.rig_type, RigType::Humanoid);
    assert_same_motion(&made, &back[0], 5e-3);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn procedural_preset_exports_and_reimports() {
    let dir = temp_dir("proc");
    let jump = presets::find_builtin("builtin.human.jump").unwrap().clone();
    let path = dir.join("jump.bvh");
    presets::export_file(&jump, &path).unwrap();
    let back = presets::import_file(&path).unwrap();
    assert_eq!(back.len(), 1);
    assert_eq!(back[0].info.rig_type, RigType::Humanoid);
    assert!(back[0].info.frames > 5, "{}", back[0].info.frames);
    // L'animation réimportée s'applique à un projet.
    let mut e = Editor::new(Rig::demo());
    assert!(e.apply_preset(&back[0], &ApplyOptions::default()).unwrap().frames > 5);
    let _ = std::fs::remove_dir_all(&dir);
}
