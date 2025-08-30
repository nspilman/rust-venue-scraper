// New abstracted architecture
pub mod base;
pub mod parsers;
pub mod factory;

// Legacy crawlers (keeping for reference during migration)
#[cfg(feature = "scraping")]
pub mod barboza;
pub mod blue_moon;
#[cfg(feature = "scraping")]
pub mod conor_byrne;
#[cfg(feature = "scraping")]
pub mod darrells_tavern;
#[cfg(feature = "scraping")]
pub mod kexp;
#[cfg(feature = "scraping")]
pub mod neumos;
#[cfg(feature = "scraping")]
pub mod sea_monster;
#[cfg(feature = "scraping")]
pub mod sunset_tavern;
