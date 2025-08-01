/// API name constants to ensure consistency across the codebase
/// These constants define the mapping between user-friendly API names and internal names
// User-friendly API names (used in CLI)
pub const BLUE_MOON_API: &str = "blue_moon";
pub const SEA_MONSTER_API: &str = "sea_monster";
pub const DARRELLS_TAVERN_API: &str = "darrells_tavern";

// Internal API names (used by carpenter implementations)
pub const BLUE_MOON_INTERNAL: &str = "crawler_blue_moon";
pub const SEA_MONSTER_INTERNAL: &str = "crawler_sea_monster_lounge";
pub const DARRELLS_TAVERN_INTERNAL: &str = "crawler_darrells_tavern";

// Venue names (consistent across the application)
pub const BLUE_MOON_VENUE_NAME: &str = "Blue Moon Tavern";
pub const SEA_MONSTER_VENUE_NAME: &str = "Sea Monster Lounge";
pub const DARRELLS_TAVERN_VENUE_NAME: &str = "Darrell's Tavern";

// Note: The sea monster crawler returns "crawler_sea_monster" but we map it to
// "crawler_sea_monster_lounge" for storage consistency

/// Convert user-friendly API name to internal name used by carpenter/storage
pub fn api_name_to_internal(api_name: &str) -> String {
    match api_name {
        BLUE_MOON_API => BLUE_MOON_INTERNAL.to_string(),
        SEA_MONSTER_API => SEA_MONSTER_INTERNAL.to_string(),
        DARRELLS_TAVERN_API => DARRELLS_TAVERN_INTERNAL.to_string(),
        other => other.to_string(),
    }
}

/// Get all supported user-friendly API names
pub fn get_supported_apis() -> Vec<&'static str> {
    vec![BLUE_MOON_API, SEA_MONSTER_API, DARRELLS_TAVERN_API]
}
