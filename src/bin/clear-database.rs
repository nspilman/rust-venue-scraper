use sms_scraper::db::DatabaseManager;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    
    println!("⚠️  WARNING: This will delete ALL data from the database!");
    println!("Press Enter to continue or Ctrl+C to cancel...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    
    println!("🗑️  Clearing database...");
    let db = DatabaseManager::new().await?;
    db.clear_all_data().await?;
    
    println!("✅ Database cleared successfully!");
    Ok(())
}
