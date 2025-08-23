use askama::Template;

use crate::models::{Artist, Event, Venue};

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    pub events: Vec<Event>,
}

#[derive(Template)]
#[template(path = "events_list.html")]
pub struct EventsListTemplate {
    pub events: Vec<Event>,
}

#[derive(Template)]
#[template(path = "artist.html")]
pub struct ArtistTemplate {
    pub artist: Artist,
    pub events: Vec<Event>,
}

#[derive(Template)]
#[template(path = "venue.html")]
pub struct VenueTemplate {
    pub venue: Venue,
    pub events: Vec<Event>,
}
