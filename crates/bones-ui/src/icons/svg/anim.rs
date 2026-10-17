//! Animation et physique : lecture, clés, poids, balistique, centre de masse.
//!
//! Fragments SVG plats (viewBox 32 × 32) ; `@p`, `@s`, `@d` sont les
//! teintes de la famille (voir `icons::hue`).

pub const PLAY: &str = r##"<path d="M10 7.2C10 5.5 11.8 4.5 13.3 5.4L26.7 13.6C28.1 14.5 28.1 16.5 26.7 17.4L13.3 25.6C11.8 26.5 10 25.5 10 23.8Z" fill="@p"/>"##;

pub const PAUSE: &str = r##"<rect x="7.8" y="5.6" width="6.2" height="20.8" rx="2.4" fill="@p"/><rect x="18" y="5.6" width="6.2" height="20.8" rx="2.4" fill="@p"/>"##;

pub const PREV_KEY: &str = r##"<g transform="matrix(-1 0 0 1 32 0)"><path d="M6 8.3C6 7 7.4 6.3 8.4 7L18.8 14.5C19.7 15.2 19.7 16.6 18.8 17.3L8.4 24.8C7.4 25.5 6 24.8 6 23.5Z" fill="@p"/><rect x="21.6" y="6.6" width="4" height="18.8" rx="2" fill="@p"/></g>"##;

pub const NEXT_KEY: &str = r##"<path d="M6 8.3C6 7 7.4 6.3 8.4 7L18.8 14.5C19.7 15.2 19.7 16.6 18.8 17.3L8.4 24.8C7.4 25.5 6 24.8 6 23.5Z" fill="@p"/><rect x="21.6" y="6.6" width="4" height="18.8" rx="2" fill="@p"/>"##;

pub const KEY: &str = r##"<path d="M16 4.2 27.8 16 16 27.8 4.2 16Z" fill="@p" stroke="@p" stroke-width="2.4" stroke-linejoin="round"/><path d="M16 11.4 20.6 16 16 20.6 11.4 16Z" fill="#1C1C1E" stroke="#1C1C1E" stroke-width="1" stroke-linejoin="round"/>"##;

pub const KEY_REMOVE: &str = r##"<path d="M16 4.6 27.4 16 16 27.4 4.6 16Z" stroke="@p" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round" fill="none"/><rect x="8.6" y="13.4" width="14.8" height="5.2" rx="2.6" fill="#1C1C1E"/><rect x="9.6" y="14.4" width="12.8" height="3.2" rx="1.6" fill="#FF453A"/>"##;

pub const WEIGHTS: &str = r##"<clipPath id="w"><path d="M16 3.4C16 3.4 6.6 13.4 6.6 19.6a9.4 9.4 0 0 0 18.8 0C25.4 13.4 16 3.4 16 3.4Z"/></clipPath><g clip-path="url(#w)"><rect x="0" y="0" width="32" height="12.4" fill="#FF453A"/><rect x="0" y="12.4" width="32" height="5.2" fill="#FFD60A"/><rect x="0" y="17.6" width="32" height="5.6" fill="#30D158"/><rect x="0" y="23.2" width="32" height="8.8" fill="#0A84FF"/></g>"##;

pub const BALLISTIC: &str = r##"<path d="M3.6 28H28.4" stroke="@s" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" fill="none"/><path d="M4.8 25.6C7.6 7.4 18.6 4.6 23.4 19.8" stroke="@p" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" fill="none" stroke-dasharray=".01 3.7"/><circle cx="24.6" cy="23.6" r="3.9" fill="@p"/>"##;

pub const CENTER_OF_MASS: &str = r##"<circle cx="16" cy="16" r="10.6" fill="#fff"/><path d="M16 16V5.4A10.6 10.6 0 0 0 5.4 16Z" fill="@p"/><path d="M16 16V26.6A10.6 10.6 0 0 0 26.6 16Z" fill="@p"/><circle cx="16" cy="16" r="10.6" fill="none" stroke="@p" stroke-width="1.8"/><path d="M16 2.4V4.4M16 27.6V29.6M2.4 16H4.4M27.6 16H29.6" stroke="@p" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>"##;
