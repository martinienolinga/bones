//! Fichiers `.bones` : document JSON versionné.

use std::io::{Read, Write};

use serde::{Deserialize, Serialize};

use crate::Rig;
use crate::rig::Revisions;

const FORMAT: &str = "paintsharp-bones";
const VERSION: u32 = 1;

#[derive(Serialize, Deserialize)]
struct RigFile<T> {
    format: String,
    version: u32,
    rig: T,
}

#[derive(Debug)]
pub enum RigError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Format(String),
}

impl std::fmt::Display for RigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RigError::Io(e) => write!(f, "input/output error: {e}"),
            RigError::Json(e) => write!(f, "invalid file: {e}"),
            RigError::Format(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for RigError {}

impl From<std::io::Error> for RigError {
    fn from(e: std::io::Error) -> Self {
        RigError::Io(e)
    }
}

impl From<serde_json::Error> for RigError {
    fn from(e: serde_json::Error) -> Self {
        RigError::Json(e)
    }
}

impl Rig {
    pub fn save(&self, writer: impl Write) -> Result<(), RigError> {
        let file = RigFile {
            format: FORMAT.into(),
            version: VERSION,
            rig: self,
        };
        serde_json::to_writer(std::io::BufWriter::new(writer), &file)?;
        Ok(())
    }

    pub fn load(reader: impl Read) -> Result<Rig, RigError> {
        let file: RigFile<Rig> = serde_json::from_reader(std::io::BufReader::new(reader))?;
        if file.format != FORMAT {
            return Err(RigError::Format(format!("unknown format: {}", file.format)));
        }
        if file.version > VERSION {
            return Err(RigError::Format(format!(
                "file version {} is too recent (max {VERSION})",
                file.version
            )));
        }
        let mut rig = file.rig;
        rig.revisions = Revisions::default();
        rig.pose.sync_len(&rig.skeleton);
        if rig
            .clip
            .keys()
            .iter()
            .any(|k| k.pose.bones.len() != rig.skeleton.len())
        {
            let skeleton = rig.skeleton.clone();
            rig.clip_mut().sync_len(&skeleton);
        }
        let n = rig.skeleton.len();
        let skin_ok = rig.skin.weights.len() == rig.mesh.vertex_count()
            && rig
                .skin
                .weights
                .iter()
                .all(|w| w.iter().all(|(b, _)| b.index() < n));
        if !skin_ok {
            // Poids incohérents : on force une nouvelle liaison.
            rig.touch_skeleton();
        }
        rig.limbs.retain(|l| l.is_valid(&rig.skeleton));
        let world = rig.world();
        for limb in &mut rig.limbs {
            limb.snap_to_pose(&rig.skeleton, &world);
        }
        rig.pins.retain(|p| rig.skeleton.contains(p.bone));
        Ok(rig)
    }
}
