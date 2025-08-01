use crate::graphql::resolvers::Query;
use crate::storage::Storage;
use async_graphql::{EmptyMutation, EmptySubscription, Schema};
use std::sync::Arc;

/// GraphQL context containing shared application state
pub struct GraphQLContext {
    pub storage: Arc<dyn Storage>,
}

/// The complete GraphQL schema
pub type GraphQLSchema = Schema<Query, EmptyMutation, EmptySubscription>;

/// Create a new GraphQL schema with the given storage
pub fn create_schema(storage: Arc<dyn Storage>) -> GraphQLSchema {
    Schema::build(Query, EmptyMutation, EmptySubscription)
        .data(GraphQLContext { storage })
        .finish()
}
