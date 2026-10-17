//! Commandes générales : annuler, rétablir, actions, réglages, fermer, supprimer.
//!
//! Fragments SVG (viewBox 32 × 32) ; voir `icons::hue` pour les dégradés.

pub const ACTIONS: &str = r##"<path d="M21.4 4.2a6.6 6.6 0 0 0-6.2 8.8L5.4 22.8a2.7 2.7 0 0 0 3.8 3.8l9.8-9.8a6.6 6.6 0 0 0 8.8-6.2c0-.9-.2-1.7-.5-2.5l-3.9 3.9-3.6-1-1-3.6 3.9-3.9c-.8-.3-1.6-.5-2.5-.5Z" fill="url(#a)"/><circle cx="7.3" cy="24.7" r="1.2" fill="#2a2d36"/>"##;

pub const ADJUST: &str = r##"<path d="M5 8H27M5 16H27M5 24H27" stroke="#fff" stroke-opacity=".45" stroke-width="2" stroke-linecap="round"/><circle cx="11" cy="8" r="3.3" fill="#0A84FF" stroke="#fff" stroke-width="1.4"/><circle cx="21" cy="16" r="3.3" fill="#FF9F0A" stroke="#fff" stroke-width="1.4"/><circle cx="14" cy="24" r="3.3" fill="#30D158" stroke="#fff" stroke-width="1.4"/>"##;

pub const DELETE: &str = r##"<rect x="12.6" y="3.6" width="6.8" height="4" rx="1.6" fill="none" stroke="url(#a)" stroke-width="2"/><rect x="5.6" y="7.4" width="20.8" height="3.4" rx="1.7" fill="url(#a)"/><path d="M8 12.4H24L22.6 26a2.6 2.6 0 0 1-2.6 2.3H12A2.6 2.6 0 0 1 9.4 26Z" fill="url(#a)"/><path d="M12.9 15.6V25M16 15.6V25M19.1 15.6V25" stroke="#fff" stroke-opacity=".6" stroke-width="1.5" stroke-linecap="round"/>"##;

pub const UNDO: &str = r##"<path d="M24.8 25V20.6A8 8 0 0 0 16.8 12.6H9.4" stroke="url(#a)" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M4.2 12.6L10.8 8.1L10.8 17.1Z" fill="url(#a)" stroke="url(#a)" stroke-width="1.6" stroke-linejoin="round"/>"##;

pub const REDO: &str = r##"<g transform="matrix(-1 0 0 1 32 0)"><path d="M24.8 25V20.6A8 8 0 0 0 16.8 12.6H9.4" stroke="url(#a)" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M4.2 12.6L10.8 8.1L10.8 17.1Z" fill="url(#a)" stroke="url(#a)" stroke-width="1.6" stroke-linejoin="round"/></g>"##;

pub const SETTINGS: &str = r##"<path d="M16.92 6.65L19.48 4.52L21.66 5.42L21.96 8.73L23.27 10.04L26.58 10.34L27.48 12.52L25.35 15.08L25.35 16.92L27.48 19.48L26.58 21.66L23.27 21.96L21.96 23.27L21.66 26.58L19.48 27.48L16.92 25.35L15.08 25.35L12.52 27.48L10.34 26.58L10.04 23.27L8.73 21.96L5.42 21.66L4.52 19.48L6.65 16.92L6.65 15.08L4.52 12.52L5.42 10.34L8.73 10.04L10.04 8.73L10.34 5.42L12.52 4.52L15.08 6.65ZM20.2 16A4.2 4.2 0 1 0 11.8 16A4.2 4.2 0 1 0 20.2 16Z" fill="url(#a)" fill-rule="evenodd" stroke="url(#a)" stroke-width="1.2" stroke-linejoin="round"/><circle cx="16" cy="16" r="4.2" fill="none" stroke="#fff" stroke-opacity=".35" stroke-width="1.2"/>"##;

pub const CLOSE: &str =
    r##"<path d="M9 9 23 23M23 9 9 23" stroke="url(#a)" stroke-width="3.2" stroke-linecap="round"/>"##;
