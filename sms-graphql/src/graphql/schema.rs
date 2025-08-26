use crate::graphql::loaders::{ArtistLoader, VenueLoader};
use crate::graphql::resolvers::Query;
use sms_core::storage::Storage;
use async_graphql::dataloader::DataLoader;
use async_graphql::{EmptyMutation, EmptySubscription, Schema};
use std::sync::Arc;

/// GraphQL context containing shared application state
pub struct GraphQLContext {
    pub storage: Arc<dyn Storage>,
    pub venue_loader: DataLoader<VenueLoader>,
    pub artist_loader: DataLoader<ArtistLoader>,
}

/// The complete GraphQL schema
#[allow(dead_code)]
pub type GraphQLSchema = Schema<Query, EmptyMutation, EmptySubscription>;

/// Create a new GraphQL schema with the given storage
#[allow(dead_code)]
pub fn create_schema(storage: Arc<dyn Storage>) -> GraphQLSchema {
    let venue_loader = VenueLoader::new(storage.clone());
    let artist_loader = ArtistLoader::new(storage.clone());
    
    Schema::build(Query, EmptyMutation, EmptySubscription)
        .data(GraphQLContext { 
            storage,
            venue_loader,
            artist_loader,
        })
        .finish()
}
