//! Tests du cycle de vie des projets, sur des dossiers temporaires.

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use bones_core::glam::Vec3;
use bones_core::project::ProjectSnapshot;
use bones_core::{Editor, Mesh, Rig};

use super::autosave::{Autosave, DELAY};
use super::tasks::SaveTask;
use super::worker::Queue;
use super::*;

fn temp_docs(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bones-projects-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn small_editor() -> Editor {
    Editor::new(templates::empty_rig())
}

fn save_task(path: &Path, edits: u64, thumbnail: Option<ThumbnailSource>) -> SaveTask {
    SaveTask {
        path: path.to_owned(),
        // Nom inscrit au manifeste : il identifie l'enregistrement.
        name: format!("v{edits}"),
        created: 1,
        snapshot: ProjectSnapshot::capture(&small_editor()),
        thumbnail,
    }
}

fn files_stem(path: &Path) -> String {
    crate::files::stem(path)
}

thread_local! {
    /// Résultats relevés mais pas encore consommés par [`next`].
    static PENDING: std::cell::RefCell<std::collections::VecDeque<Event>> = Default::default();
}

/// Attend le prochain résultat du fil des projets.
fn next(worker: &Worker) -> Event {
    let start = Instant::now();
    loop {
        PENDING.with_borrow_mut(|p| p.extend(worker.poll()));
        if let Some(event) = PENDING.with_borrow_mut(|p| p.pop_front()) {
            return event;
        }
        assert!(start.elapsed() < Duration::from_secs(20), "aucun résultat");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn worker(docs: &Path) -> Worker {
    Worker::new(dirs(docs), std::sync::Arc::new(|| {}))
}

#[test]
fn autosave_waits_for_two_quiet_seconds_and_a_lifted_pointer() {
    let t0 = Instant::now();
    let mut a = Autosave::new(3);
    assert!(!a.due(3, false, t0));
    assert_eq!(a.deadline(), None);
    // Modification : enregistrement 2 s plus tard, pas avant.
    assert!(!a.due(4, false, t0));
    assert_eq!(a.deadline(), Some(t0 + DELAY));
    assert!(!a.due(4, false, t0 + DELAY / 2));
    // Nouvelle modification : le délai repart.
    assert!(!a.due(5, false, t0 + DELAY));
    assert!(!a.due(5, false, t0 + DELAY + DELAY / 2));
    // Doigt posé : on attend qu'il se lève.
    let late = t0 + DELAY * 3;
    assert!(!a.due(5, true, late));
    assert!(a.due(5, false, late));
    a.saved(5);
    assert!(!a.is_dirty(5));
    assert!(!a.due(5, false, late + DELAY * 2));
    assert_eq!(a.deadline(), None);
    // Annuler jusqu'à l'état enregistré : plus rien à faire.
    assert!(!a.due(6, false, late));
    assert!(!a.due(5, false, late + DELAY * 2));
}

#[test]
fn queue_keeps_one_save_per_file_and_moves_lists_last() {
    let (a, b) = (Path::new("/p/A.kbrig"), Path::new("/p/B.kbrig"));
    let mut queue = Queue::default();
    queue.push(Task::List);
    queue.push(save_task(a, 1, None).into());
    queue.push(save_task(b, 1, None).into());
    queue.push(Task::Rename(b.to_owned(), "C".into()));
    queue.push(save_task(a, 2, None).into());
    queue.push(Task::List);
    let kinds: Vec<String> = queue
        .iter()
        .map(|t| match t {
            Task::Save(s) => format!("save {} {}", files_stem(&s.path), s.name),
            Task::Rename(..) => "rename".into(),
            Task::List => "list".into(),
            _ => "other".into(),
        })
        .collect();
    assert_eq!(kinds, ["save A v2", "save B v1", "rename", "list"]);
}

impl From<SaveTask> for Task {
    fn from(task: SaveTask) -> Self {
        Task::Save(Box::new(task))
    }
}

#[test]
fn saves_of_one_file_never_overlap_and_coalesce() {
    let docs = temp_docs("coalesce");
    let worker = worker(&docs);
    let path = projects_dir(&docs).join("Busy.kbrig");
    // Le premier enregistrement attend sa vignette (rendu GPU simulé).
    let (release, gate) = mpsc::channel::<()>();
    let blocked: ThumbnailSource = Box::new(move || {
        gate.recv().ok()?;
        Some(Rgba {
            size: [4, 3],
            pixels: vec![200; 4 * 3 * 4],
        })
    });
    worker.push(Task::Save(Box::new(save_task(&path, 1, Some(blocked)))));
    std::thread::sleep(Duration::from_millis(50));
    for edits in 2..=4 {
        worker.push(Task::Save(Box::new(save_task(&path, edits, None))));
    }
    assert!(worker.is_busy());
    release.send(()).unwrap();
    worker.flush();
    let saved: Vec<String> = worker
        .poll()
        .into_iter()
        .filter_map(|e| match e {
            Event::Saved { result, .. } => Some(result.expect("enregistrement").0.name),
            _ => None,
        })
        .collect();
    assert_eq!(saved, ["v1", "v4"], "le premier, puis seulement le plus récent");
    assert!(!worker.is_busy());
    std::fs::remove_dir_all(&docs).unwrap();
}

#[test]
fn create_save_load_rename_duplicate_delete() {
    let docs = temp_docs("lifecycle");
    let worker = worker(&docs);
    let build: tasks::Build = Box::new(|| {
        let mesh = Mesh::capsule(Vec3::ZERO, Vec3::Y, 0.2, 8, 2);
        Ok(Rig::new("Capsule", mesh))
    });
    worker.push(Task::Create {
        name: "Hero".into(),
        build,
        open: true,
    });
    let Event::Created { open: true, result } = next(&worker) else {
        panic!("création attendue");
    };
    let created = result.expect("création");
    assert_eq!(created.path, projects_dir(&docs).join("Hero.kbrig"));
    assert_eq!(created.info.name, "Hero");
    assert_eq!(created.editor.rig.name, "Hero");

    // Enregistrement avec vignette : elle est d'abord envoyée telle quelle.
    let mut project = OpenProject::new(
        created.path.clone(),
        "Hero".into(),
        created.info.created,
        &created.editor,
        false,
    );
    assert!(project.needs_save(&created.editor));
    let thumb: ThumbnailSource = Box::new(|| {
        Some(Rgba {
            size: [8, 6],
            pixels: vec![128; 8 * 6 * 4],
        })
    });
    worker.push(Task::Save(Box::new(
        project.save_task(&created.editor, Some(thumb)),
    )));
    assert!(!project.needs_save(&created.editor));
    assert!(matches!(next(&worker), Event::Thumbnail { image, .. } if image.size == [8, 6]));
    let Event::Saved { result, .. } = next(&worker) else {
        panic!("enregistrement attendu");
    };
    assert!(result.expect("enregistrement").0.vertices > 0);

    worker.push(Task::Load(created.path.clone()));
    let Event::Loaded { result, .. } = next(&worker) else {
        panic!("ouverture attendue");
    };
    let loaded = result.expect("ouverture");
    assert!(loaded.thumbnail.is_some(), "vignette enregistrée");

    worker.push(Task::Rename(created.path.clone(), "Villain".into()));
    let Event::Renamed { result, .. } = next(&worker) else {
        panic!("renommage attendu");
    };
    let renamed = result.expect("renommage");
    worker.push(Task::Duplicate(renamed.clone()));
    let Event::Duplicated(copy) = next(&worker) else {
        panic!("copie attendue");
    };
    let copy = copy.expect("copie");
    worker.push(Task::Delete(renamed.clone()));
    worker.push(Task::List);
    worker.flush();
    let listed = worker.poll().into_iter().find_map(|e| match e {
        Event::Listed(entries) => Some(entries),
        _ => None,
    });
    let names: Vec<String> = listed.unwrap().into_iter().map(|e| e.name).collect();
    assert_eq!(names, [files_stem(&copy)]);
    assert_eq!(files_stem(&copy), "Villain 2");
    std::fs::remove_dir_all(&docs).unwrap();
}

#[test]
fn received_projects_are_copied_and_inbox_files_moved() {
    let docs = temp_docs("adopt");
    let worker = worker(&docs);
    let source = docs.join("Shared.kbrig");
    let inbox = docs.join(crate::files::INBOX).join("Mail.kbrig");
    std::fs::create_dir_all(inbox.parent().unwrap()).unwrap();
    for path in [&source, &inbox] {
        ProjectSnapshot::capture(&small_editor())
            .save(path, "x", 1, None)
            .unwrap();
    }
    std::fs::write(docs.join("Broken.kbrig"), b"not a zip").unwrap();
    for path in [&source, &inbox, &docs.join("Broken.kbrig")] {
        worker.push(Task::Adopt(path.clone()));
    }
    worker.flush();
    let results: Vec<Result<PathBuf, String>> = worker
        .poll()
        .into_iter()
        .filter_map(|e| match e {
            Event::Adopted(r) => Some(r),
            _ => None,
        })
        .collect();
    assert_eq!(results.len(), 3);
    assert_eq!(
        results[0].as_deref().unwrap(),
        projects_dir(&docs).join("Shared.kbrig")
    );
    assert_eq!(
        results[1].as_deref().unwrap(),
        projects_dir(&docs).join("Mail.kbrig")
    );
    assert!(results[2].is_err(), "fichier invalide refusé");
    assert!(source.exists(), "copié depuis Documents");
    assert!(!inbox.exists(), "déplacé depuis la boîte de réception");
    std::fs::remove_dir_all(&docs).unwrap();
}

#[test]
fn open_project_tracks_view_changes_and_new_files() {
    let mut editor = small_editor();
    let mut project = OpenProject::new("/p/A.kbrig".into(), "A".into(), 1, &editor, true);
    let now = Instant::now();
    assert!(!project.needs_save(&editor));
    assert!(!project.autosave_due(&editor, false, now));
    // Tourner la vue : pas d'enregistrement automatique, mais en quittant.
    editor.camera.yaw += 0.5;
    assert!(!project.autosave_due(&editor, false, now + DELAY * 2));
    assert!(project.needs_save(&editor));
    project.save_task(&editor, None);
    assert!(!project.needs_save(&editor));
    // Nouveau projet sans vignette : enregistré dès que le doigt est levé.
    let mut fresh = OpenProject::new("/p/B.kbrig".into(), "B".into(), 1, &editor, false);
    assert!(!fresh.autosave_due(&editor, true, now));
    assert!(fresh.autosave_due(&editor, false, now));
    fresh.renaming = true;
    assert!(
        !fresh.autosave_due(&editor, false, now),
        "pas pendant un renommage"
    );
}

#[test]
fn a_panicking_task_is_reported_and_the_worker_keeps_going() {
    let docs = temp_docs("panic");
    let worker = worker(&docs);
    let build: tasks::Build = Box::new(|| panic!("boom"));
    worker.push(Task::Create {
        name: "Broken".into(),
        build,
        open: true,
    });
    assert!(matches!(next(&worker), Event::Failed(m) if m == "boom"));
    worker.push(Task::List);
    assert!(
        matches!(next(&worker), Event::Listed(_)),
        "le fil travaille encore"
    );
    std::fs::remove_dir_all(&docs).unwrap();
}

#[test]
fn templates_build_valid_rigs() {
    let empty = Template::Empty.rig();
    assert_eq!(empty.skeleton.len(), 1);
    assert_eq!(empty.mesh.vertex_count(), 0);
    assert!(Template::Mannequin.rig().mesh.vertex_count() > 0);
}
