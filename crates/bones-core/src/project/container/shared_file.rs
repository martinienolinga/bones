//! Lecteur positionnel d'un fichier partagé entre plusieurs fils : chacun a
//! sa propre position et lit le même fichier ouvert, même si un
//! enregistrement le remplace entre-temps sur le disque. Le contenu peut
//! aussi être déjà en mémoire (projet embarqué dans l'application).

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::sync::Arc;

#[derive(Clone)]
enum Source {
    File(Arc<File>),
    Memory(Arc<[u8]>),
}

#[derive(Clone)]
pub(super) struct SharedFile {
    source: Source,
    len: u64,
    pos: u64,
}

impl SharedFile {
    pub fn new(file: File) -> io::Result<Self> {
        let len = file.metadata()?.len();
        Ok(Self {
            source: Source::File(Arc::new(file)),
            len,
            pos: 0,
        })
    }

    /// Contenu déjà en mémoire, partagé sans copie entre les clones.
    pub fn memory(bytes: Arc<[u8]>) -> Self {
        Self {
            len: bytes.len() as u64,
            source: Source::Memory(bytes),
            pos: 0,
        }
    }
}

impl Read for SharedFile {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = match &self.source {
            Source::File(file) => read_at(file, buf, self.pos)?,
            Source::Memory(bytes) => {
                let start = usize::try_from(self.pos).map_or(bytes.len(), |p| p.min(bytes.len()));
                let n = buf.len().min(bytes.len() - start);
                buf[..n].copy_from_slice(&bytes[start..start + n]);
                n
            }
        };
        self.pos += n as u64;
        Ok(n)
    }
}

impl Seek for SharedFile {
    fn seek(&mut self, from: SeekFrom) -> io::Result<u64> {
        let pos = match from {
            SeekFrom::Start(p) => Some(p),
            SeekFrom::End(d) => self.len.checked_add_signed(d),
            SeekFrom::Current(d) => self.pos.checked_add_signed(d),
        };
        self.pos = pos.ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "invalid seek"))?;
        Ok(self.pos)
    }
}

#[cfg(unix)]
fn read_at(file: &File, buf: &mut [u8], offset: u64) -> io::Result<usize> {
    std::os::unix::fs::FileExt::read_at(file, buf, offset)
}

#[cfg(windows)]
fn read_at(file: &File, buf: &mut [u8], offset: u64) -> io::Result<usize> {
    std::os::windows::fs::FileExt::seek_read(file, buf, offset)
}
