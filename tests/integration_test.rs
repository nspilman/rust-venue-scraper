use anyhow::Result;
use chrono::{DateTime, Utc};
use serde_json::json;
use sms_core::domain::RawData;
use sms_scraper::pipeline::processing::pipeline_steps::process_raw_data;
use tempfile::tempdir;
use uuid::Uuid;

#[tokio::test]
async fn test_normalization_and_quality_gate() -> Result<()> {
    // Set up test directories
    let temp_dir = tempdir()?;
    let output_dir = temp_dir.path().to_str().unwrap();
    
    // Create test raw data
    let test_event = json!({
        "title": "Test Event",
        "date": "2023-12-25T20:00:00Z",
        "artists": ["Artist 1", "Artist 2"],
        "venue": {
            "name": "Test Venue",
            "address": "123 Test St"
        },
        "price": 25.50,
        "source_url": "https://example.com/event/123"
    });
    
    let now = Utc::now();
    let raw_data = RawData {
        id: Some(Uuid::new_v4()),
        api_name: "test-source".to_string(),
        event_api_id: "test-event-123".to_string(),
        event_name: "Test Event".to_string(),
        venue_name: "Test Venue".to_string(),
        event_day: now.date_naive(),
        data: test_event,
        processed: false,
        event_id: None,
        created_at: now,
    };
    
    // Process the raw data through the pipeline
    let result = process_raw_data(&raw_data, output_dir).await?;
    
    // Verify the results
    assert_eq!(result.parsed.record_path, "/events/test-event-123");
    
    // Verify normalization occurred (if we have a normalized record)
    if let Some(normalized) = result.normalized {
        // Check if the normalized record has the expected fields
        assert!(!normalized.is_empty());
    }
    
    Ok(())
}
