//! Animation et physique : lecture, clés, poids, balistique, centre de masse.
//!
//! Fragments SVG (viewBox 32 × 32) ; voir `icons::hue` pour les dégradés.

pub const PLAY: &str = r##"<path d="M10 7.2C10 5.5 11.8 4.5 13.3 5.4L26.7 13.6C28.1 14.5 28.1 16.5 26.7 17.4L13.3 25.6C11.8 26.5 10 25.5 10 23.8Z" fill="url(#a)"/><path d="M12.6 8.6V14" stroke="#fff" stroke-opacity="0.55" stroke-linecap="round" fill="none" stroke-width="1.4"/>"##;

pub const PAUSE: &str = r##"<rect x="7.8" y="5.6" width="6.2" height="20.8" rx="2.4" fill="url(#a)"/><rect x="18" y="5.6" width="6.2" height="20.8" rx="2.4" fill="url(#a)"/><path d="M10.4 8.4V13.4M20.6 8.4V13.4" stroke="#fff" stroke-opacity="0.55" stroke-linecap="round" fill="none" stroke-width="1.3"/>"##;

pub const PREV_KEY: &str = r##"<g transform="matrix(-1 0 0 1 32 0)"><path d="M6 8.3C6 7 7.4 6.3 8.4 7L18.8 14.5C19.7 15.2 19.7 16.6 18.8 17.3L8.4 24.8C7.4 25.5 6 24.8 6 23.5Z" fill="url(#a)"/><rect x="21.6" y="6.6" width="4" height="18.8" rx="2" fill="url(#a)"/><path d="M8.2 10.2V14.6" stroke="#fff" stroke-opacity="0.55" stroke-linecap="round" fill="none" stroke-width="1.2"/></g>"##;

pub const NEXT_KEY: &str = r##"<path d="M6 8.3C6 7 7.4 6.3 8.4 7L18.8 14.5C19.7 15.2 19.7 16.6 18.8 17.3L8.4 24.8C7.4 25.5 6 24.8 6 23.5Z" fill="url(#a)"/><rect x="21.6" y="6.6" width="4" height="18.8" rx="2" fill="url(#a)"/><path d="M8.2 10.2V14.6" stroke="#fff" stroke-opacity="0.55" stroke-linecap="round" fill="none" stroke-width="1.2"/>"##;

pub const KEY: &str = r##"<path d="M16 4.2 27.8 16 16 27.8 4.2 16Z" fill="url(#a)" stroke="url(#a)" stroke-width="2.4" stroke-linejoin="round"/><path d="M16 8.6 23.4 16H8.6Z" fill="#fff" fill-opacity=".28"/><circle cx="16" cy="16" r="2.2" fill="#fff" fill-opacity=".9"/>"##;

pub const KEY_REMOVE: &str = r##"<path d="M16 4.6 27.4 16 16 27.4 4.6 16Z" fill="url(#a)" fill-opacity=".2" stroke="url(#a)" stroke-width="2.4" stroke-linejoin="round"/><rect x="9.6" y="14.4" width="12.8" height="3.2" rx="1.6" fill="#FF5E57"/>"##;

pub const WEIGHTS: &str = r##"<defs><linearGradient id="w" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#FF453A"/><stop offset=".34" stop-color="#FFD60A"/><stop offset=".66" stop-color="#32D74B"/><stop offset="1" stop-color="#0A84FF"/></linearGradient></defs><path d="M16 3.4C16 3.4 6.6 13.4 6.6 19.6a9.4 9.4 0 0 0 18.8 0C25.4 13.4 16 3.4 16 3.4Z" fill="url(#w)"/><path d="M10.6 18.6a5.6 5.6 0 0 1 2.2-4.6" stroke="#fff" stroke-opacity="0.55" stroke-linecap="round" fill="none" stroke-width="1.6"/>"##;

pub const BALLISTIC: &str = r##"<path d="M3.6 28H28.4" stroke="#fff" stroke-opacity=".38" stroke-width="1.5" stroke-linecap="round"/><path d="M4.8 25.6C7.6 7.4 18.6 4.6 23.4 19.8" fill="none" stroke="url(#a)" stroke-width="2.5" stroke-linecap="round" stroke-dasharray=".01 3.7"/><circle cx="24.6" cy="23.6" r="3.9" fill="url(#b)"/><circle cx="23.4" cy="22.4" r="1.2" fill="#fff"/>"##;

pub const CENTER_OF_MASS: &str = r##"<circle cx="16" cy="16" r="10.6" fill="#fff" fill-opacity=".92"/><path d="M16 16V5.4A10.6 10.6 0 0 0 5.4 16Z" fill="url(#a)"/><path d="M16 16V26.6A10.6 10.6 0 0 0 26.6 16Z" fill="url(#a)"/><circle cx="16" cy="16" r="10.6" fill="none" stroke="url(#a)" stroke-width="1.8"/><path d="M16 2.4V4.4M16 27.6V29.6M2.4 16H4.4M27.6 16H29.6" stroke="url(#a)" stroke-width="2" stroke-linecap="round"/>"##;
