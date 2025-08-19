use crate::domain::{Artist, Event, Venue};

/// Handles idempotency checks for catalog operations
pub struct IdempotencyChecker;

impl IdempotencyChecker {
    /// Check if an event has changes compared to stored version
    pub fn event_has_changes(existing: &Event, updated: &Event) -> bool {
        existing.title != updated.title
            || existing.description != updated.description
            || existing.event_url != updated.event_url
            || existing.event_image_url != updated.event_image_url
            || existing.start_time != updated.start_time
            || existing.artist_ids != updated.artist_ids
    }

    /// Check if a venue has changes compared to stored version
    pub fn venue_has_changes(existing: &Venue, updated: &Venue) -> bool {
        existing.name != updated.name
            || existing.address != updated.address
            || existing.city != updated.city
            || existing.postal_code != updated.postal_code
            || existing.neighborhood != updated.neighborhood
            || existing.latitude != updated.latitude
            || existing.longitude != updated.longitude
            || existing.venue_url != updated.venue_url
            || existing.venue_image_url != updated.venue_image_url
            || existing.description != updated.description
    }

    /// Check if an artist has changes compared to stored version
    pub fn artist_has_changes(existing: &Artist, updated: &Artist) -> bool {
        existing.name != updated.name
            || existing.name_slug != updated.name_slug
            || existing.bio != updated.bio
            || existing.artist_image_url != updated.artist_image_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    #[test]
    fn test_event_has_changes() {
        let venue_id = Uuid::new_v4();
        let now = Utc::now();
        
        let event1 = Event {
            id: Some(Uuid::new_v4()),
            title: "Concert A".to_string(),
            event_day: now.date_naive(),
            start_time: Some(now.time()),
            event_url: Some("http://example.com".to_string()),
            description: Some("Description".to_string()),
            event_image_url: None,
            venue_id,
            artist_ids: vec![],
            show_event: true,
            finalized: false,
            created_at: now,
        };
        
        let mut event2 = event1.clone();
        
        // No changes
        assert!(!IdempotencyChecker::event_has_changes(&event1, &event2));
        
        // Title changed
        event2.title = "Concert B".to_string();
        assert!(IdempotencyChecker::event_has_changes(&event1, &event2));
        
        // Reset and change description
        event2 = event1.clone();
        event2.description = Some("New Description".to_string());
        assert!(IdempotencyChecker::event_has_changes(&event1, &event2));
    }

    #[test]
    fn test_venue_has_changes() {
        let now = Utc::now();
        
        let venue1 = Venue {
            id: Some(Uuid::new_v4()),
            name: "Blue Moon".to_string(),
            name_lower: "blue moon".to_string(),
            slug: "blue-moon".to_string(),
            address: "123 Main St".to_string(),
            city: "Seattle".to_string(),
            postal_code: "98105".to_string(),
            neighborhood: Some("U-District".to_string()),
            latitude: 47.6,
            longitude: -122.3,
            venue_url: None,
            venue_image_url: None,
            description: None,
            show_venue: true,
            created_at: now,
        };
        
        let mut venue2 = venue1.clone();
        
        // No changes
        assert!(!IdempotencyChecker::venue_has_changes(&venue1, &venue2));
        
        // Name changed
        venue2.name = "Blue Moon Tavern".to_string();
        assert!(IdempotencyChecker::venue_has_changes(&venue1, &venue2));
        
        // Reset and change neighborhood
        venue2 = venue1.clone();
        venue2.neighborhood = Some("University District".to_string());
        assert!(IdempotencyChecker::venue_has_changes(&venue1, &venue2));
    }

    #[test]
    fn test_artist_has_changes() {
        let now = Utc::now();
        
        let artist1 = Artist {
            id: Some(Uuid::new_v4()),
            name: "The Beatles".to_string(),
            name_slug: "the-beatles".to_string(),
            bio: Some("Famous band".to_string()),
            artist_image_url: None,
            created_at: now,
        };
        
        let mut artist2 = artist1.clone();
        
        // No changes
        assert!(!IdempotencyChecker::artist_has_changes(&artist1, &artist2));
        
        // Bio changed
        artist2.bio = Some("Very famous band".to_string());
        assert!(IdempotencyChecker::artist_has_changes(&artist1, &artist2));
    }
}
