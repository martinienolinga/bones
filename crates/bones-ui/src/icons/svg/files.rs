//! Fichiers : galerie, import, export, enregistrement, dossiers.
//!
//! Glyphes SVG monochromes (viewBox 32 × 32), dessinés en blanc.

pub const GALLERY: &str = r##"<rect x="5" y="5" width="9.4" height="9.4" rx="2.6" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><rect x="17.6" y="5" width="9.4" height="9.4" rx="2.6" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><rect x="5" y="17.6" width="9.4" height="9.4" rx="2.6" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><rect x="17.6" y="17.6" width="9.4" height="9.4" rx="2.6" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

pub const EXPORT: &str = r##"<path d="M6 18.4V23.8A2.8 2.8 0 0 0 8.8 26.6H23.2A2.8 2.8 0 0 0 26 23.8V18.4" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M16 19.4V5.6M11 10.6 16 5.6 21 10.6" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

pub const IMPORT: &str = r##"<path d="M6 18.4V23.8A2.8 2.8 0 0 0 8.8 26.6H23.2A2.8 2.8 0 0 0 26 23.8V18.4" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M16 5V18.8M11 13.8 16 18.8 21 13.8" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

pub const SAVE: &str = r##"<path d="M6 8.6A2.6 2.6 0 0 1 8.6 6H21.4L26 10.6V23.4A2.6 2.6 0 0 1 23.4 26H8.6A2.6 2.6 0 0 1 6 23.4Z" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M11 6.4V10.6H19V6.4" stroke="#fff" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M10.4 25.6V19.4A1.2 1.2 0 0 1 11.6 18.2H20.4A1.2 1.2 0 0 1 21.6 19.4V25.6" stroke="#fff" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

pub const FOLDER: &str = r##"<path d="M4.6 10A2.8 2.8 0 0 1 7.4 7.2H12.6L15.2 9.8H24.6A2.8 2.8 0 0 1 27.4 12.6V22.8A2.8 2.8 0 0 1 24.6 25.6H7.4A2.8 2.8 0 0 1 4.6 22.8Z" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M4.6 13.8H27.4" stroke="#fff" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;
