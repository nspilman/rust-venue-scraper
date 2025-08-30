use crate::apis::base::{BaseCrawler, VenueParser};
use crate::apis::parsers::*;
use crate::common::constants::*;
use sms_core::common::types::EventApi;

/// Factory function to create crawlers using the abstracted architecture
pub fn create_crawler(api_name: &str) -> Option<Box<dyn EventApi>> {
    match api_name {
        BLUE_MOON_API => Some(Box::new(BaseCrawler::new(
            BLUE_MOON_API,
            Box::new(BlueMoonParser::new()),
        ))),
        SEA_MONSTER_API => Some(Box::new(BaseCrawler::new(
            SEA_MONSTER_API,
            Box::new(SeaMonsterParser::new()),
        ))),
        DARRELLS_TAVERN_API => Some(Box::new(BaseCrawler::new(
            DARRELLS_TAVERN_API,
            Box::new(DarrellsTavernParser::new()),
        ))),
        KEXP_API => Some(Box::new(BaseCrawler::new(
            KEXP_API,
            Box::new(KexpParser::new()),
        ))),
        BARBOZA_API => Some(Box::new(BaseCrawler::new(
            BARBOZA_API,
            Box::new(BarbozaParser::new()),
        ))),
        NEUMOS_API => Some(Box::new(BaseCrawler::new(
            NEUMOS_API,
            Box::new(NeumosParser::new()),
        ))),
        CONOR_BYRNE_API => Some(Box::new(BaseCrawler::new(
            CONOR_BYRNE_API,
            Box::new(ConorByrneParser::new()),
        ))),
        _ => None,
    }
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