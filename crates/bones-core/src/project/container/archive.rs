//! Lecture bornée des entrées d'un conteneur. Les fichiers peuvent venir
//! d'ailleurs (partage, AirDrop) : les tailles décompressées sont limitées,
//! entrée par entrée et au total, quoi qu'annonce le conteneur.

use std::fs::File;
use std::io::{Read, Seek, Write};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use bytemuck::Pod;
use zip::read::{ArchiveOffset, Config};
use zip::result::ZipError;
use zip::{ZipArchive, ZipWriter};

use super::shared_file::SharedFile;
use crate::project::ProjectError;
use crate::project::error::invalid;

/// Document JSON.
pub(crate) const MAX_JSON: u64 = 256 << 20;
pub(crate) const MAX_MANIFEST: u64 = 1 << 20;
pub(crate) const MAX_THUMBNAIL: u64 = 16 << 20;
pub(crate) const MAX_TEXTURE: u64 = 256 << 20;
/// Total décompressé d'un projet (les entrées compressées peuvent
/// s'étendre jusqu'à mille fois : la limite doit rester supportable par un iPad).
pub(crate) const MAX_TOTAL: u64 = 1536 << 20;

/// Conteneur ouvert en lecture. Cloner un `Archive` est peu coûteux (même
/// fichier ouvert, répertoire déjà lu, plafond partagé) : chaque fil lit
/// ainsi ses entrées en parallèle.
#[derive(Clone)]
pub(crate) struct Archive {
    zip: ZipArchive<SharedFile>,
    /// Octets décompressés restant autorisés, pour tous les clones.
    budget: Arc<AtomicU64>,
}

impl Archive {
    pub fn open(path: &Path) -> Result<Self, ProjectError> {
        Self::with_source(SharedFile::new(File::open(path)?)?)
    }

    /// Conteneur déjà en mémoire (projet embarqué dans l'application).
    pub fn from_bytes(bytes: Arc<[u8]>) -> Result<Self, ProjectError> {
        Self::with_source(SharedFile::memory(bytes))
    }

    fn with_source(file: SharedFile) -> Result<Self, ProjectError> {
        // Conteneur toujours au début du fichier : la recherche (coûteuse)
        // d'un décalage éventuel est inutile.
        let config = Config {
            archive_offset: ArchiveOffset::Known(0),
        };
        let zip = ZipArchive::with_config(config, file).map_err(|e| match e {
            ZipError::InvalidArchive(_) | ZipError::UnsupportedArchive(_) => {
                invalid("unrecognized file format")
            }
            e => e.into(),
        })?;
        Ok(Self {
            zip,
            budget: Arc::new(AtomicU64::new(MAX_TOTAL)),
        })
    }

    /// Lit une entrée entière (au plus `max` octets) ; `None` si absente.
    pub fn read(&mut self, name: &str, max: u64) -> Result<Option<Vec<u8>>, ProjectError> {
        let mut entry = match self.zip.by_name(name) {
            Ok(entry) => entry,
            Err(ZipError::FileNotFound) => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let max = max.min(self.budget.load(Ordering::Relaxed));
        let declared = entry.size();
        if declared > max {
            return Err(too_large(name, declared));
        }
        // La taille annoncée peut mentir : la lecture est bornée elle aussi.
        // Réservation plafonnée : une taille annoncée mensongère ne fait pas
        // allouer d'avance une mémoire démesurée.
        let mut out = Vec::with_capacity(declared.min(64 << 20) as usize);
        (&mut entry)
            .take(max + 1)
            .read_to_end(&mut out)
            .map_err(|e| damaged(name, e))?;
        if out.len() as u64 > max {
            return Err(too_large(name, out.len() as u64));
        }
        spend(&self.budget, name, out.len() as u64)?;
        Ok(Some(out))
    }

    /// Lit un tableau binaire de `count` éléments (taille exacte attendue).
    /// Une entrée absente n'est acceptée que pour un tableau vide.
    pub fn read_array<T: Pod>(&mut self, name: &str, count: u32) -> Result<Vec<T>, ProjectError> {
        if count == 0 {
            return Ok(Vec::new());
        }
        let expected = u64::from(count) * size_of::<T>() as u64;
        let mut entry = match self.zip.by_name(name) {
            Ok(entry) => entry,
            Err(ZipError::FileNotFound) => return Err(invalid(format!("missing entry {name}"))),
            Err(e) => return Err(e.into()),
        };
        if entry.size() != expected {
            return Err(invalid(format!("entry {name} has an unexpected size")));
        }
        // Plafond prélevé avant d'allouer.
        spend(&self.budget, name, expected)?;
        let mut out: Vec<T> = Vec::new();
        out.try_reserve_exact(count as usize)
            .map_err(|_| too_large(name, expected))?;
        out.resize(count as usize, T::zeroed());
        entry
            .read_exact(bytemuck::cast_slice_mut(&mut out))
            .map_err(|e| damaged(name, e))?;
        // Lecture jusqu'à la fin : vérifie qu'il ne reste rien, et la somme CRC.
        let mut extra = [0u8; 1];
        if entry.read(&mut extra).map_err(|e| damaged(name, e))? != 0 {
            return Err(invalid(format!("entry {name} is longer than declared")));
        }
        Ok(out)
    }
}

impl Archive {
    /// Copie toutes les entrées, sauf `skip`, telles quelles (sans les
    /// décompresser ni les recompresser).
    pub fn copy_raw<W: Write + Seek>(
        &mut self,
        zip: &mut ZipWriter<W>,
        skip: &str,
    ) -> Result<(), ProjectError> {
        for i in 0..self.zip.len() {
            let entry = self.zip.by_index_raw(i)?;
            if entry.name()? != skip {
                zip.raw_copy_file(entry)?;
            }
        }
        Ok(())
    }
}

/// Prélève `size` octets sur le plafond total.
fn spend(budget: &AtomicU64, name: &str, size: u64) -> Result<(), ProjectError> {
    budget
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |b| b.checked_sub(size))
        .map(|_| ())
        .map_err(|_| too_large(name, size))
}

fn too_large(name: &str, size: u64) -> ProjectError {
    ProjectError::TooLarge(format!("entry {name} ({size} bytes)"))
}

fn damaged(name: &str, e: std::io::Error) -> ProjectError {
    match e.kind() {
        std::io::ErrorKind::UnexpectedEof | std::io::ErrorKind::InvalidData => {
            invalid(format!("entry {name} is damaged: {e}"))
        }
        _ => e.into(),
    }
}
