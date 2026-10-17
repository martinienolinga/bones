//! Lecture bornée des entrées d'un conteneur. Les fichiers peuvent venir
//! d'ailleurs (partage, AirDrop) : les tailles décompressées sont limitées,
//! entrée par entrée et au total, quoi qu'annonce le conteneur.

use std::fs::File;
use std::io::{BufReader, Read, Seek, Write};
use std::path::Path;

use bytemuck::Pod;
use zip::result::ZipError;
use zip::{ZipArchive, ZipWriter};

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

pub(crate) struct Archive {
    zip: ZipArchive<BufReader<File>>,
    /// Octets décompressés restant autorisés.
    budget: u64,
}

impl Archive {
    pub fn open(path: &Path) -> Result<Self, ProjectError> {
        let file = BufReader::with_capacity(1 << 16, File::open(path)?);
        let zip = ZipArchive::new(file).map_err(|e| match e {
            ZipError::InvalidArchive(_) | ZipError::UnsupportedArchive(_) => {
                invalid("unrecognized file format")
            }
            e => e.into(),
        })?;
        Ok(Self {
            zip,
            budget: MAX_TOTAL,
        })
    }

    /// Lit une entrée entière (au plus `max` octets) ; `None` si absente.
    pub fn read(&mut self, name: &str, max: u64) -> Result<Option<Vec<u8>>, ProjectError> {
        let mut entry = match self.zip.by_name(name) {
            Ok(entry) => entry,
            Err(ZipError::FileNotFound) => return Ok(None),
            Err(e) => return Err(e.into()),
        };
        let max = max.min(self.budget);
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
        self.budget -= out.len() as u64;
        Ok(Some(out))
    }

    /// Lit un tableau binaire de `count` éléments (taille exacte attendue).
    /// Une entrée absente n'est acceptée que pour un tableau vide.
    pub fn read_array<T: Pod>(&mut self, name: &str, count: u32) -> Result<Vec<T>, ProjectError> {
        if count == 0 {
            return Ok(Vec::new());
        }
        let expected = u64::from(count) * size_of::<T>() as u64;
        if expected > self.budget {
            return Err(too_large(name, expected));
        }
        let mut entry = match self.zip.by_name(name) {
            Ok(entry) => entry,
            Err(ZipError::FileNotFound) => return Err(invalid(format!("missing entry {name}"))),
            Err(e) => return Err(e.into()),
        };
        if entry.size() != expected {
            return Err(invalid(format!("entry {name} has an unexpected size")));
        }
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
        self.budget -= expected;
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
