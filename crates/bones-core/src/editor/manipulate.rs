//! Manipulateur 3D dans l'éditeur : cible (os sélectionné, ou modèle entier
//! en mode Édition sans sélection), repère, saisie prioritaire sur les os,
//! fin de geste et réglages.

use glam::{Quat, Vec2, Vec3};

use super::drag::Drag;
use super::{EditTool, Editor, Mode, PoseTool};
use crate::manipulator::{
    Frame, Geometry, ManipDrag, ManipulatorMode, Options, Part, Projector, TransformSpace,
};
use crate::{BoneId, LimbMode, Transform, Viewport};

/// Objet manipulé.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Target {
    Bone(BoneId),
    Model,
}

/// Comment un os suit une translation en mode Pose.
#[derive(Clone, Debug)]
pub(super) enum Follow {
    /// Translation de son delta de pose (racine ou os détaché).
    Delta,
    /// Membre IK : sa cible se déplace (indice, position de départ).
    Limb(usize, Vec3),
    /// Os connecté : la chaîne parente (FABRIK) amène sa tête sous le doigt.
    Chain(Vec<BoneId>),
}

/// Geste en cours sur le manipulateur.
#[derive(Clone, Debug)]
pub(super) struct ManipGesture {
    pub drag: ManipDrag,
    pub target: Target,
    /// Delta de pose de l'os au début (mode Pose).
    pub start_pose: Transform,
    /// Tête, longueur et rotation monde de repos au début (mode Édition).
    pub start_head: Vec3,
    pub start_len: f32,
    pub start_rot: Quat,
    pub follow: Follow,
    /// Zone d'affichage du geste (sélection d'un os après un simple toucher).
    pub viewport: Viewport,
}

impl Editor {
    /// Cible du manipulateur, son repère et la disponibilité de l'échelle
    /// par axe ; `None` quand il est masqué ou sans objet.
    fn manipulator_target(&self) -> Option<(Target, Frame, bool)> {
        if !self.manipulator.visible {
            return None;
        }
        let tool_ok = match self.mode {
            Mode::Pose => self.pose_tool != PoseTool::Pin,
            // Les outils d'édition directe (tête, os, roulis…) gardent la main.
            Mode::Edit => self.edit_tool == EditTool::Select,
            Mode::AutoRig => false,
        };
        if !tool_ok {
            return None;
        }
        let local = self.manipulator.space == TransformSpace::Local;
        let oriented = |rotation: Quat| {
            if local {
                rotation.normalize()
            } else {
                Quat::IDENTITY
            }
        };
        match self.selected.filter(|b| self.rig.skeleton.contains(*b)) {
            Some(bone) => {
                let world = match self.mode {
                    Mode::Pose => self.rig.world()[bone.index()],
                    _ => {
                        let (head, _) = self.rig.skeleton.rest_head_tail(bone);
                        Transform::from_rotation_translation(
                            self.rig.skeleton.rest_world_rotation(bone),
                            head,
                        )
                    }
                };
                let frame = Frame {
                    origin: world.translation,
                    orientation: oriented(world.rotation),
                };
                // Échelle d'un os : uniforme (delta de pose ou longueur).
                Some((Target::Bone(bone), frame, false))
            }
            None if self.mode == Mode::Edit => {
                let origin = match &self.drag {
                    Some(Drag::Manip(g)) => g.drag.frame.origin + g.drag.last.translate,
                    _ => self.model_pivot(),
                };
                let frame = Frame {
                    origin,
                    orientation: Quat::IDENTITY,
                };
                Some((Target::Model, frame, true))
            }
            None => None,
        }
    }

    /// Pivot du modèle : centre de sa base (au sol), sinon tête de la
    /// première racine.
    fn model_pivot(&self) -> Vec3 {
        if let Some((lo, hi)) = self.rig.mesh.bounds() {
            let c = (lo + hi) * 0.5;
            return Vec3::new(c.x, lo.y, c.z);
        }
        let s = &self.rig.skeleton;
        s.ids()
            .find(|b| s.bone(*b).parent().is_none())
            .map_or(Vec3::ZERO, |b| s.rest_head_tail(b).0)
    }

    fn manipulator_radius(&self) -> f32 {
        self.manipulator.size * self.pixels_per_point.max(0.25)
    }

    /// Formes à dessiner (pixels physiques) ; `false` si aucun manipulateur.
    pub fn manipulator_geometry(&self, vp: &Viewport, out: &mut Geometry) -> bool {
        out.clear();
        let Some((_, frame, axis_scale)) = self.manipulator_target() else {
            return false;
        };
        let options = Options {
            mode: self.manipulator.mode,
            radius_px: self.manipulator_radius(),
            axis_scale,
        };
        out.build(&Projector::new(&self.camera, vp), &frame, &options)
    }

