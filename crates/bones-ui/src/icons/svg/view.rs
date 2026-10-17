//! Affichage : œil, grille, maillage, fantômes, trajectoire, cadrage.
//!
//! Glyphes SVG monochromes (viewBox 32 × 32), dessinés en blanc.

pub const EYE: &str = r##"<path d="M3.5 16C7.4 9.4 11.6 7.2 16 7.2S24.6 9.4 28.5 16C24.6 22.6 20.4 24.8 16 24.8S7.4 22.6 3.5 16Z" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><circle cx="16" cy="16" r="4.4" fill="#fff"/>"##;

pub const GHOST: &str = r##"<circle cx="10.8" cy="7.24" r="3.24" stroke="#fff" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M4.68 18.58A6.12 6.66 0 0 1 16.92 18.58Z" stroke="#fff" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/><circle cx="20" cy="10" r="3.6" fill="#fff"/><path d="M13.2 22.6A6.8 7.4 0 0 1 26.8 22.6Z" fill="#fff"/>"##;

pub const TRAJECTORY: &str = r##"<path d="M5.6 26.2C6.8 15 15.6 22.6 20.6 11.6" stroke="#fff" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round" fill="none" stroke-dasharray=".1 3.9"/><path d="M25.8 5.8L24.32 14.03L18.17 9.23Z" fill="#fff" stroke="#fff" stroke-width="1.4" stroke-linejoin="round"/>"##;

pub const FRAME: &str = r##"<path d="M5.4 11.4V8A2.6 2.6 0 0 1 8 5.4H11.4M20.6 5.4H24A2.6 2.6 0 0 1 26.6 8V11.4M26.6 20.6V24A2.6 2.6 0 0 1 24 26.6H20.6M11.4 26.6H8A2.6 2.6 0 0 1 5.4 24V20.6" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><circle cx="16" cy="16" r="2.6" fill="#fff"/>"##;

pub const GRID: &str = r##"<rect x="4.8" y="4.8" width="22.4" height="22.4" rx="4" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M12.3 5.6V26.4M19.7 5.6V26.4M5.6 12.3H26.4M5.6 19.7H26.4" stroke="#fff" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

pub const MESH: &str = r##"<circle cx="16" cy="16" r="11.2" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><ellipse cx="16" cy="16" rx="5.2" ry="11.2" stroke="#fff" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/><ellipse cx="16" cy="16" rx="11.2" ry="4.2" stroke="#fff" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

pub const LOCK: &str = r##"<path d="M11 14V10.8a5 5 0 0 1 10 0V14" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><mask id="k" maskUnits="userSpaceOnUse" x="0" y="0" width="32" height="32"><rect width="32" height="32" fill="#fff"/><circle cx="16" cy="19.4" r="2" fill="#000"/><path d="M16 20.4V23.2" stroke="#000" stroke-width="2" stroke-linecap="round"/></mask><g mask="url(#k)"><rect x="7.4" y="13.6" width="17.2" height="13.6" rx="3.2" fill="#fff"/></g>"##;

pub const HOME: &str = r##"<path d="M5 15.6 16 6 27 15.6" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M8.6 13.4V25A1.8 1.8 0 0 0 10.4 26.8H21.6A1.8 1.8 0 0 0 23.4 25V13.4" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M13.6 26.6V21.2A1.4 1.4 0 0 1 15 19.8H17A1.4 1.4 0 0 1 18.4 21.2V26.6" stroke="#fff" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

pub const ORTHO: &str = r##"<path d="M16 4.6 26.4 10.4V21.6L16 27.4 5.6 21.6V10.4Z" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M5.6 10.4 16 16.2 26.4 10.4M16 16.2V27.4" stroke="#fff" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

pub const GLOBE: &str = r##"<circle cx="16" cy="16" r="11.4" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><ellipse cx="16" cy="16" rx="5" ry="11.4" stroke="#fff" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M4.8 16H27.2M7 10.4H25M7 21.6H25" stroke="#fff" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

pub const LOCAL: &str = r##"<path d="M11 20.6 25.6 16.4M21.4 14.2 25.6 16.4 22.6 20.2" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M11 20.6 13.8 5.8M10.6 9.4 13.8 5.8 16.4 9.6" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M11 20.6 5.4 26.6" stroke="#fff" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/><circle cx="11" cy="20.6" r="2.4" fill="#fff"/>"##;
