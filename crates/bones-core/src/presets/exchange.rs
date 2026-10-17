//! Import et export de préréglages via les formats d'échange : import des
//! animations d'un fichier BVH, glTF/GLB ou FBX (un préréglage par animation),
//! export d'un préréglage en `.kb3da` natif, BVH ou glTF. Les fichiers reçus
//! d'ailleurs sont lus avec une taille bornée.
//!
//! Un préréglage exporté en BVH ou glTF est cuit sur son squelette de
//! référence (procédural) ou enregistré sur son squelette source (clés), sans
//! maillage : c'est une animation de squelette, lisible par les autres outils.

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use super::create::travel;
use super::error::{PresetError, incompatible, invalid};
use super::file;
use super::model::{Category, KeyframeData, Preset, PresetInfo, PresetKind, PresetSource, new_id};
use super::procedural;
use super::roles::detect;
use crate::io::{self, Format};
use crate::project::{store, unix_now};
use crate::{Clip, Mesh, Rig, Skeleton};

/// Taille maximale d'un fichier d'échange importé (décodé par le parseur).
const MAX_IMPORT: u64 = 256 << 20;

/// Importe les préréglages d'un fichier (natif `.kb3da` ou format d'échange).
pub fn import_file(path: &Path) -> Result<Vec<Preset>, PresetError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_lowercase();
    if ext == file::EXTENSION {
        return Ok(vec![file::load(path)?]);
    }
    let format = Format::from_path(path).ok_or_else(|| invalid("unsupported file type"))?;
    let hint = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Imported".into());
    let bytes = read_bounded(path)?;
    import_bytes(format, &bytes, path.parent(), &hint)
}

/// Importe les préréglages d'un fichier déjà en mémoire. `hint` nomme le
/// préréglage quand le fichier ne contient qu'une animation (nom de fichier).
pub fn import_bytes(
    format: Format,
    bytes: &[u8],
    base: Option<&Path>,
    hint: &str,
) -> Result<Vec<Preset>, PresetError> {
    let (skeleton, clips) = io::read_animations(format, bytes, base)?;
    if skeleton.is_empty() {
        return Err(incompatible("the file has no skeleton"));
    }
    if clips.is_empty() {
        return Err(PresetError::Empty("the file has no animation".into()));
    }
    let single = clips.len() == 1;
    let kind = format.extension().to_uppercase();
    Ok(clips
        .into_iter()
        .map(|(name, clip)| {
            let label = if single || name.trim().is_empty() {
                hint
            } else {
                &name
            };
            from_clip(&skeleton, label, clip, &kind)
        })
        .collect())
}

/// Enregistre un préréglage dans un fichier, d'après son extension : `.kb3da`
/// natif, sinon BVH ou glTF/GLB (animation de squelette). Retourne les
/// fichiers écrits.
pub fn export_file(preset: &Preset, path: &Path) -> Result<Vec<PathBuf>, PresetError> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or_default()
        .to_lowercase();
    if ext == file::EXTENSION {
        file::save(preset, path)?;
        return Ok(vec![path.to_owned()]);
    }
    let format = Format::from_path(path).ok_or_else(|| invalid("unsupported file type"))?;
    if !matches!(format, Format::Bvh | Format::Glb | Format::Gltf) {
        return Err(PresetError::Exchange(crate::IoError::Unsupported(format!(
            "{} preset export",
            format.extension().to_uppercase()
        ))));
    }
    let rig = to_rig(preset)?;
    Ok(io::export(&rig, format, path)?)
}

/// Préréglage à clés tiré d'une animation importée.
fn from_clip(skeleton: &Skeleton, name: &str, mut clip: Clip, format: &str) -> Preset {
    clip.sync_len(skeleton);
    let roles = detect(skeleton);
    let (fps, frames) = (clip.fps, clip.frame_count.saturating_sub(1));
    let rig_type = roles.rig_type;
    let data = KeyframeData {
        skeleton: skeleton.clone(),
        roles,
        clip,
        bones: Vec::new(),
    };
    let root_motion = travel(&data) > 0.01 * data.roles.frame.size.max(1e-3);
    let now = unix_now();
    let info = PresetInfo {
        id: new_id("imported"),
        name: store::sanitize_name(name),
        category: Category::for_rig(rig_type),
        source: PresetSource::Imported {
            format: format.to_owned(),
        },
        rig_type,
        fps,
        frames,
        root_motion,
        bones: skeleton.len() as u32,
        created: now,
        modified: now,
        ..PresetInfo::default()
    };
    Preset {
        info,
        kind: PresetKind::Keyframes(Box::new(data)),
    }
}

/// Squelette animé (sans maillage) représentant le préréglage, pour l'export
/// vers un format d'échange.
fn to_rig(preset: &Preset) -> Result<Rig, PresetError> {
    let (skeleton, clip) = match &preset.kind {
        PresetKind::Keyframes(d) => (d.skeleton.clone(), d.clip.clone()),
        PresetKind::Procedural(p) => {
            let skeleton = p.reference().skeleton();
            let roles = detect(&skeleton);
            let clip = procedural::bake(
                p.generator,
                &p.params,
                &skeleton,
                &roles,
                preset.info.fps.max(1.0),
                1.0,
                1,
            )?;
            (skeleton, clip)
        }
        // Expression faciale : pose unique sur une tête de référence.
        PresetKind::Expression(data) => {
            let skeleton = super::expression::reference_head();
            let (pose, _) = super::expression::face_pose(data, &skeleton, 1.0, false);
            let mut clip = Clip::new(preset.info.fps.max(1.0), 2);
            clip.set_key(0, pose);
            (skeleton, clip)
        }
    };
    if skeleton.is_empty() {
        return Err(incompatible("the preset has no skeleton"));
    }
    let mut rig = Rig::new(preset.info.name.clone(), Mesh::default());
    rig.set_skeleton(skeleton, Vec::new());
    *rig.clip_mut() = clip;
    Ok(rig)
}

/// Lit un fichier entier avec une taille bornée (fichier reçu d'ailleurs).
fn read_bounded(path: &Path) -> Result<Vec<u8>, PresetError> {
    let mut file = File::open(path)?;
    let mut out = Vec::new();
    file.by_ref()
        .take(MAX_IMPORT + 1)
        .read_to_end(&mut out)
        .map_err(PresetError::Io)?;
    if out.len() as u64 > MAX_IMPORT {
        return Err(invalid("file too large"));
    }
    Ok(out)
}
