//! Opérations sur les fichiers de projets d'un dossier.

use std::time::{Duration, SystemTime};

use super::super::store::{self, sanitize_name};
use super::super::{load, read_info};
use super::{TempDir, full_editor, save};

fn names(entries: &[store::Entry]) -> Vec<&str> {
    entries.iter().map(|e| e.name.as_str()).collect()
}

#[test]
fn unique_paths_count_up() {
    let dir = TempDir::new("unique");
    let first = store::unique_path(dir.path(), "Untitled");
    assert_eq!(first, dir.join("Untitled.kbrig"));
    std::fs::write(&first, b"").unwrap();
    let second = store::unique_path(dir.path(), "Untitled");
    assert_eq!(second, dir.join("Untitled 2.kbrig"));
    std::fs::write(&second, b"").unwrap();
    assert_eq!(store::unique_path(dir.path(), "  "), dir.join("Untitled 3.kbrig"));
    assert_eq!(store::unique_path(dir.path(), "a/b"), dir.join("a-b.kbrig"));
}

#[test]
fn names_are_sanitized_but_keep_unicode() {
    assert_eq!(sanitize_name("  Été: walk/run\n "), "Été- walk-run");
    assert_eq!(sanitize_name("../../etc"), "-..-etc");
    assert_eq!(sanitize_name(".hidden"), "hidden");
    assert_eq!(sanitize_name("Trailing dots..."), "Trailing dots");
    assert_eq!(sanitize_name("tab\there"), "tab here");
    assert_eq!(sanitize_name("日本語のキャラ"), "日本語のキャラ");
    assert_eq!(sanitize_name(" \u{7} "), "");
    let long = sanitize_name(&"é".repeat(500));
    assert_eq!(long.chars().count(), 80);
    let wide = sanitize_name(&"😀".repeat(100));
    assert!(wide.len() <= 200 && !wide.is_empty());
}

#[test]
fn rename_handles_collisions_and_bad_names() {
    let dir = TempDir::new("rename");
    let editor = full_editor();
    let a = save(&editor, &dir, "Alpha");
    save(&editor, &dir, "Beta");
    let renamed = store::rename(&a, "Beta").unwrap();
    assert_eq!(renamed, dir.join("Beta 2.kbrig"));
    assert!(!a.exists());
    assert_eq!(read_info(&renamed).unwrap().0.name, "Beta 2");
    assert_eq!(store::rename(&renamed, " Beta 2 ").unwrap(), renamed);
    assert!(store::rename(&renamed, " / ").is_ok_and(|p| p == dir.join("-.kbrig")));
    assert!(store::rename(&dir.join("-.kbrig"), "...").is_err());
    let cased = store::rename(&dir.join("-.kbrig"), "Élan: v2").unwrap();
    assert_eq!(cased, dir.join("Élan- v2.kbrig"));
    assert!(load(&cased).is_ok());
}

#[test]
fn duplicate_numbers_copies_and_refreshes_the_manifest() {
    let dir = TempDir::new("duplicate");
    let original = save(&full_editor(), &dir, "Walk");
    let copy = store::duplicate(&original).unwrap();
    assert_eq!(copy, dir.join("Walk 2.kbrig"));
    let again = store::duplicate(&copy).unwrap();
    assert_eq!(again, dir.join("Walk 3.kbrig"));

    let (info, thumb) = read_info(&copy).unwrap();
    assert_eq!(info.name, "Walk 2");
    assert!(info.created > 1_700_000_000);
    assert!(thumb.is_some());
    let a = load(&original).unwrap().snapshot.rig;
    let b = load(&copy).unwrap().snapshot.rig;
    assert_eq!(
        serde_json::to_value(&a).unwrap(),
        serde_json::to_value(&b).unwrap()
    );
}

#[test]
fn delete_removes_and_ignores_missing_files() {
    let dir = TempDir::new("delete");
    let path = save(&full_editor(), &dir, "Gone");
    store::delete(&path).unwrap();
    assert!(!path.exists());
    store::delete(&path).unwrap();
}

#[test]
fn list_shows_projects_newest_first() {
    let dir = TempDir::new("list");
    let t0 = SystemTime::now() - Duration::from_secs(3600);
    for (i, name) in ["Old", "Newest", "Middle"].into_iter().enumerate() {
        let path = dir.join(&format!("{name}.kbrig"));
        std::fs::write(&path, b"").unwrap();
        let age = [0, 120, 60][i];
        let file = std::fs::File::options().write(true).open(&path).unwrap();
        file.set_modified(t0 + Duration::from_secs(age)).unwrap();
    }
    for other in ["notes.txt", ".hidden.kbrig", ".Old.kbrig.42-0.tmp", "UPPER.KBRIG"] {
        std::fs::write(dir.join(other), b"").unwrap();
    }
    std::fs::create_dir(dir.join("folder.kbrig")).unwrap();
    let entries = store::list(dir.path());
    assert_eq!(names(&entries)[..3], ["UPPER", "Newest", "Middle"]);
    assert_eq!(names(&entries).len(), 4);
    assert_eq!(names(&entries)[3], "Old");
    assert!(store::list(&dir.join("absent")).is_empty());
}

#[test]
fn generic_helpers_work_for_other_extensions() {
    let dir = TempDir::new("ext");
    let path = store::unique_path_ext(dir.path(), "Jump", "kb3da");
    assert_eq!(path, dir.join("Jump.kb3da"));
    std::fs::write(&path, b"preset").unwrap();
    let copy = store::duplicate_file(&path).unwrap();
    assert_eq!(copy, dir.join("Jump 2.kb3da"));
    assert_eq!(std::fs::read(&copy).unwrap(), b"preset");
    let renamed = store::rename(&copy, "Hop").unwrap();
    assert_eq!(renamed, dir.join("Hop.kb3da"));
    assert_eq!(names(&store::list_ext(dir.path(), "kb3da")).len(), 2);
    assert!(store::list(dir.path()).is_empty());
}
