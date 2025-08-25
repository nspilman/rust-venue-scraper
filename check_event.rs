use sms_scraper::pipeline::storage::{DatabaseStorage, Storage};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    
    let storage = DatabaseStorage::new().await?;
    
    // Check the PLUGGED IN event
    let event_id = Uuid::parse_str("142f515d-591e-4064-9c16-374d88e1226c")?;
    
    if let Some(event) = storage.get_event_by_id(event_id).await? {
        println!("Event: {}", event.title);
        println!("Artist IDs: {:?}", event.artist_ids);
        println!("Number of artist IDs: {}", event.artist_ids.len());
    } else {
        println!("Event not found");
    }
    
    Ok(())
}
