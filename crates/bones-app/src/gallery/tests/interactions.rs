//! Gestes et commandes de la galerie.

use egui::{Event, Key, PointerButton, pos2};

use super::super::{GalleryAction, Menu};
use super::{Harness, SCREEN, button, project};
use crate::files::ImportFile;
use crate::projects::Template;

#[test]
fn tapping_a_card_opens_its_project_once() {
    let mut h = Harness::new(&["Hero", "Fox", "Robot"]);
    h.tap(h.card(1));
    assert_eq!(h.take(), [GalleryAction::Open(project("Fox"))]);
    assert_eq!(h.gallery.opening, Some(project("Fox")));
    // Pendant l'ouverture, les autres cartes ne réagissent pas.
    h.tap(h.card(0));
    assert!(h.take().is_empty());
}

#[test]
fn select_mode_toggles_cards_and_confirms_deletion() {
    let mut h = Harness::new(&["Hero", "Fox", "Robot"]);
    h.tap(h.bar_button(0)); // Select
    assert!(h.gallery.is_selecting());
    h.tap(h.card(0));
    h.tap(h.card(2));
    h.tap(h.card(0));
    assert_eq!(h.gallery.selection, Some(vec![project("Robot")]));
    assert!(h.take().is_empty(), "aucune ouverture en mode sélection");
    h.tap(h.card(1));

    h.tap(h.bar_button(1)); // Delete : confirmation d'abord
    assert!(h.take().is_empty());
    assert_eq!(h.gallery.confirm, Some(vec![project("Robot"), project("Fox")]));
    // Bouton rouge « Delete », à droite dans le panneau centré.
    h.tap(pos2(SCREEN[0] * 0.5 + 76.0, SCREEN[1] * 0.5 + 46.0));
    assert_eq!(
        h.take(),
        [GalleryAction::Delete(vec![project("Robot"), project("Fox")])]
    );
    let names: Vec<&str> = h
        .gallery
        .library
        .cards()
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    assert_eq!(names, ["Hero"], "cartes retirées tout de suite");

    h.tap(h.bar_button(2)); // Done
    assert!(!h.gallery.is_selecting());
}

#[test]
fn cancelling_the_confirmation_keeps_the_projects() {
    let mut h = Harness::new(&["Hero"]);
    h.right_click(h.card(0));
    assert!(matches!(h.gallery.menu, Some(Menu::Card(..))));
    // Menu de carte : Rename, Duplicate, Delete (lignes de 48 pt).
    let menu = h
        .ctx
        .memory(|m| m.area_rect(egui::Id::unique("gallery-card")))
        .unwrap();
    h.tap(pos2(menu.center().x, menu.top() + 6.0 + 48.0 * 2.5));
    assert_eq!(h.gallery.confirm, Some(vec![project("Hero")]));
    h.tap(pos2(SCREEN[0] * 0.5 - 76.0, SCREEN[1] * 0.5 + 46.0)); // Cancel
    assert!(h.gallery.confirm.is_none());
    assert!(h.take().is_empty());
    assert_eq!(h.gallery.library.cards().len(), 1);
}

#[test]
fn context_menu_duplicates_and_renames() {
    let mut h = Harness::new(&["Hero", "Fox"]);
    h.right_click(h.card(1));
    let menu = h
        .ctx
        .memory(|m| m.area_rect(egui::Id::unique("gallery-card")))
        .unwrap();
    h.tap(pos2(menu.center().x, menu.top() + 6.0 + 48.0 * 1.5));
    assert_eq!(h.take(), [GalleryAction::Duplicate(vec![project("Fox")])]);
    assert!(h.gallery.menu.is_none());

    h.right_click(h.card(1));
    let menu = h
        .ctx
        .memory(|m| m.area_rect(egui::Id::unique("gallery-card")))
        .unwrap();
    h.tap(pos2(menu.center().x, menu.top() + 6.0 + 24.0));
    assert!(
        h.gallery
            .rename
            .as_ref()
            .is_some_and(|r| r.path == project("Fox"))
    );
}

