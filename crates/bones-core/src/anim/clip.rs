//! Clip d'animation : poses-clés triées par image et échantillonnage.

use glam::Vec3;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use super::interpolate::{blend_into, catmull_rom, copy_pose};
use crate::{BoneId, BoneRemap, Pose, Skeleton, Transform, head_tail};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Interpolation {
    /// Maintien de la pose jusqu'à la clé suivante (blocking).
    Step,
    Linear,
    /// Courbe de Catmull-Rom passant par les clés voisines.
    #[default]
    Smooth,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Key {
    pub frame: u32,
    pub pose: Pose,
    /// Interpolation de l'intervalle qui commence à cette clé.
    pub interpolation: Interpolation,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Clip {
    pub fps: f32,
    /// Nombre d'images (la dernière image est `frame_count - 1`).
    pub frame_count: u32,
    /// Clés triées par image, sans doublon.
    keys: Vec<Key>,
}

impl Default for Clip {
    fn default() -> Self {
        Self {
            fps: 30.0,
            frame_count: 60,
            keys: Vec::new(),
        }
    }
}

impl Clip {
    pub fn new(fps: f32, frame_count: u32) -> Self {
        Self {
            fps,
            frame_count,
            keys: Vec::new(),
        }
    }

    pub fn keys(&self) -> &[Key] {
        &self.keys
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    pub fn key_at(&self, frame: u32) -> Option<&Key> {
        self.keys
            .binary_search_by_key(&frame, |k| k.frame)
            .ok()
            .map(|i| &self.keys[i])
    }

    pub fn key_at_mut(&mut self, frame: u32) -> Option<&mut Key> {
        self.keys
            .binary_search_by_key(&frame, |k| k.frame)
            .ok()
            .map(|i| &mut self.keys[i])
    }

    /// Enregistre (ou remplace) la pose à l'image `frame`.
    pub fn set_key(&mut self, frame: u32, pose: Pose) {
        self.frame_count = self.frame_count.max(frame + 1);
        match self.keys.binary_search_by_key(&frame, |k| k.frame) {
            Ok(i) => self.keys[i].pose = pose,
            Err(i) => {
                let interpolation = i
                    .checked_sub(1)
                    .map(|p| self.keys[p].interpolation)
                    .unwrap_or_default();
                self.keys.insert(
                    i,
                    Key {
                        frame,
                        pose,
                        interpolation,
                    },
                )
            }
        }
    }

    pub fn remove_key(&mut self, frame: u32) -> bool {
        match self.keys.binary_search_by_key(&frame, |k| k.frame) {
            Ok(i) => {
                self.keys.remove(i);
                true
            }
            Err(_) => false,
        }
    }

    pub fn prev_key_frame(&self, frame: u32) -> Option<u32> {
        self.keys.iter().rev().map(|k| k.frame).find(|f| *f < frame)
    }

    pub fn next_key_frame(&self, frame: u32) -> Option<u32> {
        self.keys.iter().map(|k| k.frame).find(|f| *f > frame)
    }

    /// Intervalle de clés `[début, fin]` contenant `frame`.
    pub fn interval_at(&self, frame: u32) -> Option<(u32, u32)> {
        let i = self.keys.partition_point(|k| k.frame <= frame);
        if i == 0 || i >= self.keys.len() {
            return None;
        }
        Some((self.keys[i - 1].frame, self.keys[i].frame))
    }

    /// Pose à une image (éventuellement fractionnaire). Sans clé, laisse
    /// `out` intact.
    pub fn sample_into(&self, frame: f32, out: &mut Pose) {
        let Some(first) = self.keys.first() else { return };
        let last = self.keys.last().unwrap();
        if frame <= first.frame as f32 {
            copy_pose(&first.pose, out);
            return;
        }
        if frame >= last.frame as f32 {
            copy_pose(&last.pose, out);
            return;
        }
        let i = self.keys.partition_point(|k| k.frame as f32 <= frame);
        let (k1, k2) = (&self.keys[i - 1], &self.keys[i]);
        let t = (frame - k1.frame as f32) / (k2.frame - k1.frame) as f32;
        match k1.interpolation {
            Interpolation::Step => copy_pose(&k1.pose, out),
            Interpolation::Linear => blend_into(&k1.pose, &k2.pose, t, out),
            Interpolation::Smooth => {
                let k0 = if i >= 2 { &self.keys[i - 2] } else { k1 };
                let k3 = self.keys.get(i + 1).unwrap_or(k2);
                let len = out.bones.len().max(k1.pose.bones.len());
                out.bones.resize(len, Transform::IDENTITY);
                for (b, slot) in out.bones.iter_mut().enumerate() {
                    let id = BoneId(b as u32);
                    *slot = catmull_rom(
                        &k0.pose.get(id),
                        &k1.pose.get(id),
                        &k2.pose.get(id),
                        &k3.pose.get(id),
                        t,
                    );
                }
            }
        }
    }

    /// Trajectoire de la queue d'un os sur toutes les images (calcul
    /// parallèle, une image par tâche).
    pub fn trajectory(&self, skeleton: &Skeleton, bone: BoneId) -> Vec<Vec3> {
        (0..self.frame_count)
            .into_par_iter()
            .map_init(
                || Pose::rest(skeleton),
                |pose, f| {
                    self.sample_into(f as f32, pose);
                    let world = pose.world(skeleton);
                    head_tail(&world[bone.index()], skeleton.bone(bone).length).1
                },
            )
            .collect()
    }

    /// Ajoute un os à toutes les clés (pose identité).
    pub fn push_bone(&mut self) {
        for k in &mut self.keys {
            k.pose.bones.push(Transform::IDENTITY);
        }
    }

    pub fn remap(&mut self, remap: &BoneRemap) {
        for k in &mut self.keys {
            k.pose.remap(remap);
        }
    }

    /// Recadre toutes les clés sur la taille du squelette.
    pub fn sync_len(&mut self, skeleton: &Skeleton) {
        for k in &mut self.keys {
            k.pose.sync_len(skeleton);
        }
    }
}
