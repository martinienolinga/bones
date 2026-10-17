//! Historique d'annulation et changements de mode.

use super::{Editor, Mode, UNDO_LIMIT};

impl Editor {
    pub(super) fn checkpoint(&mut self) {
        self.undo.push(self.rig.clone());
        if self.undo.len() > UNDO_LIMIT {
            self.undo.remove(0);
        }
        self.redo_backup = std::mem::take(&mut self.redo);
    }

    /// Abandonne le dernier point de reprise (geste annulé ou sans effet) :
    /// l'historique redevient exactement celui d'avant. Avec `restore`, le
    /// document revient aussi à l'état sauvegardé.
    pub(super) fn drop_checkpoint(&mut self, restore: bool) {
        if let Some(rig) = self.undo.pop()
            && restore
        {
            self.rig = rig;
            self.after_history();
        }
        self.redo = std::mem::take(&mut self.redo_backup);
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self) {
        if let Some(rig) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.rig, rig));
            self.after_history();
        }
    }

    pub fn redo(&mut self) {
        if let Some(rig) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.rig, rig));
            self.after_history();
        }
    }

    fn after_history(&mut self) {
        self.drag = None;
        if self.selected.is_some_and(|s| !self.rig.skeleton.contains(s)) {
            self.selected = None;
        }
        self.snap_limbs();
    }

    pub fn set_mode(&mut self, mode: Mode) {
        if self.drag.is_some() {
            self.pointer_cancel();
        }
        self.mode = mode;
        self.playing = false;
        match mode {
            Mode::Pose => self.snap_limbs(),
            Mode::AutoRig if self.markers.is_none() => self.guess_markers(),
            _ => {}
        }
    }

    pub(super) fn snap_limbs(&mut self) {
        let world = self.rig.world();
        let skeleton = &self.rig.skeleton;
        for limb in &mut self.rig.limbs {
            if limb.is_valid(skeleton) {
                limb.snap_to_pose(skeleton, &world);
            }
        }
    }
}