#[test]
fn long_press_opens_the_menu_without_opening_the_project() {
    let mut h = Harness::new(&["Hero"]);
    let pos = h.card(0);
    h.frame(vec![
        Event::PointerMoved(pos),
        button(pos, PointerButton::Primary, true),
    ]);
    for _ in 0..40 {
        h.frame(vec![]); // ≈ 0,66 s
    }
    assert!(matches!(h.gallery.menu, Some(Menu::Card(..))));
    h.frame(vec![button(pos, PointerButton::Primary, false)]);
    h.frames(4);
    assert!(h.take().is_empty(), "relâcher n'ouvre pas le projet");
    assert!(h.gallery.menu.is_some());
    // Toucher à côté ferme le menu sans rien faire d'autre.
    h.tap(pos2(700.0, 900.0));
    assert!(h.gallery.menu.is_none());
    assert!(h.take().is_empty());
}

#[test]
fn tapping_the_name_renames_in_place() {
    let mut h = Harness::new(&["Hero", "Fox"]);
    h.tap(h.title(0));
    assert!(h.gallery.rename.is_some());
    // Tout le nom est sélectionné : la saisie le remplace.
    h.frame(vec![Event::Text("Villain".into())]);
    h.key(Key::Enter);
    assert_eq!(
        h.take(),
        [GalleryAction::Rename(project("Hero"), "Villain".into())]
    );
    assert_eq!(h.gallery.library.cards()[0].name, "Villain");
    assert!(h.gallery.rename.is_none());

    // Échap annule.
    h.tap(h.title(1));
    h.frame(vec![Event::Text("Nope".into())]);
    h.key(Key::Escape);
    assert!(h.take().is_empty());
    assert_eq!(h.gallery.library.cards()[1].name, "Fox");
}

#[test]
fn plus_menu_creates_projects_and_leads_to_import() {
    let mut h = Harness::new(&["Hero"]);
    h.tap(h.bar_button(2)); // « + »
    assert!(matches!(h.gallery.menu, Some(Menu::New(_))));
    let menu = h
        .ctx
        .memory(|m| m.area_rect(egui::Id::unique("gallery-new")))
        .unwrap();
    // Titre, puis trois lignes de 54 pt.
    let row = |i: f32| pos2(menu.center().x, menu.top() + 20.0 + 40.0 + 54.0 * (i + 0.5));
    h.tap(row(1.0));
    assert_eq!(h.take(), [GalleryAction::New(Template::Empty)]);
    assert!(h.gallery.menu.is_none());

    h.tap(h.bar_button(2));
    h.tap(row(2.0));
    assert_eq!(h.take(), [GalleryAction::ListImports]);
    assert!(matches!(h.gallery.menu, Some(Menu::Import(_))));
}

#[test]
fn import_lists_files_and_imports_the_picked_one() {
    let mut h = Harness::new(&["Hero"]);
    h.tap(h.bar_button(1)); // Import
    assert_eq!(h.take(), [GalleryAction::ListImports]);
    assert!(matches!(h.gallery.menu, Some(Menu::Import(_))));
    let fox = ImportFile {
        name: "Fox".into(),
        path: "/docs/Fox.glb".into(),
        format: Some(bones_core::Format::Glb),
    };
    let shared = ImportFile {
        name: "Shared".into(),
        path: "/docs/Inbox/Shared.kbrig".into(),
        format: None,
    };
    h.gallery.imports = Some(vec![fox.clone(), shared]);
    h.frames(3);
    let menu = h
        .ctx
        .memory(|m| m.area_rect(egui::Id::unique("gallery-import")))
        .unwrap();
    // Titre, titre de section « 3D models », puis la ligne.
    h.tap(pos2(menu.center().x, menu.top() + 20.0 + 40.0 + 32.0 + 27.0));
    assert_eq!(h.take(), [GalleryAction::Import(fox)]);
    assert!(h.gallery.menu.is_none());
}

#[test]
fn escape_leaves_select_mode_and_empty_library_shows_the_invitation() {
    let mut h = Harness::new(&["Hero"]);
    h.tap(h.bar_button(0));
    assert!(h.gallery.is_selecting());
    h.key(Key::Escape);
    assert!(!h.gallery.is_selecting());

    let mut empty = Harness::new(&[]);
    assert!(empty.gallery.library.is_listed());
    empty.tap(pos2(683.0, 500.0));
    assert!(empty.take().is_empty());
}
