//! Timeline : images, clés, lecture, interpolation et physique.

use super::{Editor, Mode};
use crate::Interpolation;
use crate::anim::physics;

impl Editor {
    pub fn set_frame(&mut self, frame: f32) {
        let last = self.rig.clip.frame_count.saturating_sub(1) as f32;
        self.frame = frame.clamp(0.0, last);
        if !self.rig.clip.is_empty() {
            self.rig.clip.sample_into(self.frame, &mut self.rig.pose);
            self.snap_limbs();
        }
    }

    /// Avance la lecture ; retourne `true` si l'affichage doit être rafraîchi.
    pub fn update(&mut self, dt: f32) -> bool {
        if !self.playing || self.mode != Mode::Pose {
            return false;
        }
        let count = self.rig.clip.frame_count.max(1) as f32;
        let next = (self.frame + dt * self.rig.clip.fps) % count;
        self.set_frame(next);
        true
    }

    pub fn toggle_play(&mut self) {
        self.playing = !self.playing && self.mode == Mode::Pose;
    }

    pub fn add_key(&mut self) {
        self.checkpoint();
        let frame = self.current_frame();
        let pose = self.rig.pose.clone();
        self.rig.clip_mut().set_key(frame, pose);
        self.frame = frame as f32;
    }

    pub fn delete_key(&mut self) {
        let frame = self.current_frame();
        if self.rig.clip.key_at(frame).is_none() {
            return;
        }
        self.checkpoint();
        self.rig.clip_mut().remove_key(frame);
    }

    pub fn jump_key(&mut self, forward: bool) {
        let frame = self.current_frame();
        let target = if forward {
            self.rig.clip.next_key_frame(frame)
        } else {
            self.rig.clip.prev_key_frame(frame)
        };
        if let Some(f) = target {
            self.set_frame(f as f32);
        }
    }

    /// Interpolation de l'intervalle commençant à la clé courante ou précédente.
    pub fn interpolation(&self) -> Option<Interpolation> {
        let frame = self.current_frame();
        let start = if self.rig.clip.key_at(frame).is_some() {
            frame
        } else {
            self.rig.clip.prev_key_frame(frame)?
        };
        self.rig.clip.key_at(start).map(|k| k.interpolation)
    }

    pub fn set_interpolation(&mut self, interpolation: Interpolation) {
        let frame = self.current_frame();
        let start = if self.rig.clip.key_at(frame).is_some() {
            Some(frame)
        } else {
            self.rig.clip.prev_key_frame(frame)
        };
        let Some(start) = start else { return };
        self.checkpoint();
        if let Some(k) = self.rig.clip_mut().key_at_mut(start) {
            k.interpolation = interpolation;
        }
        self.set_frame(self.frame);
    }

    pub fn set_frame_count(&mut self, count: u32) {
        let count = count.clamp(2, 100_000);
        if count == self.rig.clip.frame_count {
            return;
        }
        self.checkpoint();
        let last_key = self.rig.clip.keys().last().map(|k| k.frame + 1).unwrap_or(0);
        self.rig.clip_mut().frame_count = count.max(last_key);
        self.set_frame(self.frame);
    }

    /// Trajectoire balistique sur l'intervalle de clés contenant l'image courante.
    pub fn bake_ballistic(&mut self) {
        let Some((start, end)) = self.rig.clip.interval_at(self.current_frame()) else {
            self.status = Some("Stand between two keys (take-off and landing)".into());
            return;
        };
        self.checkpoint();
        let masses = physics::bone_masses(&self.rig.skeleton, &self.rig.skin);
        let skeleton = self.rig.skeleton.clone();
        let result = physics::bake_ballistic(
            &skeleton,
            self.rig.clip_mut(),
            &masses,
            start,
            end,
            physics::GRAVITY,
        );
        self.status = Some(match result {
            Ok(n) => format!("Ballistic jump: {n} frames computed"),
            Err(e) => e.to_string(),
        });
        self.set_frame(self.frame);
    }
}