    /// Partie en cours de manipulation.
    pub fn manipulator_active(&self) -> Option<Part> {
        match &self.drag {
            Some(Drag::Manip(g)) => Some(g.drag.part),
            _ => None,
        }
    }

    /// Valeur affichée pendant le geste (« X +0.250 m », « +45.0° », « ×1.20 »)
    /// et position du doigt.
    pub fn manipulator_readout(&self) -> Option<(String, Vec2)> {
        match &self.drag {
            Some(Drag::Manip(g)) if self.drag_moved => Some((g.drag.readout(), g.drag.last_pos)),
            _ => None,
        }
    }

    pub fn set_manipulator_mode(&mut self, mode: ManipulatorMode) {
        self.manipulator.mode = mode;
        self.manipulator.visible = true;
    }

    pub fn toggle_manipulator_space(&mut self) {
        self.manipulator.space = match self.manipulator.space {
            TransformSpace::World => TransformSpace::Local,
            TransformSpace::Local => TransformSpace::World,
        };
    }

    pub fn toggle_manipulator_snap(&mut self) {
        self.manipulator.snap = !self.manipulator.snap;
    }

    pub fn toggle_manipulator(&mut self) {
        self.manipulator.visible = !self.manipulator.visible;
    }

    /// Saisie d'une poignée du manipulateur (essayée avant les os).
    pub(super) fn manipulator_down(&mut self, pos: Vec2, vp: &Viewport, radius: f32) -> bool {
        let Some((target, frame, axis_scale)) = self.manipulator_target() else {
            return false;
        };
        let projector = Projector::new(&self.camera, vp);
        let radius_px = self.manipulator_radius();
        let mut geometry = Geometry::default();
        let options = Options {
            mode: self.manipulator.mode,
            radius_px,
            axis_scale,
        };
        if !geometry.build(&projector, &frame, &options) {
            return false;
        }
        let Some(part) = geometry.pick(pos, radius * 0.8) else {
            return false;
        };
        self.checkpoint();
        if self.mode == Mode::Pose {
            self.snap_limbs();
        }
        let drag = ManipDrag::begin(part, frame, projector, pos, radius_px);
        let gesture = self.start_gesture(drag, target, *vp);
        self.drag = Some(Drag::Manip(Box::new(gesture)));
        true
    }

    fn start_gesture(&self, drag: ManipDrag, target: Target, viewport: Viewport) -> ManipGesture {
        let skeleton = &self.rig.skeleton;
        let mut gesture = ManipGesture {
            drag,
            target,
            start_pose: Transform::IDENTITY,
            start_head: drag.frame.origin,
            start_len: 0.0,
            start_rot: Quat::IDENTITY,
            follow: Follow::Delta,
            viewport,
        };
        let Target::Bone(bone) = target else {
            return gesture;
        };
        gesture.start_pose = self.rig.pose.get(bone);
        gesture.start_len = skeleton.bone(bone).length;
        gesture.start_rot = skeleton.rest_world_rotation(bone);
        if self.mode == Mode::Pose && drag.part.is_translation() {
            let limb = self
                .rig
                .limbs
                .iter()
                .position(|l| l.mode == LimbMode::Ik && l.contains(bone) && l.is_valid(skeleton));
            gesture.follow = match (limb, skeleton.bone(bone).parent()) {
                (Some(i), _) => Follow::Limb(i, self.rig.limbs[i].target),
                (None, Some(parent)) if skeleton.is_connected(bone) => {
                    Follow::Chain(skeleton.chain(parent, self.chain_len))
                }
                _ => Follow::Delta,
            };
        }
        gesture
    }

    /// Fin de geste. Un simple toucher sans effet ne crée pas d'étape
    /// d'annulation et sélectionne l'os éventuellement situé dessous.
    pub(super) fn manipulator_up(&mut self, gesture: &ManipGesture) {
        let tap = gesture.drag.travel() < 6.0 * self.pixels_per_point.max(0.25);
        if !self.drag_moved || (tap && gesture.drag.last.is_identity()) {
            self.drop_checkpoint(true);
            let (at, radius) = (gesture.drag.last_pos, 12.0 * self.pixels_per_point);
            if let Some(bone) = self.pick(at, &gesture.viewport, radius) {
                self.selected = Some(bone);
            }
            return;
        }
        match (self.mode, gesture.target) {
            (Mode::Pose, _) => {
                self.snap_limbs();
                self.store_key_if_any();
            }
            // Les marqueurs d'auto-rig ne correspondent plus au modèle : ils
            // seront de nouveau estimés.
            (_, Target::Model) => self.markers = None,
            _ => {}
        }
    }
}
