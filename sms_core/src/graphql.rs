use async_graphql::*;
use crate::domain::*;
use chrono::{NaiveDate, NaiveTime};

/// GraphQL representation of a Venue
#[Object]
impl Venue {
    async fn id(&self) -> Option<ID> {
        self.id.map(|id| ID(id.to_string()))
    }
    
    async fn name(&self) -> &str {
        &self.name
    }
    
    async fn address(&self) -> &str {
        &self.address
    }
    
    async fn city(&self) -> &str {
        &self.city
    }
    
    async fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
    
    async fn venue_url(&self) -> Option<&str> {
        self.venue_url.as_deref()
    }
    
    async fn venue_image_url(&self) -> Option<&str> {
        self.venue_image_url.as_deref()
    }
    
    async fn latitude(&self) -> f64 {
        self.latitude
    }
    
    async fn longitude(&self) -> f64 {
        self.longitude
    }
    
    async fn neighborhood(&self) -> Option<&str> {
        self.neighborhood.as_deref()
    }
}

/// GraphQL representation of an Artist
#[Object]
impl Artist {
    async fn id(&self) -> Option<ID> {
        self.id.map(|id| ID(id.to_string()))
    }
    
    async fn name(&self) -> &str {
        &self.name
    }
    
    async fn name_slug(&self) -> &str {
        &self.name_slug
    }
    
    async fn bio(&self) -> Option<&str> {
        self.bio.as_deref()
    }
    
    async fn artist_image_url(&self) -> Option<&str> {
        self.artist_image_url.as_deref()
    }
}

/// GraphQL representation of an Event
#[Object]
impl Event {
    async fn id(&self) -> Option<ID> {
        self.id.map(|id| ID(id.to_string()))
    }
    
    async fn title(&self) -> &str {
        &self.title
    }
    
    async fn event_day(&self) -> NaiveDate {
        self.event_day
    }
    
    async fn start_time(&self) -> Option<NaiveTime> {
        self.start_time
    }
    
    async fn event_url(&self) -> Option<&str> {
        self.event_url.as_deref()
    }
    
    async fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
    
    async fn event_image_url(&self) -> Option<&str> {
        self.event_image_url.as_deref()
    }
    
    // Note: venue and artists relationships would be resolved by the parent GraphQL server
    // using the venue_id and artist_ids fields
}

/// Common GraphQL input types
#[derive(InputObject)]
pub struct PaginationInput {
    pub limit: Option<i32>,
    pub offset: Option<i32>,
}

#[derive(InputObject)]
pub struct EventFilter {
    pub search: Option<String>,
    pub venue_id: Option<ID>,
    pub start_date: Option<NaiveDate>,
    pub end_date: Option<NaiveDate>,
}