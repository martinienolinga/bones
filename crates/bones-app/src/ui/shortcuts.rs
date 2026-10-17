//! Raccourcis clavier (Magic Keyboard sur iPad, clavier sur ordinateur).

use bones_core::{Editor, Mode};
use egui::{Key, Modifiers};

use super::UiState;

pub fn handle(ctx: &egui::Context, editor: &mut Editor, state: &mut UiState) {
    if ctx.egui_wants_keyboard_input() {
        return;
    }
    let (undo, redo, keys) = ctx.input_mut(|i| {
        let redo = i.consume_key(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z);
        let undo = i.consume_key(Modifiers::COMMAND, Key::Z);
        let keys: Vec<Key> = [
            Key::Space,
            Key::Delete,
            Key::Backspace,
            Key::K,
            Key::F,
            Key::Num1,
            Key::Num2,
            Key::Num3,
            Key::Escape,
            Key::ArrowLeft,
            Key::ArrowRight,
        ]
        .into_iter()
        .filter(|k| i.consume_key(Modifiers::NONE, *k))
        .collect();
        (undo, redo, keys)
    });
    if redo && editor.can_redo() {
        editor.redo();
    } else if undo && editor.can_undo() {
        editor.undo();
    }
    for key in keys {
        match key {
            Key::Space => editor.toggle_play(),
            Key::Delete | Key::Backspace if editor.mode == Mode::Edit => editor.delete_selected(),
            Key::K if editor.mode == Mode::Pose => editor.add_key(),
            Key::F => editor.frame_all(),
            Key::Num1 => editor.set_mode(Mode::Pose),
            Key::Num2 => editor.set_mode(Mode::Edit),
            Key::Num3 => editor.set_mode(Mode::AutoRig),
            Key::Escape => state.popover = None,
            Key::ArrowLeft => editor.set_frame(editor.frame.round() - 1.0),
            Key::ArrowRight => editor.set_frame(editor.frame.round() + 1.0),
            _ => {}
        }
    }
}
