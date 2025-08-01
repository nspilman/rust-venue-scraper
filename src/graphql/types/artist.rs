use crate::carpenter::Artist as CarpenterArtist;
use crate::graphql::schema::GraphQLContext;
use async_graphql::{Context, FieldResult, Object, ID};

/// GraphQL representation of an Artist
#[derive(Clone)]
pub struct Artist {
    pub inner: CarpenterArtist,
}

impl From<CarpenterArtist> for Artist {
    fn from(artist: CarpenterArtist) -> Self {
        Self { inner: artist }
    }
}

#[Object]
impl Artist {
    /// The unique identifier for the artist
    async fn id(&self) -> ID {
        ID(self.inner.id.unwrap_or_default().to_string())
    }

    /// The name of the artist
    async fn name(&self) -> &str {
        &self.inner.name
    }

    /// The artist's name as a URL-friendly slug
    async fn name_slug(&self) -> &str {
        &self.inner.name_slug
    }

    /// Biography or description of the artist
    async fn bio(&self) -> Option<&str> {
        self.inner.bio.as_deref()
    }

    /// URL to the artist's image
    async fn artist_image_url(&self) -> Option<&str> {
        self.inner.artist_image_url.as_deref()
    }

    /// When the artist was created
    async fn created_at(&self) -> chrono::DateTime<chrono::Utc> {
        self.inner.created_at
    }

    /// Events where this artist is performing
    async fn events(&self, ctx: &Context<'_>) -> FieldResult<Vec<super::event::Event>> {
        let context = ctx.data::<GraphQLContext>()?;
        let artist_id = self.inner.id.ok_or("Artist ID not available")?;

        match context.storage.get_events_by_artist_id(artist_id).await {
            Ok(events) => Ok(events.into_iter().map(|e| e.into()).collect()),
            Err(e) => Err(e.into()),
        }
    }
}
