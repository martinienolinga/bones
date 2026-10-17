//! Rig : squelette, pose, symétrie et document de travail.

mod demo;
mod document;
mod edit;
pub mod mirror;
mod model;
mod pose;
pub mod skeleton;
#[cfg(test)]
mod tests;

pub use document::{Revisions, Rig, next_revision};
pub use pose::Pose;
pub use skeleton::{Bone, BoneId, BoneRemap, Skeleton, SkeletonError, head_tail};
