use crate::carpenter::Venue as CarpenterVenue;
use crate::graphql::schema::GraphQLContext;
use async_graphql::{Context, FieldResult, Object, ID};

/// GraphQL representation of a Venue
#[derive(Clone)]
pub struct Venue {
    pub inner: CarpenterVenue,
}

impl From<CarpenterVenue> for Venue {
    fn from(venue: CarpenterVenue) -> Self {
        Self { inner: venue }
    }
}

#[Object]
impl Venue {
    /// The unique identifier for the venue
    async fn id(&self) -> ID {
        ID(self.inner.id.unwrap_or_default().to_string())
    }

    /// The name of the venue
    async fn name(&self) -> &str {
        &self.inner.name
    }

    /// The venue's address
    async fn address(&self) -> &str {
        &self.inner.address
    }

    /// The city where the venue is located
    async fn city(&self) -> &str {
        &self.inner.city
    }

    /// The venue's latitude coordinate
    async fn latitude(&self) -> f64 {
        self.inner.latitude
    }

    /// The venue's longitude coordinate  
    async fn longitude(&self) -> f64 {
        self.inner.longitude
    }

    /// The venue's postal code
    async fn postal_code(&self) -> &str {
        &self.inner.postal_code
    }

    /// The venue's website URL
    async fn venue_url(&self) -> Option<&str> {
        self.inner.venue_url.as_deref()
    }

    /// URL to the venue's image
    async fn venue_image_url(&self) -> Option<&str> {
        self.inner.venue_image_url.as_deref()
    }

    /// Description of the venue
    async fn description(&self) -> Option<&str> {
        self.inner.description.as_deref()
    }

    /// The neighborhood where the venue is located
    async fn neighborhood(&self) -> Option<&str> {
        self.inner.neighborhood.as_deref()
    }

    /// Whether the venue should be shown publicly
    async fn show_venue(&self) -> bool {
        self.inner.show_venue
    }

    /// When the venue was created
    async fn created_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.inner.created_at
    }

    /// Events happening at this venue
    async fn events(&self, ctx: &Context<'_>) -> FieldResult<Vec<super::event::Event>> {
        let context = ctx.data::<GraphQLContext>()?;
        let venue_id = self.inner.id.ok_or("Venue ID not available")?;

        match context.storage.get_events_by_venue_id(venue_id).await {
            Ok(events) => Ok(events.into_iter().map(|e| e.into()).collect()),
            Err(e) => Err(e.into()),
        }
    }
}
