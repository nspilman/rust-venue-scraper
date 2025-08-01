use crate::graphql::schema::GraphQLContext;
use crate::graphql::types::{Artist, Event, Venue};
use async_graphql::{Context, FieldResult, Object, ID};
use chrono::NaiveDate;
use uuid::Uuid;

/// Root query object for GraphQL
pub struct Query;

#[Object]
impl Query {
    /// Get a venue by ID
    async fn venue(&self, ctx: &Context<'_>, id: ID) -> FieldResult<Option<Venue>> {
        let context = ctx.data::<GraphQLContext>()?;
        let venue_id = Uuid::parse_str(&id)?;

        match context.storage.get_venue_by_id(venue_id).await {
            Ok(venue) => Ok(venue.map(|v| v.into())),
            Err(e) => Err(e.into()),
        }
    }

    /// Get all venues with optional pagination
    async fn venues(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> FieldResult<Vec<Venue>> {
        let context = ctx.data::<GraphQLContext>()?;

        let limit = limit.map(|l| l as usize);
        let offset = offset.map(|o| o as usize);

        match context.storage.get_all_venues(limit, offset).await {
            Ok(venues) => Ok(venues.into_iter().map(|v| v.into()).collect()),
            Err(e) => Err(e.into()),
        }
    }

    /// Get venues by city
    async fn venues_by_city(&self, ctx: &Context<'_>, city: String) -> FieldResult<Vec<Venue>> {
        let context = ctx.data::<GraphQLContext>()?;

        // For now, get all venues and filter by city
        // In a production system, you'd want a more efficient query
        match context.storage.get_all_venues(None, None).await {
            Ok(venues) => {
                let filtered: Vec<Venue> = venues
                    .into_iter()
                    .filter(|v| v.city.to_lowercase() == city.to_lowercase())
                    .map(|v| v.into())
                    .collect();
                Ok(filtered)
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Get an artist by ID
    async fn artist(&self, ctx: &Context<'_>, id: ID) -> FieldResult<Option<Artist>> {
        let context = ctx.data::<GraphQLContext>()?;
        let artist_id = Uuid::parse_str(&id)?;

        match context.storage.get_artist_by_id(artist_id).await {
            Ok(artist) => Ok(artist.map(|a| a.into())),
            Err(e) => Err(e.into()),
        }
    }

    /// Get all artists with optional pagination
    async fn artists(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> FieldResult<Vec<Artist>> {
        let context = ctx.data::<GraphQLContext>()?;

        let limit = limit.map(|l| l as usize);
        let offset = offset.map(|o| o as usize);

        match context.storage.get_all_artists(limit, offset).await {
            Ok(artists) => Ok(artists.into_iter().map(|a| a.into()).collect()),
            Err(e) => Err(e.into()),
        }
    }

    /// Search artists by name
    async fn search_artists(&self, ctx: &Context<'_>, query: String) -> FieldResult<Vec<Artist>> {
        let context = ctx.data::<GraphQLContext>()?;

        match context.storage.search_artists(&query).await {
            Ok(artists) => Ok(artists.into_iter().map(|a| a.into()).collect()),
            Err(e) => Err(e.into()),
        }
    }

    /// Get an event by ID
    async fn event(&self, ctx: &Context<'_>, id: ID) -> FieldResult<Option<Event>> {
        let context = ctx.data::<GraphQLContext>()?;
        let event_id = Uuid::parse_str(&id)?;

        match context.storage.get_event_by_id(event_id).await {
            Ok(event) => Ok(event.map(|e| e.into())),
            Err(e) => Err(e.into()),
        }
    }

    /// Get all events with optional pagination
    async fn events(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> FieldResult<Vec<Event>> {
        let context = ctx.data::<GraphQLContext>()?;

        let limit = limit.map(|l| l as usize);
        let offset = offset.map(|o| o as usize);

        match context.storage.get_all_events(limit, offset).await {
            Ok(events) => Ok(events.into_iter().map(|e| e.into()).collect()),
            Err(e) => Err(e.into()),
        }
    }

    /// Get events by venue ID
    async fn events_by_venue(&self, ctx: &Context<'_>, venue_id: ID) -> FieldResult<Vec<Event>> {
        let context = ctx.data::<GraphQLContext>()?;
        let venue_uuid = Uuid::parse_str(&venue_id)?;

        match context.storage.get_events_by_venue_id(venue_uuid).await {
            Ok(events) => Ok(events.into_iter().map(|e| e.into()).collect()),
            Err(e) => Err(e.into()),
        }
    }

    /// Get events in a date range
    async fn events_by_date_range(
        &self,
        ctx: &Context<'_>,
        start_date: NaiveDate,
        end_date: NaiveDate,
    ) -> FieldResult<Vec<Event>> {
        let context = ctx.data::<GraphQLContext>()?;

        match context
            .storage
            .get_events_by_date_range(start_date, end_date)
            .await
        {
            Ok(events) => Ok(events.into_iter().map(|e| e.into()).collect()),
            Err(e) => Err(e.into()),
        }
    }

    /// Get upcoming events (next 30 days by default)
    async fn upcoming_events(
        &self,
        ctx: &Context<'_>,
        days: Option<i32>,
    ) -> FieldResult<Vec<Event>> {
        let context = ctx.data::<GraphQLContext>()?;
        let days = days.unwrap_or(30);

        let start_date = chrono::Utc::now().date_naive();
        let end_date = start_date + chrono::Duration::days(days as i64);

        match context
            .storage
            .get_events_by_date_range(start_date, end_date)
            .await
        {
            Ok(events) => Ok(events.into_iter().map(|e| e.into()).collect()),
            Err(e) => Err(e.into()),
        }
    }
}
