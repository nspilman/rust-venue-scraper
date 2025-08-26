use chrono::{Local, NaiveDate, NaiveTime};
use serde_json::json;
use tracing::info;

use crate::pipeline::processing::parser::{Parser, ParsedRecord};

pub struct VenuePilotGraphQLV1Parser {
    pub source_id: String,
    pub envelope_id: String,
    pub payload_ref: String,
}

impl VenuePilotGraphQLV1Parser {
    pub fn new(source_id: String, envelope_id: String, payload_ref: String) -> Self {
        Self {
            source_id,
            envelope_id,
            payload_ref,
        }
    }
}

impl Parser for VenuePilotGraphQLV1Parser {
    fn parse(&self, bytes: &[u8]) -> anyhow::Result<Vec<ParsedRecord>> {
        let text = String::from_utf8_lossy(bytes);
        let json_response: serde_json::Value = serde_json::from_str(&text)?;

        // Extract events from the GraphQL response
        let events = json_response
            .get("data")
            .and_then(|d| d.get("paginatedEvents"))
            .and_then(|p| p.get("collection"))
            .and_then(|c| c.as_array())
            .ok_or_else(|| anyhow::anyhow!("Invalid GraphQL response structure"))?;

        info!("Found {} events in GraphQL response", events.len());
        let mut parsed_records = Vec::new();

        for event in events {
            let mut record = json!({});

            // Extract basic event info
            if let Some(name) = event.get("name").and_then(|n| n.as_str()) {
                record["title"] = json!(name);
            }

            // Extract date and times
            if let Some(date_str) = event.get("date").and_then(|d| d.as_str()) {
                record["event_day"] = json!(date_str);

                // Parse date for validation
                if let Ok(_date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                    // Parse start time and door time
                    let start_time = event.get("startTime")
                        .and_then(|t| t.as_str())
                        .and_then(|t| NaiveTime::parse_from_str(t, "%H:%M:%S").ok())
                        .or_else(|| event.get("doorTime")
                            .and_then(|t| t.as_str())
                            .and_then(|t| NaiveTime::parse_from_str(t, "%H:%M:%S").ok()));

                    if let Some(time) = start_time {
                        record["start_time"] = json!(time.format("%H:%M:%S").to_string());
                    }
                }
            }

            // Extract age restriction
            if let Some(age) = event.get("minimumAge") {
                record["age_restriction"] = json!(format!("{}+", age));
            }

            // Extract promoter
            if let Some(promoter) = event.get("promoter").and_then(|p| p.as_str()) {
                record["promoter"] = json!(promoter);
            }

            // Extract support acts
            if let Some(support) = event.get("support").and_then(|s| s.as_str()) {
                if !support.is_empty() {
                    record["supporting_acts"] = json!(support);
                }
            }

            // Extract description
            if let Some(desc) = event.get("description").and_then(|d| d.as_str()) {
                record["description"] = json!(desc);
            }

            // Extract ticket URL
            if let Some(url) = event.get("ticketsUrl").and_then(|u| u.as_str()) {
                record["ticket_url"] = json!(url);
            }

            // Extract ticket status
            if let Some(status) = event.get("status").and_then(|s| s.as_str()) {
                record["ticket_status"] = json!(status);
            }

            // Extract artists
            let mut artists = Vec::new();

            // First try regular artists
            if let Some(artist_list) = event.get("artists").and_then(|a| a.as_array()) {
                for artist in artist_list {
                    if let Some(name) = artist.get("name").and_then(|n| n.as_str()) {
                        artists.push(json!({
                            "name": name,
                            "bio": artist.get("bio").and_then(|b| b.as_str()),
                        }));
                    }
                }
            }

            // Then try announce artists (which have more social media info)
            if let Some(announce_list) = event.get("announceArtists").and_then(|a| a.as_array()) {
                for artist in announce_list {
                    if let Some(name) = artist.get("name").and_then(|n| n.as_str()) {
                        // Create social media links object
                        let mut links = json!({});
                        for field in ["website", "facebook", "twitter", "instagram", "spotify", "bandcamp"] {
                            if let Some(url) = artist.get(field).and_then(|u| u.as_str()) {
                                links[field] = json!(url);
                            }
                        }

                        artists.push(json!({
                            "name": name,
                            "social_links": links
                        }));
                    }
                }
            }

            if !artists.is_empty() {
                record["artists"] = json!(artists);
            }

            // Add venue info
            record["venue"] = json!({
                "name": event.get("venue").and_then(|v| v.get("name")).and_then(|n| n.as_str()).unwrap_or("Conor Byrne Pub")
            });

            // Add metadata
            record["source_id"] = json!(self.source_id);
            record["scraped_at"] = json!(Local::now().to_rfc3339());

            parsed_records.push(ParsedRecord {
                source_id: self.source_id.clone(),
                envelope_id: self.envelope_id.clone(),
                payload_ref: self.payload_ref.clone(),
                record_path: "data.paginatedEvents.collection".to_string(),
                record,
            });
        }

        Ok(parsed_records)
    }
}
