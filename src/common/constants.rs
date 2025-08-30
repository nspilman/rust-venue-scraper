/// API name constants to ensure consistency across the codebase
/// These constants define the mapping between user-friendly API names and internal names
// User-friendly API names (used in CLI)
pub const BLUE_MOON_API: &str = "blue_moon";
pub const SEA_MONSTER_API: &str = "sea_monster";
pub const DARRELLS_TAVERN_API: &str = "darrells_tavern";
pub const KEXP_API: &str = "kexp";
pub const BARBOZA_API: &str = "barboza";
pub const NEUMOS_API: &str = "neumos";
pub const CONOR_BYRNE_API: &str = "conor_byrne";
pub const SUNSET_TAVERN_API: &str = "sunset_tavern";

// Internal API names (used by storage implementations)
pub const BLUE_MOON_INTERNAL: &str = "crawler_blue_moon";
pub const SEA_MONSTER_INTERNAL: &str = "crawler_sea_monster_lounge";
pub const DARRELLS_TAVERN_INTERNAL: &str = "crawler_darrells_tavern";
pub const KEXP_INTERNAL: &str = "crawler_kexp";
pub const BARBOZA_INTERNAL: &str = "crawler_barboza";
pub const NEUMOS_INTERNAL: &str = "crawler_neumos";
pub const CONOR_BYRNE_INTERNAL: &str = "crawler_conor_byrne";
pub const SUNSET_TAVERN_INTERNAL: &str = "crawler_sunset_tavern";

// Venue names (consistent across the application)
pub const BLUE_MOON_VENUE_NAME: &str = "Blue Moon Tavern";
pub const SEA_MONSTER_VENUE_NAME: &str = "Sea Monster Lounge";
pub const DARRELLS_TAVERN_VENUE_NAME: &str = "Darrell's Tavern";
pub const KEXP_VENUE_NAME: &str = "KEXP";
pub const BARBOZA_VENUE_NAME: &str = "The Barboza";
pub const NEUMOS_VENUE_NAME: &str = "Neumos";
pub const CONOR_BYRNE_VENUE_NAME: &str = "Conor Byrne Pub";
pub const SUNSET_TAVERN_VENUE_NAME: &str = "Sunset Tavern";

// Note: The sea monster crawler returns "crawler_sea_monster" but we map it to
// "crawler_sea_monster_lounge" for storage consistency

/// Convert user-friendly API name to internal name used by storage/persistence layers
pub fn api_name_to_internal(api_name: &str) -> String {
    match api_name {
        BLUE_MOON_API => BLUE_MOON_INTERNAL.to_string(),
        SEA_MONSTER_API => SEA_MONSTER_INTERNAL.to_string(),
        DARRELLS_TAVERN_API => DARRELLS_TAVERN_INTERNAL.to_string(),
        KEXP_API => KEXP_INTERNAL.to_string(),
        BARBOZA_API => BARBOZA_INTERNAL.to_string(),
        NEUMOS_API => NEUMOS_INTERNAL.to_string(),
        CONOR_BYRNE_API => CONOR_BYRNE_INTERNAL.to_string(),
        SUNSET_TAVERN_API => SUNSET_TAVERN_INTERNAL.to_string(),
        other => other.to_string(),
    }
}

/// Get all supported user-friendly API names
pub fn get_supported_apis() -> Vec<&'static str> {
    vec![BLUE_MOON_API, SEA_MONSTER_API, DARRELLS_TAVERN_API, KEXP_API, BARBOZA_API, NEUMOS_API, CONOR_BYRNE_API, SUNSET_TAVERN_API]
}
