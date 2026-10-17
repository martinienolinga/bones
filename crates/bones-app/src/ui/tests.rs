//! Tests de l'interface sans GPU : le studio des os modifie bien l'os
//! sélectionné quand on glisse ses valeurs.

use bones_core::{Editor, Mode, Rig, Viewport};
use egui::{Event, PointerButton, RawInput, Rect, pos2, vec2};

use super::{Busy, Popover, UiState, show};
use crate::settings::Settings;

struct Harness {
    ctx: egui::Context,
    editor: Editor,
    state: UiState,
}

impl Harness {
    fn new() -> Self {
        let ctx = egui::Context::default();
        bones_ui::install(&ctx);
        let docs = std::env::temp_dir().join("bones-ui-tests");
        Self {
            ctx,
            editor: Editor::new(Rig::demo()),
            state: UiState::new(docs, Settings::default()),
        }
    }

    fn frame(&mut self, events: Vec<Event>) {
        let input = RawInput {
            screen_rect: Some(Rect::from_min_size(pos2(0.0, 0.0), vec2(1366.0, 1024.0))),
            events,
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
    }
}

fn button(pos: egui::Pos2, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Default::default(),
    }
}

#[test]
fn dragging_the_head_chip_moves_the_bone() {
    let mut h = Harness::new();
    h.editor.set_mode(Mode::Edit);
    let bone = h.editor.rig.skeleton.find("arm.l").expect("os du mannequin");
    h.editor.select(Some(bone));
    h.state.popover = Some(Popover::Outliner);
    h.state.popover_anchor = 212.0;
    h.state.bones_section = 2;
    for _ in 0..4 {
        h.frame(vec![]);
    }
    let before = h.editor.bone_info(bone).head.x;
    // Pastille « Head X » : première pastille de la ligne Head (voir la
    // capture), repérée par sa position dans le studio.
    let start = pos2(557.0, 260.0);
    h.frame(vec![Event::PointerMoved(start), button(start, true)]);
    for i in 1..=8 {
        h.frame(vec![Event::PointerMoved(start + vec2(8.0 * i as f32, 0.0))]);
    }
    h.frame(vec![button(start + vec2(64.0, 0.0), false)]);
    let after = h.editor.bone_info(bone).head.x;
    assert!(after - before > 0.08, "tête : {before} → {after}");
}

#[test]
fn reparent_messages_become_a_hint_others_a_toast() {
    let mut h = Harness::new();
    h.editor.set_mode(Mode::Edit);
    h.editor.edit_tool = bones_core::EditTool::Reparent;
    h.editor.status = Some("Weights ready".into());
    h.frame(vec![]);
    assert!(h.state.hint.is_none());
    h.editor.status = Some("arm.l: tap the new parent (or empty space for none)".into());
    h.frame(vec![]);
    assert!(h.state.hint.as_deref().is_some_and(|m| m.starts_with("arm.l")));
    h.editor.edit_tool = bones_core::EditTool::Select;
    h.frame(vec![]);
    assert!(h.state.hint.is_none());
}

fn key(key: egui::Key) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Default::default(),
    }
}

#[test]
fn transform_bar_follows_the_manipulator_context() {
    use super::transform_bar::available;
    use bones_core::{EditTool, PoseTool};
    let mut h = Harness::new();
    let bone = h.editor.rig.skeleton.find("arm.l").expect("os du mannequin");
    assert!(!available(&h.editor), "Pose sans sélection");
    h.editor.select(Some(bone));
    assert!(available(&h.editor));
    h.editor.pose_tool = PoseTool::Pin;
    assert!(!available(&h.editor), "outil Épingler");
    h.editor.set_mode(Mode::Edit);
    h.editor.select(None);
    assert!(available(&h.editor), "Édition : modèle entier");
    h.editor.edit_tool = EditTool::Roll;
    assert!(!available(&h.editor), "outils d'édition directe");
    h.editor.set_mode(Mode::AutoRig);
    assert!(!available(&h.editor));
}

#[test]
fn manipulator_keys_change_mode_and_are_saved() {
    use bones_core::manipulator::ManipulatorMode;
    let mut h = Harness::new();
    h.frame(vec![]);
    h.state.settings_dirty = false;
    for (k, mode) in [
        (egui::Key::W, ManipulatorMode::Translate),
        (egui::Key::E, ManipulatorMode::Rotate),
        (egui::Key::R, ManipulatorMode::Scale),
        (egui::Key::Q, ManipulatorMode::Combined),
    ] {
        h.frame(vec![key(k)]);
        assert_eq!(h.editor.manipulator.mode, mode);
        assert_eq!(h.state.settings.manipulator.mode, mode);
    }
    assert!(h.state.settings_dirty);
    let space = h.editor.manipulator.space;
    h.frame(vec![key(egui::Key::X)]);
    assert_ne!(h.editor.manipulator.space, space);
    assert_eq!(h.state.settings.manipulator.space, h.editor.manipulator.space);
}

#[test]
fn saved_manipulator_settings_reach_the_editor() {
    let mut h = Harness::new();
    h.state.settings.manipulator.snap = true;
    h.state.settings.manipulator.angle_step_deg = 30.0;
    h.editor.manipulator.invert_snap = true;
    h.frame(vec![]);
    assert!(h.editor.manipulator.snap);
    assert_eq!(h.editor.manipulator.angle_step_deg, 30.0);
    // L'inversion par Maj appartient au geste en cours, pas aux réglages.
    assert!(h.editor.manipulator.invert_snap);
    assert!(!h.state.settings.manipulator.invert_snap);
}
