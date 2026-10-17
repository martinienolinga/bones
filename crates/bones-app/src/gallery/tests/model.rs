//! Mise en page, liste des projets, résumés et cache des vignettes.

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use bones_core::project::{ProjectInfo, store};
use egui::{Pos2, Rect, pos2, vec2};

use super::super::layout::Grid;
use super::super::library::subtitle;
use super::super::{Library, Thumbs, thumbs};
use super::project;

#[test]
fn grid_columns_follow_the_screen_width() {
    // iPad 12,9 pouces paysage et portrait, iPad 11 pouces, iPad mini.
    for (width, columns) in [(1366.0, 5), (1024.0, 4), (1194.0, 4), (834.0, 3), (744.0, 3)] {
        let grid = Grid::new(width);
        assert_eq!(grid.columns, columns, "largeur {width}");
        let last = grid.thumb_rect(Pos2::ZERO, columns - 1);
        assert!(
            (width - last.right() - grid.margin).abs() < 0.5,
            "marges égales ({width})"
        );
        assert!(grid.thumb.x >= 150.0 && grid.thumb.y >= 44.0, "cibles tactiles");
    }
    let grid = Grid::new(1366.0);
    assert_eq!(grid.visible(23, 0.0, 300.0), 0..10);
    assert_eq!(grid.visible(23, grid.pitch.y * 4.0, 2000.0), 20..23);
    let fitted = super::layout::fit(
        Rect::from_min_size(Pos2::ZERO, vec2(200.0, 150.0)),
        vec2(100.0, 100.0),
    );
    assert_eq!(fitted, Rect::from_min_size(pos2(25.0, 0.0), vec2(150.0, 150.0)));
}

#[test]
fn subtitles_are_short_and_informative() {
    let info = |bones, triangles, frames| ProjectInfo {
        name: String::new(),
        created: 0,
        modified: 0,
        bones,
        vertices: triangles,
        triangles,
        frames,
        fps: 24.0,
        textured: false,
    };
    assert_eq!(subtitle(&info(52, 9000, 60)), "52 bones · 60 frames");
    assert_eq!(subtitle(&info(1, 0, 0)), "1 bone · No mesh");
    assert_eq!(subtitle(&info(0, 12_400, 1)), "No bones · 12k triangles");
    assert_eq!(subtitle(&info(3, 1500, 0)), "3 bones · 1.5k triangles");
}

#[test]
fn library_merges_listings_and_reflects_saves_at_once() {
    let mut library = Library::default();
    let t0 = SystemTime::UNIX_EPOCH + Duration::from_secs(100);
    let entry = |name: &str, s: u64| store::Entry {
        path: project(name),
        name: name.into(),
        modified: t0 + Duration::from_secs(s),
    };
    library.set_entries(vec![entry("B", 2), entry("A", 1)]);
    let info = ProjectInfo {
        name: "A".into(),
        created: 0,
        modified: 0,
        bones: 2,
        vertices: 0,
        triangles: 0,
        frames: 0,
        fps: 24.0,
        textured: false,
    };
    library.set_info(&project("A"), t0 + Duration::from_secs(1), info.clone());
    // Liste relue, fichier inchangé : le résumé est gardé.
    library.set_entries(vec![entry("B", 2), entry("A", 1)]);
    assert_eq!(library.get(&project("A")).unwrap().info, Some(info.clone()));
    // Enregistré : en tête de liste, résumé à jour.
    library.saved(&project("A"), info, t0 + Duration::from_secs(5));
    assert_eq!(library.cards()[0].path, project("A"));
    library.moved(&project("A"), &project("C"));
    assert_eq!(library.cards()[0].name, "C");
    library.remove(&[project("C")]);
    assert_eq!(library.index(&project("B")), Some(0));
}

#[test]
fn fresh_thumbnails_are_cached_and_memory_is_bounded() {
    let ctx = egui::Context::default();
    let mut thumbs = Thumbs::new(Arc::new(|| {}));
    let image = || egui::ColorImage::filled([8, 6], egui::Color32::GRAY);
    for i in 0..thumbs::MAX_TEXTURES + 10 {
        thumbs.install(&ctx, &project(&format!("P{i}")), image());
        thumbs.poll(&ctx);
        thumbs.poll(&ctx);
    }
    assert!(thumbs.textures() <= thumbs::MAX_TEXTURES);
    let last = project(&format!("P{}", thumbs::MAX_TEXTURES + 9));
    assert!(thumbs.texture(&last).is_some(), "la plus récente reste");
    // Vignette fraîche confirmée par l'enregistrement : pas de relecture.
    let modified = SystemTime::now();
    thumbs.confirm(&last, modified);
    assert!(thumbs.get(&last, modified, 200).is_some());
}
