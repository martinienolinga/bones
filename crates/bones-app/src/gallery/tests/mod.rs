//! Tests de la galerie sans GPU : [`interactions`] (toucher une carte, mode
//! sélection, renommage, confirmation de suppression, menus « + »,
//! « Import » et de carte, appui long) et [`model`] (mise en page, liste,
//! résumés, cache des vignettes). Ce module fournit le banc d'essai.

mod interactions;
mod model;

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use bones_core::project::store;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, RawInput, Rect, pos2};

use super::layout::{BAR_H, GRID_TOP, Grid};
use super::*;

const SCREEN: [f32; 2] = [1366.0, 1024.0];

struct Harness {
    ctx: egui::Context,
    gallery: Gallery,
    actions: Vec<GalleryAction>,
    time: f64,
}

fn project(name: &str) -> PathBuf {
    PathBuf::from(format!("/nonexistent/Projects/{name}.kbrig"))
}

impl Harness {
    fn new(names: &[&str]) -> Self {
        let ctx = egui::Context::default();
        bones_ui::install(&ctx);
        let mut gallery = Gallery::new(Arc::new(|| {}));
        let now = SystemTime::now();
        gallery.library.set_entries(
            names
                .iter()
                .enumerate()
                .map(|(i, name)| store::Entry {
                    path: project(name),
                    name: (*name).to_owned(),
                    modified: now - Duration::from_secs(i as u64),
                })
                .collect(),
        );
        let mut h = Self {
            ctx,
            gallery,
            actions: Vec::new(),
            time: 0.0,
        };
        h.frames(3);
        h
    }

    fn frame(&mut self, events: Vec<Event>) {
        self.time += 1.0 / 60.0;
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, SCREEN.into())),
            time: Some(self.time),
            events,
            ..Default::default()
        };
        let (gallery, actions) = (&mut self.gallery, &mut self.actions);
        let mut output = self.ctx.run_ui(input, |ui| {
            actions.extend(show(ui.ctx(), gallery, None));
        });
        output.textures_delta.clear();
    }

    fn frames(&mut self, n: usize) {
        for _ in 0..n {
            self.frame(vec![]);
        }
    }

    fn tap(&mut self, pos: Pos2) {
        self.frame(vec![
            Event::PointerMoved(pos),
            button(pos, PointerButton::Primary, true),
        ]);
        self.frame(vec![button(pos, PointerButton::Primary, false)]);
        self.frames(12);
    }

    fn right_click(&mut self, pos: Pos2) {
        self.frame(vec![
            Event::PointerMoved(pos),
            button(pos, PointerButton::Secondary, true),
        ]);
        self.frame(vec![button(pos, PointerButton::Secondary, false)]);
        self.frames(12);
    }

    fn key(&mut self, key: Key) {
        let event = |pressed| Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: Modifiers::NONE,
        };
        self.frame(vec![event(true), event(false)]);
        self.frames(2);
    }

    /// Centre de la vignette de la carte `index`.
    fn card(&self, index: usize) -> Pos2 {
        Grid::new(SCREEN[0])
            .thumb_rect(pos2(0.0, BAR_H + GRID_TOP), index)
            .center()
    }

    /// Nom de la carte `index`.
    fn title(&self, index: usize) -> Pos2 {
        let thumb = Grid::new(SCREEN[0]).thumb_rect(pos2(0.0, BAR_H + GRID_TOP), index);
        pos2(thumb.center().x, thumb.bottom() + 21.0)
    }

    /// Bouton `index` de la barre (capsule, puis « + »).
    fn bar_button(&self, index: usize) -> Pos2 {
        self.gallery.buttons[index].center()
    }

    fn take(&mut self) -> Vec<GalleryAction> {
        std::mem::take(&mut self.actions)
    }
}

fn button(pos: Pos2, button: PointerButton, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: Default::default(),
    }
}
