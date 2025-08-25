use sms_scraper::pipeline::storage::{DatabaseStorage, Storage};
use sms_scraper::db::DatabaseManager;
use uuid::Uuid;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::init();
    
    println!("🔍 Diagnosing Artist-Event Linking Issues");
    println!("{}", "=".repeat(60));
    
    // Connect to database
    let storage = DatabaseStorage::new().await?;
    let db_manager = Arc::new(DatabaseManager::new().await?);
    
    // Test event ID (PLUGGED IN event)
    let event_id = Uuid::parse_str("142f515d-591e-4064-9c16-374d88e1226c")?;
    
    println!("\n📊 Step 1: Check Event Data");
    println!("{}", "-".repeat(40));
    
    // Get event from storage
    if let Some(event) = storage.get_event_by_id(event_id).await? {
        println!("✅ Event found: {}", event.title);
        println!("   Event ID: {}", event_id);
        println!("   Artist IDs in event: {} IDs", event.artist_ids.len());
        
        if !event.artist_ids.is_empty() {
            println!("   Artist IDs stored in event:");
            for aid in &event.artist_ids {
                println!("     - {}", aid);
            }
        }
        
        // Check if artists exist
        println!("\n📊 Step 2: Verify Artists Exist");
        println!("{}", "-".repeat(40));
        
        let mut found_artists = 0;
        let mut missing_artists = 0;
        
        for artist_id in &event.artist_ids {
            if let Some(artist) = storage.get_artist_by_id(*artist_id).await? {
                println!("   ✅ Artist {} exists: {}", artist_id, artist.name);
                found_artists += 1;
            } else {
                println!("   ❌ Artist {} NOT FOUND", artist_id);
                missing_artists += 1;
            }
        }
        
        println!("\n   Summary: {} found, {} missing", found_artists, missing_artists);
        
        // Check edges in database
        println!("\n📊 Step 3: Check Database Edges");
        println!("{}", "-".repeat(40));
        
        // Get all edges for this event
        let edges = db_manager.get_edges_for_node(&event_id.to_string()).await?;
        
        println!("   Total edges for event: {}", edges.len());
        
        let mut performs_at_edges = 0;
        let mut hosts_edges = 0;
        
        for (_edge_id, source_id, target_id, relation, _data) in &edges {
            match relation.as_str() {
                "performs_at" => {
                    performs_at_edges += 1;
                    // For performs_at, artist is source, event is target
                    if target_id == &event_id.to_string() {
                        println!("   ✅ performs_at edge: artist {} -> event", source_id);
                        
                        // Check if this artist ID is in our event's artist_ids
                        if let Ok(artist_uuid) = Uuid::parse_str(source_id) {
                            if event.artist_ids.contains(&artist_uuid) {
                                println!("      ✓ Artist IS in event.artist_ids");
                            } else {
                                println!("      ✗ Artist NOT in event.artist_ids");
                            }
                        }
                    }
                },
                "hosts" => {
                    hosts_edges += 1;
                    println!("   ℹ️ hosts edge: venue {} -> event", source_id);
                },
                other => {
                    println!("   ? Unknown edge type: {}", other);
                }
            }
        }
        
        println!("\n   Edge Summary:");
        println!("   - performs_at edges: {}", performs_at_edges);
        println!("   - hosts edges: {}", hosts_edges);
        
        // Check raw node data
        println!("\n📊 Step 4: Check Raw Event Node Data");
        println!("{}", "-".repeat(40));
        
        if let Some((id, label, data)) = db_manager.get_node(&event_id.to_string()).await? {
            println!("   Node ID: {}", id);
            println!("   Label: {}", label);
            
            // Parse JSON to check artist_ids field
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&data) {
                if let Some(artist_ids) = json.get("artist_ids") {
                    println!("   artist_ids field in JSON: {}", artist_ids);
                    
                    if let Some(arr) = artist_ids.as_array() {
                        println!("   Number of artist IDs in JSON: {}", arr.len());
                    }
                } else {
                    println!("   ❌ No artist_ids field in JSON!");
                }
            }
        }
        
        // Test GraphQL resolver logic directly
        println!("\n📊 Step 5: Test GraphQL Resolver Logic");
        println!("{}", "-".repeat(40));
        
        println!("   The GraphQL resolver does:");
        println!("   1. Gets event.artist_ids array");
        println!("   2. For each ID, calls storage.get_artist_by_id()");
        println!("   3. Returns found artists");
        
        println!("\n   Simulating GraphQL resolver:");
        let mut resolved_artists = Vec::new();
        for artist_id in &event.artist_ids {
            match storage.get_artist_by_id(*artist_id).await {
                Ok(Some(artist)) => {
                    println!("   ✅ Resolved artist: {}", artist.name);
                    resolved_artists.push(artist);
                },
                Ok(None) => {
                    println!("   ⚠️ Artist {} not found", artist_id);
                },
                Err(e) => {
                    println!("   ❌ Error fetching artist {}: {}", artist_id, e);
                }
            }
        }
        
        println!("\n   GraphQL would return {} artists", resolved_artists.len());
        
    } else {
        println!("❌ Event not found!");
    }
    
    // Additional check: Get a known artist and check its events
    println!("\n📊 Step 6: Reverse Check - Artist to Events");
    println!("{}", "-".repeat(40));
    
    let artist_id = Uuid::parse_str("cba1a33b-c1b6-563b-afee-4e77da954e23")?; // PLUGGED IN artist
    
    if let Some(artist) = storage.get_artist_by_id(artist_id).await? {
        println!("   Artist: {}", artist.name);
        
        // Get events for this artist
        let events = storage.get_events_by_artist_id(artist_id).await?;
        println!("   Events this artist performs at: {}", events.len());
        
        for event in events {
            println!("     - {} on {}", event.title, event.event_day);
        }
    }
    
    println!("\n🎯 Diagnosis Complete!");
    println!("{}", "=".repeat(60));
    
    Ok(())
}
