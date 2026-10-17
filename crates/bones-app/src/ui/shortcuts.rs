//! Raccourcis clavier (Magic Keyboard sur iPad, clavier sur ordinateur).
//!
//! Partout : ⌘Z / ⇧⌘Z, espace (lecture), F (cadrer), 1 / 2 / 3 (modes),
//! Échap (fermer), flèches (image). En Pose : K (clé). En Édition :
//! Suppr (supprimer), D / ⇧D (dupliquer, avec les enfants), P / C / N
//! (parent, enfant, frère suivant), M (édition symétrique). Manipulateur :
//! W / E / R / Q (déplacer, tourner, échelle, tout), X (repère global /
//! local).

use bones_core::manipulator::ManipulatorMode;
use bones_core::{Editor, Mode};
use egui::{Key, Modifiers};

use super::UiState;

const KEYS: [Key; 21] = [
    Key::Space,
    Key::Delete,
    Key::Backspace,
    Key::K,
    Key::F,
    Key::D,
    Key::P,
    Key::C,
    Key::N,
    Key::M,
    Key::W,
    Key::E,
    Key::R,
    Key::Q,
    Key::X,
    Key::Num1,
    Key::Num2,
    Key::Num3,
    Key::Escape,
    Key::ArrowLeft,
    Key::ArrowRight,
];

pub fn handle(ctx: &egui::Context, editor: &mut Editor, state: &mut UiState) {
    if ctx.egui_wants_keyboard_input() {
        return;
    }
    let (undo, redo, shift_d, keys) = ctx.input_mut(|i| {
        let redo = i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z);
        let undo = i.consume_key(Modifiers::COMMAND, Key::Z);
        // ⇧D d'abord : sans modificateur demandé, egui ignore Maj.
        let shift_d = i.consume_key(Modifiers::SHIFT, Key::D);
        let keys: Vec<Key> = KEYS
            .into_iter()
            .filter(|k| i.consume_key(Modifiers::NONE, *k))
            .collect();
        (undo, redo, shift_d, keys)
    });
    if redo && editor.can_redo() {
        editor.redo();
    } else if undo && editor.can_undo() {
        editor.undo();
    }
    let editing = editor.mode == Mode::Edit;
    if shift_d && editing {
        editor.duplicate_selected(true);
    }
    for key in keys {
        match key {
            Key::Space => editor.toggle_play(),
            Key::Delete | Key::Backspace if editing => editor.delete_selected(),
            Key::D if editing => editor.duplicate_selected(false),
            Key::P if editing => editor.select_parent(),
            Key::C if editing => editor.select_child(),
            Key::N if editing => editor.select_next_sibling(),
            Key::M if editing => editor.mirror_edit = !editor.mirror_edit,
            Key::K if editor.mode == Mode::Pose => editor.add_key(),
            Key::W => editor.set_manipulator_mode(ManipulatorMode::Translate),
            Key::E => editor.set_manipulator_mode(ManipulatorMode::Rotate),
            Key::R => editor.set_manipulator_mode(ManipulatorMode::Scale),
            Key::Q => editor.set_manipulator_mode(ManipulatorMode::Combined),
            Key::X => editor.toggle_manipulator_space(),
            Key::F => editor.frame_all(),
            Key::Num1 => editor.set_mode(Mode::Pose),
            Key::Num2 => editor.set_mode(Mode::Edit),
            Key::Num3 => editor.set_mode(Mode::AutoRig),
            Key::Escape => {
                state.popover = None;
                state.transform.options = false;
            }
            Key::ArrowLeft => editor.set_frame(editor.frame.round() - 1.0),
            Key::ArrowRight => editor.set_frame(editor.frame.round() + 1.0),
            _ => {}
        }
    }
}
