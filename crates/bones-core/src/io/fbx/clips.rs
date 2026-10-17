//! Toutes les piles d'animation d'un FBX binaire (prises Maya, clips
//! Mixamo, Cascadeur…) sur son squelette, sans lire les maillages.

use std::io::Cursor;

use fbxcel::tree::any::AnyTree;

use super::document::Document;
use super::models::Models;
use super::{anim, err, skin, transform};
use crate::io::IoError;
use crate::io::rigging::build_skeleton;
use crate::{Clip, Skeleton};

/// Squelette et animations nommées.
pub(crate) fn animations(bytes: &[u8]) -> Result<(Skeleton, Vec<(String, Clip)>), IoError> {
    if bytes.starts_with(b"; FBX") {
        return Err(IoError::Unsupported("ASCII FBX (re-export it as binary)".into()));
    }
    let tree = match AnyTree::from_seekable_reader(Cursor::new(bytes)).map_err(err)? {
        AnyTree::V7400(_, tree, _) => tree,
        _ => return Err(IoError::Unsupported("unknown FBX version".into())),
    };
    let doc = Document::new(&tree);
    let (axis, fps) = transform::axis_system(&doc);
    let models = Models::read(&doc);
    let rigging = skin::rigging(&doc, &models, axis).ok_or_else(|| err("no skeleton"))?;
    let built = build_skeleton(&rigging.joints);
    let clips = anim::sample_all(&doc, &models, &rigging, axis, fps, &built);
    Ok((built.skeleton, clips))
}
