//! Tests des vignettes des galeries (sans GPU) : les cartes visibles
//! demandent au rendu une vignette du mannequin — fixe pour une expression,
//! renouvelée au fil du cycle pour une animation — et les préréglages
//! d'animaux gardent leur aperçu en bâtons.

use std::time::{Duration, Instant};

use bones_core::{Editor, Mode, Rig, Viewport};
use egui::{RawInput, Rect, pos2, vec2};

use crate::settings::Settings;
use crate::showcase;
use crate::ui::{Busy, Popover, UiState, show};

struct Gallery {
    ctx: egui::Context,
    editor: Editor,
    state: UiState,
}

impl Gallery {
    fn open(popover: Popover, name: &str) -> Self {
        let ctx = egui::Context::default();
        bones_ui::install(&ctx);
        let docs = std::env::temp_dir().join(format!("bones-stage-tests-{name}"));
        let _ = std::fs::remove_dir_all(&docs);
        let mut editor = Editor::new(Rig::demo());
        editor.set_mode(Mode::Pose);
        let mut state = UiState::new(docs, Settings::default());
        state.popover = Some(popover);
        state.popover_anchor = 700.0;
        Self { ctx, editor, state }
    }

    /// Une image à l'instant `time` ; retourne les clés des vignettes
    /// demandées.
    fn frame(&mut self, time: f64) -> Vec<u64> {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1366.0, 1024.0))),
            time: Some(time),
            ..Default::default()
        };
        let viewport = Viewport::new(1366.0, 1024.0);
        let busy = Busy {
            binding: false,
            jobs: 0,
        };
        let (editor, state) = (&mut self.editor, &mut self.state);
        let mut output = self.ctx.run_ui(input, |ui| {
            show(ui.ctx(), editor, state, &viewport, busy);
        });
        output.textures_delta.clear();
        showcase::take_keys(&self.ctx)
    }

    /// Images jusqu'à ce que les vignettes visibles soient toutes demandées
    /// (cuisson sur le fil de fond terminée).
    fn settle(&mut self, time: f64) -> Vec<u64> {
        let deadline = Instant::now() + Duration::from_secs(60);
        let mut last = Vec::new();
        while Instant::now() < deadline {
            let keys = self.frame(time);
            if !keys.is_empty() && keys.len() == last.len() && !self.state.presets.stage.busy() {
                return keys;
            }
            last = keys;
            std::thread::sleep(Duration::from_millis(20));
        }
        last
    }
}

#[test]
fn expression_cards_request_still_mannequin_tiles() {
    let mut g = Gallery::open(Popover::Expressions, "expressions");
    let keys = g.settle(1.0);
    // Grille de trois colonnes : au moins deux rangées visibles, jamais toute
    // la bibliothèque (seules les cartes visibles sont rendues).
    assert!(keys.len() >= 6, "{} vignettes", keys.len());
    let total = super::grid::entries(&g.state.presets, super::Kind::Expressions).len();
    assert!(keys.len() < total, "{} sur {total}", keys.len());
    // Expression : pose fixe, même contenu d'une image à l'autre.
    assert_eq!(g.frame(2.5), keys, "vignettes fixes");
}

#[test]
fn human_animation_cards_play_in_a_loop() {
    let mut g = Gallery::open(Popover::Presets, "human");
    g.state.presets.test_section(0); // Human
    let first = g.settle(0.0);
    assert!(first.len() >= 3, "{} vignettes", first.len());
    let later = g.frame(0.4);
    assert_eq!(later.len(), first.len());
    assert_ne!(later, first, "l'animation avance");
}

#[test]
fn animal_cards_keep_the_stick_figure() {
    let mut g = Gallery::open(Popover::Presets, "animals");
    g.state.presets.test_section(1); // Animals
    for i in 0..5 {
        assert!(
            g.frame(f64::from(i) * 0.1).is_empty(),
            "aucune vignette du mannequin"
        );
    }
}

/// Points d'entrée pour les tests d'interface (pilotage sans toucher aux
/// coordonnées des cartes).
#[cfg(test)]
impl super::PresetsState {
    pub(crate) fn test_section(&mut self, i: usize) {
        self.section = i;
        self.view_slot = Some(super::View::Grid);
    }

    /// Ouvre la feuille d'application sur la première entrée de la section
    /// courante ; faux si la section est vide.
    pub(crate) fn test_open_first(&mut self, kind: super::Kind) -> bool {
        match super::grid::entries(self, kind).first() {
            Some(&entry) => {
                self.view_slot = Some(super::View::Apply(entry));
                true
            }
            None => false,
        }
    }

    pub(crate) fn test_apply(&mut self, editor: &mut Editor, kind: super::Kind) {
        if let super::View::Apply(entry) = self.view() {
            super::apply::run(editor, self, entry, kind);
        }
    }

    pub(crate) fn test_create_save(&mut self, editor: &mut Editor, docs: &std::path::Path) {
        super::create::test_save(editor, self, docs);
    }

    pub(crate) fn test_open_import(&mut self) {
        self.view_slot = Some(super::View::Import);
    }

    pub(crate) fn test_is_grid(&self) -> bool {
        matches!(self.view(), super::View::Grid)
    }

    pub(crate) fn test_report_is_some(&self) -> bool {
        self.report.is_some()
    }

    pub(crate) fn test_user_count(&self) -> usize {
        self.library.user_len()
    }
}
