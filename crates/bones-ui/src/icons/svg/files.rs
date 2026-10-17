//! Fichiers : galerie, import, export, enregistrement, dossiers.
//!
//! Fragments SVG (viewBox 32 × 32) ; voir `icons::hue` pour les dégradés.

pub const GALLERY: &str = r##"<rect x="4.5" y="4.5" width="10" height="10" rx="2.8" fill="url(#a)"/><rect x="17.5" y="4.5" width="10" height="10" rx="2.8" fill="url(#b)"/><rect x="4.5" y="17.5" width="10" height="10" rx="2.8" fill="url(#b)"/><rect x="17.5" y="17.5" width="10" height="10" rx="2.8" fill="url(#a)"/><path d="M7.4 7.2H11.6M20.4 20.2H24.6" stroke="#fff" stroke-opacity="0.55" stroke-linecap="round" fill="none" stroke-width="1.2"/>"##;

pub const EXPORT: &str = r##"<path d="M5.4 18.4V23.6A3 3 0 0 0 8.4 26.6H23.6A3 3 0 0 0 26.6 23.6V18.4" stroke="url(#a)" stroke-width="2.8" stroke-linecap="round" fill="none"/><path d="M16 19.8V6.4" stroke="url(#b)" stroke-width="2.9" stroke-linecap="round"/><path d="M10.6 11.2 16 5.4 21.4 11.2" stroke="url(#b)" stroke-width="2.9" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

pub const IMPORT: &str = r##"<path d="M5.4 18.4V23.6A3 3 0 0 0 8.4 26.6H23.6A3 3 0 0 0 26.6 23.6V18.4" stroke="url(#a)" stroke-width="2.8" stroke-linecap="round" fill="none"/><path d="M16 4.6V18.2" stroke="url(#b)" stroke-width="2.9" stroke-linecap="round"/><path d="M10.6 13.2 16 19 21.4 13.2" stroke="url(#b)" stroke-width="2.9" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

pub const SAVE: &str = r##"<path d="M5 8A3 3 0 0 1 8 5H21.6L27 10.4V24A3 3 0 0 1 24 27H8A3 3 0 0 1 5 24Z" fill="url(#a)"/><rect x="10" y="5" width="10" height="6.6" rx="1" fill="#fff" fill-opacity=".85"/><rect x="16.2" y="6.2" width="2.2" height="4.2" rx=".6" fill="url(#a)"/><rect x="9" y="16.4" width="14" height="8.4" rx="1.6" fill="#fff" fill-opacity=".92"/><path d="M11.6 19.4H20.4M11.6 22H17.6" stroke="url(#a)" stroke-opacity=".7" stroke-width="1.3" stroke-linecap="round"/>"##;

pub const FOLDER: &str = r##"<path d="M4 9.6A3 3 0 0 1 7 6.6H12.4L15 9.2H25A3 3 0 0 1 28 12.2V23.4A3 3 0 0 1 25 26.4H7A3 3 0 0 1 4 23.4Z" fill="url(#a)"/><path d="M4 13.6H28V23.4A3 3 0 0 1 25 26.4H7A3 3 0 0 1 4 23.4Z" fill="url(#b)" fill-opacity=".6"/><path d="M7.4 16.4H24.6" stroke="#fff" stroke-opacity="0.55" stroke-linecap="round" fill="none" stroke-width="1.2"/>"##;
