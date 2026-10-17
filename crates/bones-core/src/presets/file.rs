//! Format natif des préréglages `.kb3da` : un conteneur ZIP (le même que les
//! projets `.kbrig`, [`crate::project::container`]) avec un manifeste lisible
//! seul (galerie) et le préréglage sérialisé. L'écriture est atomique ; la
//! lecture est bornée et vérifiée (les fichiers reçus d'ailleurs ne peuvent ni
//! épuiser la mémoire ni faire paniquer), et un fichier d'une version plus
//! récente est refusé avec un message clair.
//!
//! | entrée          | contenu                                   | compression |
//! |-----------------|-------------------------------------------|-------------|
//! | `manifest.json` | format, version, fiche ([`PresetInfo`])   | deflate     |
//! | `preset.json`   | préréglage complet ([`Preset`])           | deflate     |

use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::error::{PresetError, invalid};
use super::model::{Preset, PresetInfo};
use crate::project::container::{Archive, MAX_JSON, deflate, finish, write_atomically, writer};

/// Extension des fichiers de préréglages (et des animations exportées).
pub const EXTENSION: &str = "kb3da";

/// Identifiant du format, dans le manifeste.
const FORMAT: &str = "paintsharp-bones-preset";
/// Version du format écrite ; les fichiers plus récents sont refusés.
const VERSION: u32 = 1;

const MANIFEST: &str = "manifest.json";
const PRESET: &str = "preset.json";
/// Taille maximale du manifeste décompressé.
const MAX_MANIFEST: u64 = 1 << 20;

/// Manifeste : petit résumé lisible sans décompresser le préréglage.
#[derive(Serialize, Deserialize)]
struct Manifest {
    format: String,
    version: u32,
    #[serde(default)]
    app_version: String,
    info: PresetInfo,
}

/// Enregistre un préréglage dans un fichier `.kb3da` (écriture atomique).
pub fn save(preset: &Preset, path: &Path) -> Result<(), PresetError> {
    save_versioned(preset, path, VERSION)
}

fn save_versioned(preset: &Preset, path: &Path, version: u32) -> Result<(), PresetError> {
    let manifest = Manifest {
        format: FORMAT.into(),
        version,
        app_version: env!("CARGO_PKG_VERSION").into(),
        info: preset.info.clone(),
    };
    let manifest = deflate(MANIFEST, &serde_json::to_vec(&manifest)?)?;
    let body = deflate(PRESET, &serde_json::to_vec(preset)?)?;
    write_atomically(path, |file| {
        let mut zip = writer(file);
        zip.add_prepared_file(manifest)?;
        zip.add_prepared_file(body)?;
        finish(zip)
    })?;
    Ok(())
}

/// Ouvre un préréglage `.kb3da` depuis le disque.
pub fn load(path: &Path) -> Result<Preset, PresetError> {
    load_archive(Archive::open(path)?)
}

/// Ouvre un préréglage `.kb3da` déjà en mémoire (préréglage embarqué, tests).
pub fn load_bytes(bytes: &[u8]) -> Result<Preset, PresetError> {
    load_archive(Archive::from_bytes(Arc::from(bytes))?)
}

/// Lit la seule fiche d'un préréglage (galerie), sans décompresser le contenu.
pub fn read_info(path: &Path) -> Result<PresetInfo, PresetError> {
    Ok(read_manifest(&mut Archive::open(path)?)?.info)
}

fn load_archive(mut archive: Archive) -> Result<Preset, PresetError> {
    read_manifest(&mut archive)?;
    let json = archive
        .read(PRESET, MAX_JSON)?
        .ok_or_else(|| invalid("missing preset data"))?;
    let preset: Preset = serde_json::from_slice(&json)?;
    validate(&preset)?;
    Ok(preset)
}

fn read_manifest(archive: &mut Archive) -> Result<Manifest, PresetError> {
    let not_preset = || invalid("not a PaintSharp Bones preset");
    let bytes = archive.read(MANIFEST, MAX_MANIFEST)?.ok_or_else(not_preset)?;
    let manifest: Manifest = serde_json::from_slice(&bytes).map_err(|_| not_preset())?;
    if manifest.format != FORMAT {
        return Err(not_preset());
    }
    if manifest.version > VERSION {
        return Err(PresetError::TooNew {
            version: manifest.version,
            supported: VERSION,
        });
    }
    Ok(manifest)
}

/// Écrit un préréglage avec un numéro de format d'une version plus récente
/// (tests du refus d'un fichier trop neuf).
#[cfg(test)]
pub(crate) fn save_future(preset: &Preset, path: &Path) -> Result<(), PresetError> {
    save_versioned(preset, path, VERSION + 1)
}

/// Cohérence minimale d'un préréglage reçu d'ailleurs : les rôles d'un
/// préréglage à clés doivent recouvrir son squelette.
fn validate(preset: &Preset) -> Result<(), PresetError> {
    if let super::model::PresetKind::Keyframes(data) = &preset.kind
        && !data.roles.fits(&data.skeleton)
    {
        return Err(invalid("preset bone roles do not match its skeleton"));
    }
    Ok(())
}
