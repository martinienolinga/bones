//! Affichage : œil, grille, maillage, fantômes, trajectoire, cadrage.
//!
//! Fragments SVG (viewBox 32 × 32) ; voir `icons::hue` pour les dégradés.

pub const EYE: &str = r##"<path d="M3 16C7.2 9 11.6 6.6 16 6.6S24.8 9 29 16C24.8 23 20.4 25.4 16 25.4S7.2 23 3 16Z" fill="url(#a)"/><circle cx="16" cy="16" r="6" fill="url(#b)"/><circle cx="16" cy="16" r="2.9" fill="#1d1546"/><circle cx="17.7" cy="14.3" r="1.25" fill="#fff"/>"##;

pub const GHOST: &str = r##"<circle cx="11.4" cy="6.8" r="3.4" fill="url(#a)" fill-opacity="0.4"/><path d="M4.96 18.49A6.44 6.99 0 0 1 17.84 18.49Z" fill="url(#a)" fill-opacity="0.4"/><circle cx="19.6" cy="9.7" r="3.7" fill="url(#a)"/><path d="M12.6 22.4A7 7.6 0 0 1 26.6 22.4Z" fill="url(#a)"/><path d="M17.2 9.4a3 3 0 0 1 2.4-2" stroke="#fff" stroke-opacity="0.55" stroke-linecap="round" fill="none" stroke-width="1.2"/>"##;

pub const TRAJECTORY: &str = r##"<path d="M5.2 26.6C6.4 15 16 23.4 21.2 11" fill="none" stroke="url(#a)" stroke-width="2.6" stroke-linecap="round" stroke-dasharray=".01 3.9"/><circle cx="5.2" cy="26.6" r="2.6" fill="url(#a)"/><path d="M25.8 5.8L24.18 14.69L17.57 9.52Z" fill="url(#a)" stroke="url(#a)" stroke-width="1.6" stroke-linejoin="round"/>"##;

pub const FRAME: &str = r##"<path d="M5.4 11.4V8A2.6 2.6 0 0 1 8 5.4H11.4M20.6 5.4H24A2.6 2.6 0 0 1 26.6 8V11.4M26.6 20.6V24A2.6 2.6 0 0 1 24 26.6H20.6M11.4 26.6H8A2.6 2.6 0 0 1 5.4 24V20.6" stroke="url(#a)" stroke-width="2.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/><circle cx="16" cy="16" r="3.6" fill="url(#b)"/>"##;

pub const GRID: &str = r##"<rect x="4.5" y="4.5" width="23" height="23" rx="4.6" fill="url(#a)" fill-opacity=".2" stroke="url(#a)" stroke-width="2.2"/><path d="M12.2 5.6V26.4M19.8 5.6V26.4M5.6 12.2H26.4M5.6 19.8H26.4" stroke="url(#a)" stroke-width="1.6"/>"##;

pub const MESH: &str = r##"<circle cx="16" cy="16" r="11.4" fill="url(#a)" fill-opacity=".2"/><ellipse cx="16" cy="16" rx="5.4" ry="11.4" fill="none" stroke="url(#a)" stroke-width="1.5"/><ellipse cx="16" cy="16" rx="11.4" ry="4.4" fill="none" stroke="url(#a)" stroke-width="1.5"/><path d="M16 4.6V27.4" fill="none" stroke="url(#a)" stroke-width="1.5"/><circle cx="16" cy="16" r="11.4" fill="none" stroke="url(#a)" stroke-width="2.2"/><path d="M9.4 8.6a9.4 9.4 0 0 1 4.6-2.6" stroke="#fff" stroke-opacity="0.55" stroke-linecap="round" fill="none" stroke-width="1.3"/>"##;

pub const LOCK: &str = r##"<path d="M10.6 14.2V10.8a5.4 5.4 0 0 1 10.8 0V14.2" stroke="url(#a)" stroke-width="2.9" fill="none" stroke-linecap="round"/><rect x="6.8" y="13.4" width="18.4" height="14.4" rx="3.8" fill="url(#a)"/><circle cx="16" cy="19.4" r="2.2" fill="#fff"/><path d="M16 20.4V23.6" stroke="#fff" stroke-width="2.1" stroke-linecap="round"/><path d="M9.8 16.2H22.2" stroke="#fff" stroke-opacity="0.45" stroke-linecap="round" fill="none" stroke-width="1.2"/>"##;

pub const HOME: &str = r##"<path d="M8.4 13.8V25.2A2.2 2.2 0 0 0 10.6 27.4H13.8V21.2A2.2 2.2 0 0 1 16 19 2.2 2.2 0 0 1 18.2 21.2V27.4H21.4A2.2 2.2 0 0 0 23.6 25.2V13.8L16 7.4Z" fill="url(#a)"/><path d="M4.8 15.4 16 5.6 27.2 15.4" stroke="url(#b)" stroke-width="2.9" stroke-linecap="round" stroke-linejoin="round" fill="none"/><rect x="21" y="5.4" width="3.2" height="5.6" rx="1" fill="url(#a)"/>"##;

pub const ORTHO: &str = r##"<path d="M16 4.4 26.6 10.3 16 16.2 5.4 10.3Z" fill="url(#b)"/><path d="M5.4 10.3 16 16.2V28.2L5.4 22.3Z" fill="url(#a)"/><path d="M26.6 10.3V22.3L16 28.2V16.2Z" fill="url(#a)" fill-opacity=".62"/><path d="M5.4 10.3 16 16.2 26.6 10.3M16 16.2V28.2" stroke="#fff" stroke-opacity=".5" stroke-width="1.1" stroke-linejoin="round" fill="none"/>"##;
