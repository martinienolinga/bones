//! Démarrage : l'app s'ouvre toujours sur la galerie, et le projet
//! d'exemple n'est créé qu'au premier lancement.

use std::sync::Arc;

use super::{Screen, Session};
use crate::projects::{Event, projects_dir};

#[test]
fn cold_start_shows_the_gallery_and_creates_the_sample_once() {
    let docs = std::env::temp_dir().join(format!("bones-session-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&docs);
    std::fs::create_dir_all(&docs).unwrap();

    for launch in 0..2 {
        let session = Session::start(&docs, Arc::new(|| {}));
        assert_eq!(
            session.screen,
            Screen::Gallery,
            "toujours la galerie au démarrage"
        );
        assert!(session.current.is_none(), "aucun projet ouvert d'office");
        session.worker.flush();
        let events = session.worker.poll();
        let created = events
            .iter()
            .filter(|e| matches!(e, Event::Created { open: false, .. }))
            .count();
        assert_eq!(
            created,
            usize::from(launch == 0),
            "exemple au premier lancement seulement"
        );
        let listed = events.iter().find_map(|e| match e {
            Event::Listed(entries) => Some(entries.iter().map(|e| e.name.clone()).collect::<Vec<_>>()),
            _ => None,
        });
        assert_eq!(listed, Some(vec!["Mannequin".to_owned()]));
    }
    assert!(projects_dir(&docs).join("Mannequin.kbrig").exists());
    std::fs::remove_dir_all(&docs).unwrap();
}
