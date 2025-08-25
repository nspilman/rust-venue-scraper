use sms_scraper::pipeline::storage::{DatabaseStorage, Storage};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    
    println!("🔍 Checking Artist-Event Links");
    println!("{}", "=".repeat(60));
    
    let storage = DatabaseStorage::new().await?;
    
    // Get all events
    let events = storage.get_all_events(Some(10), None).await?;
    println!("\n📊 Total events in database: {}", events.len());
    
    if events.is_empty() {
        println!("❌ No events found in database!");
        return Ok(());
    }
    
    // Check each event
    println!("\n📋 Event Artist Links:");
    println!("{}", "-".repeat(40));
    
    for event in events {
        print!("   {} ", event.title);
        
        if event.artist_ids.is_empty() {
            println!("❌ No artist links");
        } else {
            println!("✅ {} artist(s) linked", event.artist_ids.len());
            
            // Try to resolve the artists
            for artist_id in &event.artist_ids {
                if let Ok(Some(artist)) = storage.get_artist_by_id(*artist_id).await {
                    println!("      - {}", artist.name);
                }
            }
        }
    }
    
    // Get all artists
    let artists = storage.get_all_artists(Some(10), None).await?;
    println!("\n📊 Total artists in database: {}", artists.len());
    
    Ok(())
}
