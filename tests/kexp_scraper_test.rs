#[cfg(test)]
mod tests {
    use sms_scraper::apis::kexp::KexpCrawler;
    use sms_scraper::common::types::EventApi;
    use serde_json::json;

    #[test]
    fn test_kexp_api_name() {
        let crawler = KexpCrawler::new();
        assert_eq!(crawler.api_name(), "kexp");
    }

    #[test]
    fn test_kexp_get_raw_data_info() {
        let crawler = KexpCrawler::new();
        
        let raw_data = json!({
            "id": "pigs-pigs-pigs-live-on-kexp",
            "title": "Pigs Pigs Pigs Pigs Pigs Pigs Pigs LIVE on KEXP (OPEN TO THE PUBLIC)",
            "date": "2025-08-20",
            "time": "1 p.m.",
            "start_time": "13:00:00",
            "location": "KEXP Gathering Space",
            "url": "https://www.kexp.org/events/kexp-events/pigs-pigs-pigs-live-on-kexp/"
        });

        let result = crawler.get_raw_data_info(&raw_data).unwrap();
        
        assert_eq!(result.event_api_id, "pigs-pigs-pigs-live-on-kexp");
        assert_eq!(result.event_name, "Pigs Pigs Pigs Pigs Pigs Pigs Pigs LIVE on KEXP (OPEN TO THE PUBLIC)");
        assert_eq!(result.venue_name, "KEXP");
        assert_eq!(result.event_day.to_string(), "2025-08-20");
    }

    #[test]
    fn test_kexp_get_event_args() {
        let crawler = KexpCrawler::new();
        
        let raw_data = json!({
            "id": "car-seat-headrest-live-on-kexp",
            "title": "Car Seat Headrest LIVE on KEXP (OPEN TO THE PUBLIC)",
            "date": "2025-08-22",
            "time": "noon",
            "start_time": "12:00:00",
            "location": "KEXP Studio (NW Rooms)",
            "url": "https://www.kexp.org/events/kexp-events/car-seat-headrest-live/",
            "description": "Want to attend a KEXP live in-studio session? Learn how here."
        });

        let result = crawler.get_event_args(&raw_data).unwrap();
        
        assert_eq!(result.title, "Car Seat Headrest LIVE on KEXP (OPEN TO THE PUBLIC)");
        assert_eq!(result.event_day.to_string(), "2025-08-22");
        assert_eq!(result.start_time.unwrap().format("%H:%M:%S").to_string(), "12:00:00");
        assert_eq!(result.event_url.unwrap(), "https://www.kexp.org/events/kexp-events/car-seat-headrest-live/");
        assert_eq!(result.description.unwrap(), "Want to attend a KEXP live in-studio session? Learn how here.");
    }

    #[test]
    fn test_kexp_should_skip_broadcast_only() {
        let crawler = KexpCrawler::new();
        
        // Test that we skip broadcast-only events (not open to public)
        let broadcast_only = json!({
            "id": "indigo-de-souza-live-on-kexp",
            "title": "Indigo De Souza LIVE on KEXP",
            "date": "2025-08-31"
        });

        let (should_skip, reason) = crawler.should_skip(&broadcast_only);
        assert!(should_skip);
        assert!(reason.contains("broadcast-only"));
    }

    #[test] 
    fn test_kexp_should_not_skip_public_events() {
        let crawler = KexpCrawler::new();
        
        // Test that we don't skip public events
        let public_event = json!({
            "id": "king-stingray-live-on-kexp",
            "title": "King Stingray LIVE on KEXP (OPEN TO THE PUBLIC)",
            "date": "2025-08-28"
        });

        let (should_skip, _reason) = crawler.should_skip(&public_event);
        assert!(!should_skip);
    }

    #[test]
    fn test_kexp_should_skip_book_reading() {
        let crawler = KexpCrawler::new();
        
        let book_reading = json!({
            "id": "book-reading-event",
            "title": "Author Book Reading at KEXP",
            "date": "2025-09-01"
        });

        let (should_skip, reason) = crawler.should_skip(&book_reading);
        assert!(should_skip);
        assert!(reason.contains("book reading"));
    }
}
