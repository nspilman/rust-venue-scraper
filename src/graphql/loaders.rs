use crate::domain::{Artist, Venue};
use crate::pipeline::storage::Storage;
use async_graphql::dataloader::{DataLoader, Loader};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// DataLoader for batching venue lookups
pub struct VenueLoader {
    storage: Arc<dyn Storage>,
}

impl VenueLoader {
    pub fn new(storage: Arc<dyn Storage>) -> DataLoader<Self> {
        DataLoader::new(Self { storage }, tokio::spawn)
    }
}

#[async_trait]
impl Loader<Uuid> for VenueLoader {
    type Value = Venue;
    type Error = String;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        let venues = self.storage.get_venues_by_ids(keys.to_vec()).await
            .map_err(|e| e.to_string())?;
        
        let mut map = HashMap::new();
        for venue in venues {
            if let Some(id) = venue.id {
                map.insert(id, venue);
            }
        }
        
        Ok(map)
    }
}

/// DataLoader for batching artist lookups
pub struct ArtistLoader {
    storage: Arc<dyn Storage>,
}

impl ArtistLoader {
    pub fn new(storage: Arc<dyn Storage>) -> DataLoader<Self> {
        DataLoader::new(Self { storage }, tokio::spawn)
    }
}

#[async_trait]
impl Loader<Uuid> for ArtistLoader {
    type Value = Artist;
    type Error = String;

    async fn load(&self, keys: &[Uuid]) -> Result<HashMap<Uuid, Self::Value>, Self::Error> {
        let artists = self.storage.get_artists_by_ids(keys.to_vec()).await
            .map_err(|e| e.to_string())?;
        
        let mut map = HashMap::new();
        for artist in artists {
            if let Some(id) = artist.id {
                map.insert(id, artist);
            }
        }
        
        Ok(map)
    }
}