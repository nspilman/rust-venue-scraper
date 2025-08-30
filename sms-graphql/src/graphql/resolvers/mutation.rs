use crate::graphql::schema::GraphQLContext;
use async_graphql::{Context, FieldResult, Object, ID};
use uuid::Uuid;

/// Root mutation object for GraphQL
pub struct Mutation;

#[Object]
impl Mutation {
    /// Delete all events for a specific venue by venue name
    async fn delete_events_by_venue_name(
        &self,
        ctx: &Context<'_>,
        venue_name: String,
    ) -> FieldResult<i32> {
        let context = ctx.data::<GraphQLContext>()?;
        
        // First, get all events
        let all_events = context.storage.get_all_events(None, None).await
            .map_err(|e| async_graphql::Error::new(format!("Failed to get events: {}", e)))?;
        
        let mut deleted_count = 0;
        
        for event in all_events {
            // Get venue for this event to check name
            if let Ok(Some(venue)) = context.storage.get_venue_by_id(event.venue_id).await {
                if venue.name.to_lowercase() == venue_name.to_lowercase() {
                    // Delete this event
                    if let Some(event_id) = event.id {
                        match context.storage.delete_event(event_id).await {
                            Ok(_) => {
                                deleted_count += 1;
                                tracing::info!("Deleted event: {} (ID: {})", event.title, event_id);
                            }
                            Err(e) => {
                                tracing::error!("Failed to delete event {}: {}", event.title, e);
                            }
                        }
                    }
                }
            }
        }
        
        Ok(deleted_count)
    }
    
    /// Delete a specific event by ID
    async fn delete_event(&self, ctx: &Context<'_>, id: ID) -> FieldResult<bool> {
        let context = ctx.data::<GraphQLContext>()?;
        let event_id = Uuid::parse_str(&id)
            .map_err(|e| async_graphql::Error::new(format!("Invalid UUID: {}", e)))?;
        
        match context.storage.delete_event(event_id).await {
            Ok(_) => {
                tracing::info!("Deleted event with ID: {}", event_id);
                Ok(true)
            }
            Err(e) => Err(async_graphql::Error::new(format!("Failed to delete event: {}", e))),
        }
    }
}
