//! Noms de fichiers : nettoyage, numérotation des copies, comparaison.

use std::fs;
use std::path::Path;

/// Longueur maximale d'un nom, en caractères.
const MAX_NAME_CHARS: usize = 80;
/// … et en octets UTF-8 (les noms de fichiers sont limités à 255 octets).
const MAX_NAME_BYTES: usize = 200;

/// Nom utilisable comme nom de fichier : sans séparateurs ni caractères de
/// contrôle, sans point initial (fichier caché), espaces superflus retirés,
/// longueur bornée. Les accents et autres caractères Unicode sont gardés.
pub fn sanitize_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    let mut out = String::new();
    let start = cleaned.trim().trim_start_matches('.').trim_start();
    for (i, c) in start.chars().enumerate() {
        if i >= MAX_NAME_CHARS || out.len() + c.len_utf8() > MAX_NAME_BYTES {
            break;
        }
        out.push(c);
    }
    // Fin de nom : ni espace ni point (refusés par certains systèmes).
    out.trim_end_matches(|c: char| c.is_whitespace() || c == '.')
        .to_owned()
}

/// Nom du fichier sans son extension.
pub(super) fn stem(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// « Nom 2 » → « Nom » (pour numéroter une copie à partir du nom d'origine).
pub(super) fn strip_counter(name: &str) -> &str {
    match name.rsplit_once(' ') {
        Some((base, n))
            if !base.trim().is_empty() && !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) =>
        {
            base
        }
        _ => name,
    }
}

/// Même fichier sur le disque (mêmes volume et inode).
pub(super) fn same_file(a: &Path, b: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        match (fs::metadata(a), fs::metadata(b)) {
            (Ok(a), Ok(b)) => a.dev() == b.dev() && a.ino() == b.ino(),
            _ => false,
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (a, b);
        false
    }
}
