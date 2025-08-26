use askama::Template;

use crate::models::{WebArtist, WebEvent, WebVenue};

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    pub events: Vec<WebEvent>,
}

#[derive(Template)]
#[template(path = "events_list.html")]
pub struct EventsListTemplate {
    pub events: Vec<WebEvent>,
}

#[derive(Template)]
#[template(path = "artist.html")]
pub struct ArtistTemplate {
    pub artist: WebArtist,
    pub events: Vec<WebEvent>,
}

#[derive(Template)]
#[template(path = "venue.html")]
pub struct VenueTemplate {
    pub venue: WebVenue,
    pub events: Vec<WebEvent>,
}
