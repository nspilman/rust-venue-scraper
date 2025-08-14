use crate::domain::Event as DomainEvent;
use crate::graphql::schema::GraphQLContext;
use async_graphql::{Context, FieldResult, Object, ID};

/// GraphQL representation of an Event
#[derive(Clone)]
pub struct Event {
    pub inner: DomainEvent,
}

impl From<DomainEvent> for Event {
fn from(event: DomainEvent) -> Self {
        Self { inner: event }
    }
}

#[Object]
impl Event {
    /// The unique identifier for the event
    async fn id(&self) -> ID {
        ID(self.inner.id.unwrap_or_default().to_string())
    }

    /// The title of the event
    async fn title(&self) -> &str {
        &self.inner.title
    }

    /// The date when the event takes place
    async fn event_day(&self) -> chrono::NaiveDate {
        self.inner.event_day
    }

    /// The start time of the event
    async fn start_time(&self) -> Option<chrono::NaiveTime> {
        self.inner.start_time
    }

    /// URL to the event page
    async fn event_url(&self) -> Option<&str> {
        self.inner.event_url.as_deref()
    }

    /// Description of the event
    async fn description(&self) -> Option<&str> {
        self.inner.description.as_deref()
    }

    /// URL to the event's image
    async fn event_image_url(&self) -> Option<&str> {
        self.inner.event_image_url.as_deref()
    }

    /// Whether the event should be shown publicly
    async fn show_event(&self) -> bool {
        self.inner.show_event
    }

    /// Whether the event details are finalized
    async fn finalized(&self) -> bool {
        self.inner.finalized
    }

    /// When the event was created
    async fn created_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.inner.created_at
    }

    /// The venue where this event takes place
    async fn venue(&self, ctx: &Context<'_>) -> FieldResult<Option<super::venue::Venue>> {
        let context = ctx.data::<GraphQLContext>()?;

        // Get venue by ID from storage
        match context.storage.get_venue_by_id(self.inner.venue_id).await {
            Ok(Some(venue)) => Ok(Some(venue.into())),
            Ok(None) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Artists performing at this event
    async fn artists(&self, ctx: &Context<'_>) -> FieldResult<Vec<super::artist::Artist>> {
        let context = ctx.data::<GraphQLContext>()?;
        let mut artists = Vec::new();

        for artist_id in &self.inner.artist_ids {
            match context.storage.get_artist_by_id(*artist_id).await {
                Ok(Some(artist)) => artists.push(artist.into()),
                Ok(None) => continue, // Skip missing artists
                Err(e) => return Err(e.into()),
            }
        }

        Ok(artists)
    }
}
