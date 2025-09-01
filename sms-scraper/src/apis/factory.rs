use crate::apis::base::{BaseCrawler, VenueParser};
use crate::apis::parsers::*;
use crate::common::constants::*;
use crate::registry::source_loader::SourceRegistry;
use sms_core::common::types::EventApi;
use sms_core::common::error::Result;

/// Factory function to create crawlers using the abstracted architecture
pub fn create_crawler(api_name: &str, source_registry: SourceRegistry) -> Result<Option<Box<dyn EventApi>>> {
    let crawler = match api_name {
        BLUE_MOON_API => Some(Box::new(BaseCrawler::new(
            BLUE_MOON_API,
            Box::new(BlueMoonParser::new()),
            source_registry.clone(),
        )) as Box<dyn EventApi>),
        SEA_MONSTER_API => Some(Box::new(BaseCrawler::new(
            SEA_MONSTER_API,
            Box::new(SeaMonsterParser::new()),
            source_registry.clone(),
        )) as Box<dyn EventApi>),
        DARRELLS_TAVERN_API => Some(Box::new(BaseCrawler::new(
            DARRELLS_TAVERN_API,
            Box::new(DarrellsTavernParser::new()),
            source_registry.clone(),
        )) as Box<dyn EventApi>),
        KEXP_API => Some(Box::new(BaseCrawler::new(
            KEXP_API,
            Box::new(KexpParser::new()),
            source_registry.clone(),
        )) as Box<dyn EventApi>),
        BARBOZA_API => Some(Box::new(BaseCrawler::new(
            BARBOZA_API,
            Box::new(BarbozaParser::new()),
            source_registry.clone(),
        )) as Box<dyn EventApi>),
        NEUMOS_API => Some(Box::new(BaseCrawler::new(
            NEUMOS_API,
            Box::new(NeumosParser::new()),
            source_registry.clone(),
        )) as Box<dyn EventApi>),
        CONOR_BYRNE_API => Some(Box::new(BaseCrawler::new(
            CONOR_BYRNE_API,
            Box::new(ConorByrneParser::new()),
            source_registry.clone(),
        )) as Box<dyn EventApi>),
        _ => None,
    };
    
    Ok(crawler)
}

/// Factory function to create parsers directly
pub fn create_parser(api_name: &str) -> Option<Box<dyn VenueParser>> {
    match api_name {
        BLUE_MOON_API => Some(Box::new(BlueMoonParser::new())),
        SEA_MONSTER_API => Some(Box::new(SeaMonsterParser::new())),
        DARRELLS_TAVERN_API => Some(Box::new(DarrellsTavernParser::new())),
        KEXP_API => Some(Box::new(KexpParser::new())),
        BARBOZA_API => Some(Box::new(BarbozaParser::new())),
        NEUMOS_API => Some(Box::new(NeumosParser::new())),
        CONOR_BYRNE_API => Some(Box::new(ConorByrneParser::new())),
        _ => None,
    }
}