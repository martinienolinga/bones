//! Manipulateur 3D en mode Objet : repère de l'objet sélectionné (ou du
//! modèle entier pour le corps) et application de chaque glissement depuis
//! la transformation monde du début du geste.

use glam::Quat;

use crate::editor::Editor;
use crate::editor::manipulate::{ManipGesture, Target};
use crate::manipulator::{Frame, ManipDelta, TransformSpace};
use crate::scene::{ObjectId, ObjectRef};

impl Editor {
    /// Cible, repère et échelle par axe du manipulateur en mode Objet.
    pub(in crate::editor) fn object_manip_target(&self) -> Option<(Target, Frame, bool)> {
        match self.selected_object? {
            // Le corps : transformation du modèle entier (squelette compris).
            ObjectRef::Body => (!self.rig.body.locked).then(|| (Target::Model, self.model_frame(), true)),
            ObjectRef::Extra(id) => {
                if self.rig.object(id)?.locked {
                    return None;
                }
                let world = self.object_display_world(id)?;
                let local = self.manipulator.space == TransformSpace::Local;
                let frame = Frame {
                    origin: world.translation,
                    orientation: if local { world.rotation } else { Quat::IDENTITY },
                };
                // Échelle par axe seulement le long des axes de l'objet.
                let aligned = world.rotation.abs_diff_eq(Quat::IDENTITY, 1e-4)
                    || world.rotation.abs_diff_eq(-Quat::IDENTITY, 1e-4);
                Some((Target::Object(id), frame, local || aligned))
            }
        }
    }

    /// Translation, rotation autour de l'origine de l'objet, ou échelle le
    /// long de ses axes.
    pub(in crate::editor) fn manip_object(&mut self, id: ObjectId, g: &ManipGesture, delta: &ManipDelta) {
        let start = g.start_pose;
        let part = g.drag.part;
        let mut world = start;
        if part.is_rotation() {
            world.rotation = (delta.rotate * start.rotation).normalize();
        } else if part.is_scale() {
            world.scale = start.scale * delta.scale;
        } else {
            world.translation = start.translation + delta.translate;
        }
        self.set_object_world(id, &world);
    }
}
