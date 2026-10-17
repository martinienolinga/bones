//! Animation et physique : lecture, clés, poids, balistique, centre de masse.
//!
//! Glyphes SVG monochromes (viewBox 32 × 32), dessinés en blanc.

pub const PLAY: &str = r##"<path d="M10.6 7.6C10.6 6.1 12.2 5.2 13.5 6L25.4 13.6C26.6 14.4 26.6 16.2 25.4 17L13.5 24.6C12.2 25.4 10.6 24.5 10.6 23Z" fill="#fff" stroke="#fff" stroke-width="1" stroke-linejoin="round"/>"##;

pub const PAUSE: &str = r##"<rect x="8.6" y="6.4" width="5.2" height="19.2" rx="1.8" fill="#fff"/><rect x="18.2" y="6.4" width="5.2" height="19.2" rx="1.8" fill="#fff"/>"##;

pub const PREV_KEY: &str = r##"<g transform="matrix(-1 0 0 1 32 0)"><path d="M6.6 9C6.6 7.7 8 7 9 7.7L18.4 14.6C19.3 15.3 19.3 16.7 18.4 17.4L9 24.3C8 25 6.6 24.3 6.6 23Z" fill="#fff" stroke="#fff" stroke-width="1" stroke-linejoin="round"/><rect x="21.6" y="7.2" width="3.6" height="17.6" rx="1.8" fill="#fff"/></g>"##;

pub const NEXT_KEY: &str = r##"<path d="M6.6 9C6.6 7.7 8 7 9 7.7L18.4 14.6C19.3 15.3 19.3 16.7 18.4 17.4L9 24.3C8 25 6.6 24.3 6.6 23Z" fill="#fff" stroke="#fff" stroke-width="1" stroke-linejoin="round"/><rect x="21.6" y="7.2" width="3.6" height="17.6" rx="1.8" fill="#fff"/>"##;

pub const KEY: &str = r##"<path d="M16 5.4 26.6 16 16 26.6 5.4 16Z" fill="#fff" stroke="#fff" stroke-width="2.4" stroke-linejoin="round"/>"##;

pub const KEY_REMOVE: &str = r##"<path d="M16 5.4 26.6 16 16 26.6 5.4 16Z" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M11.8 16H20.2" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

pub const WEIGHTS: &str = r##"<clipPath id="w"><path d="M16 4.2C16 4.2 7.2 13.6 7.2 19.4a8.8 8.8 0 0 0 17.6 0C24.8 13.6 16 4.2 16 4.2Z"/></clipPath><rect x="0" y="18.6" width="32" height="14" fill="#fff" clip-path="url(#w)"/><path d="M16 4.2C16 4.2 7.2 13.6 7.2 19.4a8.8 8.8 0 0 0 17.6 0C24.8 13.6 16 4.2 16 4.2Z" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;

pub const BALLISTIC: &str = r##"<path d="M4 27.6H28" stroke="#fff" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M5.4 24.4C8 8 18.4 5.6 22.8 18.4" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none" stroke-dasharray=".1 3.7"/><circle cx="24.4" cy="22.8" r="3.4" fill="#fff"/>"##;

pub const CENTER_OF_MASS: &str = r##"<circle cx="16" cy="16" r="10.4" stroke="#fff" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M16 16V5.6A10.4 10.4 0 0 0 5.6 16ZM16 16V26.4A10.4 10.4 0 0 0 26.4 16Z" fill="#fff"/>"##;
