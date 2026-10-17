//! Commandes générales : annuler, rétablir, actions, réglages, fermer, supprimer.
//!
//! Glyphes SVG monochromes (viewBox 32 × 32), dessinés en blanc.

pub const ACTIONS: &str = r##"<mask id="k" maskUnits="userSpaceOnUse" x="0" y="0" width="32" height="32"><rect width="32" height="32" fill="#fff"/><circle cx="7.4" cy="24.6" r="1.3" fill="#000"/></mask><g mask="url(#k)"><path d="M21.4 4.2a6.6 6.6 0 0 0-6.2 8.8L5.4 22.8a2.7 2.7 0 0 0 3.8 3.8l9.8-9.8a6.6 6.6 0 0 0 8.8-6.2c0-.9-.2-1.7-.5-2.5l-3.9 3.9-3.6-1-1-3.6 3.9-3.9c-.8-.3-1.6-.5-2.5-.5Z" fill="#fff"/></g>"##;

pub const ADJUST: &str = r##"<path d="M5 9H7.4M14.6 9H27M5 16H17.4M24.6 16H27M5 23H9.4M16.6 23H27" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><circle cx="11" cy="9" r="3.4" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><circle cx="21" cy="16" r="3.4" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><circle cx="13" cy="23" r="3.4" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

pub const DELETE: &str = r##"<path d="M5.4 8.6H26.6" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M12.6 8.4V6.8A2 2 0 0 1 14.6 4.8H17.4A2 2 0 0 1 19.4 6.8V8.4" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M8.2 8.8 9.6 25.2A2.6 2.6 0 0 0 12.2 27.6H19.8A2.6 2.6 0 0 0 22.4 25.2L23.8 8.8" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M13.6 13.2V23M18.4 13.2V23" stroke="#fff" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

pub const UNDO: &str = r##"<path d="M24.8 25V20.6A8 8 0 0 0 16.8 12.6H7.4" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M11.8 7.8 7 12.6 11.8 17.4" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

pub const REDO: &str = r##"<g transform="matrix(-1 0 0 1 32 0)"><path d="M24.8 25V20.6A8 8 0 0 0 16.8 12.6H7.4" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M11.8 7.8 7 12.6 11.8 17.4" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/></g>"##;

pub const SETTINGS: &str = r##"<path d="M16.86 7.24L19.37 4.9L21.47 5.77L21.58 9.2L22.8 10.42L26.23 10.53L27.1 12.63L24.76 15.14L24.76 16.86L27.1 19.37L26.23 21.47L22.8 21.58L21.58 22.8L21.47 26.23L19.37 27.1L16.86 24.76L15.14 24.76L12.63 27.1L10.53 26.23L10.42 22.8L9.2 21.58L5.77 21.47L4.9 19.37L7.24 16.86L7.24 15.14L4.9 12.63L5.77 10.53L9.2 10.42L10.42 9.2L10.53 5.77L12.63 4.9L15.14 7.24ZM19.8 16A3.8 3.8 0 1 0 12.2 16A3.8 3.8 0 1 0 19.8 16Z" fill="#fff" fill-rule="evenodd" stroke="#fff" stroke-width="1.2" stroke-linejoin="round"/>"##;

pub const CLOSE: &str = r##"<path d="M8.5 8.5 23.5 23.5M23.5 8.5 8.5 23.5" stroke="#fff" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

pub const CHECK: &str = r##"<path d="M6.6 16.8 13 23.2 25.4 9.4" stroke="#fff" stroke-width="2.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

pub const PLUS: &str = r##"<path d="M16 6.5V25.5M6.5 16H25.5" stroke="#fff" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

pub const MAGNET: &str = r##"<path d="M6 20V14A10 10 0 0 1 26 14V20H20V14A4 4 0 0 0 12 14V20Z" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M6 22.6H12V27.4H6ZM20 22.6H26V27.4H20Z" fill="#fff" stroke="#fff" stroke-width="1.2" stroke-linejoin="round"/>"##;
