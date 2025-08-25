use serde::{Serialize, Deserialize};
use crate::observability::metrics;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParsedRecord {
    pub source_id: String,
    pub envelope_id: String,
    pub payload_ref: String,
    pub record_path: String,
    pub record: serde_json::Value,
}

pub trait Parser {
    fn parse(&self, bytes: &[u8]) -> anyhow::Result<Vec<ParsedRecord>>;
}

/// A wrapper that adds metrics to any parser implementation
pub struct MetricsParser<P: Parser> {
    inner: P,
}

impl<P: Parser> MetricsParser<P> {
    pub fn new(inner: P) -> Self {
        Self { inner }
    }
}

impl<P: Parser> Parser for MetricsParser<P> {
    fn parse(&self, bytes: &[u8]) -> anyhow::Result<Vec<ParsedRecord>> {
        let start_time = std::time::Instant::now();
        
        match self.inner.parse(bytes) {
            Ok(records) => {
                metrics::parser::parse_success();
                metrics::parser::records_extracted(records.len() as u64);
                metrics::parser::duration(start_time.elapsed().as_secs_f64());
                Ok(records)
            }
            Err(e) => {
                metrics::parser::parse_error();
                metrics::parser::duration(start_time.elapsed().as_secs_f64());
                Err(e)
            }
        }
    }
}

pub struct WixCalendarV1Parser {
    pub source_id: String,
    pub envelope_id: String,
    pub payload_ref: String,
}

impl WixCalendarV1Parser {
    pub fn new(source_id: String, envelope_id: String, payload_ref: String) -> Self {
        Self {
            source_id,
            envelope_id,
            payload_ref,
        }
    }
}

impl Parser for WixCalendarV1Parser {
    fn parse(&self, bytes: &[u8]) -> anyhow::Result<Vec<ParsedRecord>> {
        use tracing::{debug, info, warn};
        debug!("WixCalendarV1Parser: start bytes_len={}", bytes.len());
        
        // The payload for Blue Moon is often a JSON with `eventsByDates` mapping; also support plain `events`.
        let v: serde_json::Value = serde_json::from_slice(bytes)?;
        let mut out = Vec::new();

        if let Some(events) = v.get("events").and_then(|e| e.as_array()) {
            info!(
                "WixCalendarV1Parser: found events array count={}",
                events.len()
            );
            for ev in events {
                out.push(ParsedRecord {
                    source_id: self.source_id.clone(),
                    envelope_id: self.envelope_id.clone(),
                    payload_ref: self.payload_ref.clone(),
                    record_path: "$.events[*]".to_string(),
                    record: ev.clone(),
                });
            }
            return Ok(out);
        }

        if let Some(obj) = v.get("eventsByDates").and_then(|e| e.as_object()) {
            let mut total = 0usize;
            for (day, events_val) in obj {
                if let Some(arr) = events_val.as_array() {
                    total += arr.len();
                    for ev in arr {
                        let mut ev_clone = ev.clone();
                        // Inject event_day for downstream convenience (as apis/blue_moon does)
                        ev_clone["event_day"] = serde_json::Value::String(day.clone());
                        out.push(ParsedRecord {
                            source_id: self.source_id.clone(),
                            envelope_id: self.envelope_id.clone(),
                            payload_ref: self.payload_ref.clone(),
                            record_path: format!("$.eventsByDates.{}[*]", day),
                            record: ev_clone,
                        });
                    }
                }
            }
            info!(
                "WixCalendarV1Parser: aggregated events from eventsByDates total={}",
                total
            );
            return Ok(out);
        }

        warn!("WixCalendarV1Parser: neither 'events' nor 'eventsByDates' found; emitting fallback record");
        // Fallback: emit entire doc as a single record for visibility
        out.push(ParsedRecord {
            source_id: self.source_id.clone(),
            envelope_id: self.envelope_id.clone(),
            payload_ref: self.payload_ref.clone(),
            record_path: "$".to_string(),
            record: v,
        });
        Ok(out)
    }
}

// Parses HTML pages that embed a Wix warmup JSON under a script tag with id 'wix-warmup-data'.
pub struct WixWarmupV1Parser {
    pub source_id: String,
    pub envelope_id: String,
    pub payload_ref: String,
}

impl WixWarmupV1Parser {
    pub fn new(source_id: String, envelope_id: String, payload_ref: String) -> Self {
        Self {
            source_id,
            envelope_id,
            payload_ref,
        }
    }
}

impl Parser for WixWarmupV1Parser {
    fn parse(&self, bytes: &[u8]) -> anyhow::Result<Vec<ParsedRecord>> {
        use scraper::{Html, Selector};
        use tracing::{debug, info, warn};
        debug!("WixWarmupV1Parser: start bytes_len={}", bytes.len());
        let body = String::from_utf8_lossy(bytes).to_string();
        let document = Html::parse_document(&body);
        let selector =
            Selector::parse("script[type=\"application/json\"]#wix-warmup-data").unwrap();

        let mut out = Vec::new();
        if let Some(element) = document.select(&selector).next() {
            let json_text = element.inner_html();
            let data: serde_json::Value = serde_json::from_str(&json_text)?;
            let mut total = 0usize;
            if let Some(apps_data) = data["appsWarmupData"].as_object() {
                for (_, app_data) in apps_data {
                    if let Some(widgets) = app_data.as_object() {
                        for (widget_key, widget_data) in widgets {
                            if widget_key.starts_with("widget") {
                                if let Some(events_data) =
                                    widget_data.get("events").and_then(|e| e.get("events"))
                                {
                                    if let Some(events_array) = events_data.as_array() {
                                        total += events_array.len();
                                        for ev in events_array {
                                            out.push(ParsedRecord {
                                                source_id: self.source_id.clone(),
                                                envelope_id: self.envelope_id.clone(),
                                                payload_ref: self.payload_ref.clone(),
                                                record_path: format!(
                                                    "$.appsWarmupData.*.{}.events.events[*]",
                                                    widget_key
                                                ),
                                                record: ev.clone(),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            info!(
                "WixWarmupV1Parser: extracted total events={} from warmup JSON",
                total
            );
        }
        if out.is_empty() {
            warn!(
                "WixWarmupV1Parser: no events extracted; emitting fallback record with html_len={}",
                body.len()
            );
            // Fall back: emit entire HTML if nothing parsed for troubleshooting
            out.push(ParsedRecord {
                source_id: self.source_id.clone(),
                envelope_id: self.envelope_id.clone(),
                payload_ref: self.payload_ref.clone(),
                record_path: "html".to_string(),
                record: serde_json::json!({"html_len": body.len()}),
            });
        }
        Ok(out)
    }
}

// Parses Darrell's Tavern HTML schedule into basic records with title and event_day.
pub struct DarrellsHtmlV1Parser {
    pub source_id: String,
    pub envelope_id: String,
    pub payload_ref: String,
}

impl DarrellsHtmlV1Parser {
    pub fn new(source_id: String, envelope_id: String, payload_ref: String) -> Self {
        Self {
            source_id,
            envelope_id,
            payload_ref,
        }
    }
}

impl Parser for DarrellsHtmlV1Parser {
    fn parse(&self, bytes: &[u8]) -> anyhow::Result<Vec<ParsedRecord>> {
        use chrono::{Datelike, NaiveDate};
        use scraper::{Html, Selector};
        use tracing::{debug, info, warn};

        fn parse_date(text: &str) -> Option<NaiveDate> {
            // Expecting like: "MUSIC 7.12" (from earlier logic: header h1 with date)
            let parts: Vec<&str> = text.split_whitespace().collect();
            if parts.len() < 2 {
                return None;
            }
            let date_part = parts[1];
            let comps: Vec<&str> = date_part.split('.').collect();
            if comps.len() != 2 {
                return None;
            }
            let month: u32 = comps[0].parse().ok()?;
            let day: u32 = comps[1].parse().ok()?;
            let year = chrono::Utc::now().year();
            NaiveDate::from_ymd_opt(year, month, day)
        }

        fn extract_performers(element: &scraper::ElementRef) -> Vec<String> {
            let link_sel = Selector::parse("a").unwrap();
            let mut performers = Vec::new();
            for link in element.select(&link_sel) {
                let name = link.text().collect::<String>().trim().to_string();
                if !name.is_empty() {
                    performers.push(name);
                }
            }
            let text_content = element.text().collect::<String>();
            for line in text_content.split('\n') {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                if performers.iter().any(|p| line.contains(p)) {
                    continue;
                }
                if line.contains("DOORS") || line.contains("SHOW") || line.contains("$") {
                    continue;
                }
                performers.push(line.to_string());
            }
            performers
        }

        debug!("DarrellsHtmlV1Parser: start bytes_len={}", bytes.len());
        let html = String::from_utf8_lossy(bytes).to_string();
        let document = Html::parse_document(&html);
        let entry_sel = Selector::parse("div.entry-content").unwrap();

        let mut out = Vec::new();
        if let Some(entry) = document.select(&entry_sel).next() {
            let mut current_date: Option<NaiveDate> = None;
            let nodes: Vec<_> = entry.children().collect();
            let mut i = 0;
            while i < nodes.len() {
                let node = nodes[i];
                if let Some(el) = node.value().as_element() {
                    if el.name() == "h1" {
                        let element_ref = scraper::ElementRef::wrap(node).unwrap();
                        let date_text = element_ref.text().collect::<String>();
                        debug!("DarrellsHtmlV1Parser: found header date='{}'", date_text);
                        current_date = parse_date(&date_text);
                    } else if el.name() == "p" && current_date.is_some() {
                        let element_ref = scraper::ElementRef::wrap(node).unwrap();
                        let performers = extract_performers(&element_ref);
                        for perf in performers {
                            let rec = serde_json::json!({
                                "title": perf,
                                "event_day": current_date.unwrap().to_string(),
                            });
                            out.push(ParsedRecord {
                                source_id: self.source_id.clone(),
                                envelope_id: self.envelope_id.clone(),
                                payload_ref: self.payload_ref.clone(),
                                record_path: "entry-content".to_string(),
                                record: rec,
                            });
                        }
                    }
                }
                i += 1;
            }
        }
        if out.is_empty() {
            warn!("DarrellsHtmlV1Parser: no events extracted; emitting fallback record with html_len={}", html.len());
            out.push(ParsedRecord {
                source_id: self.source_id.clone(),
                envelope_id: self.envelope_id.clone(),
                payload_ref: self.payload_ref.clone(),
                record_path: "html".to_string(),
                record: serde_json::json!({"html_len": html.len()}),
            });
        } else {
            info!("DarrellsHtmlV1Parser: extracted events count={}", out.len());
        }
        Ok(out)
    }
}

// Parses KEXP HTML pages for live event listings
pub struct KexpHtmlV1Parser {
    pub source_id: String,
    pub envelope_id: String,
    pub payload_ref: String,
}

impl KexpHtmlV1Parser {
    pub fn new(source_id: String, envelope_id: String, payload_ref: String) -> Self {
        Self {
            source_id,
            envelope_id,
            payload_ref,
        }
    }
}

impl Parser for KexpHtmlV1Parser {
    fn parse(&self, bytes: &[u8]) -> anyhow::Result<Vec<ParsedRecord>> {
        use scraper::{Html, Selector};
        use tracing::{debug, info, warn};

        debug!("KexpHtmlV1Parser: start bytes_len={}", bytes.len());
        let html = String::from_utf8_lossy(bytes).to_string();
        let document = Html::parse_document(&html);
        
        // KEXP events are in article.EventItem containers with h2 date headers
        let _event_selector = Selector::parse("article.EventItem").unwrap();
        let title_selector = Selector::parse(".EventItem-body h3 a").unwrap();
        let time_selector = Selector::parse(".EventItem-DateTime h5").unwrap();
        let location_selector = Selector::parse(".EventItem-body .u-h3 a").unwrap();
        let description_selector = Selector::parse(".EventItem-description").unwrap();
        let _date_selector = Selector::parse("h2").unwrap();

        let mut out = Vec::new();
        let mut current_date: Option<String> = None;
        
        // Process the document sequentially to match dates with events (like the working KEXP API does)
        for element in document.select(&Selector::parse("h2, article.EventItem").unwrap()) {
            if element.value().name() == "h2" {
                // This is a date header
                let date_text = element.text().collect::<Vec<_>>().join(" ");
                current_date = Some(date_text.clone());
                debug!("KexpHtmlV1Parser: found date header: {}", date_text);
            } else if element.value().name() == "article" {
                // This is an event item
                let title = element
                    .select(&title_selector)
                    .next()
                    .map(|el| el.text().collect::<String>().trim().to_string())
                    .unwrap_or_else(|| "Unknown Event".to_string());
                    
                let time = element
                    .select(&time_selector)
                    .next()
                    .map(|el| el.text().collect::<String>().trim().to_string())
                    .unwrap_or_else(|| "".to_string());
                    
                let location = element
                    .select(&location_selector)
                    .next()
                    .map(|el| el.text().collect::<String>().trim().to_string())
                    .unwrap_or_else(|| "KEXP Studio".to_string());
                    
                let description = element
                    .select(&description_selector)
                    .next()
                    .map(|el| el.text().collect::<String>().trim().to_string())
                    .unwrap_or_else(|| "".to_string());

                // Skip events that don't have meaningful content
                if title.is_empty() || title == "Unknown Event" {
                    continue;
                }

                // Create the record in the same format as other parsers
                let record = serde_json::json!({
                    "title": title,
                    "event_day": current_date.clone().unwrap_or_else(|| "".to_string()),
                    "event_time": time,
                    "location": location,
                    "description": description,
                    "source": "kexp",
                    "public": true
                });
                
                debug!("KexpHtmlV1Parser: extracted event title='{}' date='{}'", title, current_date.as_deref().unwrap_or("unknown"));
                
                out.push(ParsedRecord {
                    source_id: self.source_id.clone(),
                    envelope_id: self.envelope_id.clone(),
                    payload_ref: self.payload_ref.clone(),
                    record_path: "article.EventItem".to_string(),
                    record,
                });
            }
        }
        
        if out.is_empty() {
            warn!("KexpHtmlV1Parser: no events extracted; emitting fallback record with html_len={}", html.len());
            // Fall back: emit entire HTML if nothing parsed for troubleshooting
            out.push(ParsedRecord {
                source_id: self.source_id.clone(),
                envelope_id: self.envelope_id.clone(),
                payload_ref: self.payload_ref.clone(),
                record_path: "html".to_string(),
                record: serde_json::json!({"html_len": html.len()}),
            });
        } else {
            info!("KexpHtmlV1Parser: extracted events count={}", out.len());
        }
        Ok(out)
    }
}

// Parses Barboza HTML event listings
pub struct BarbozaHtmlV1Parser {
    pub source_id: String,
    pub envelope_id: String,
    pub payload_ref: String,
}

impl BarbozaHtmlV1Parser {
    pub fn new(source_id: String, envelope_id: String, payload_ref: String) -> Self {
        Self {
            source_id,
            envelope_id,
            payload_ref,
        }
    }
}

impl Parser for BarbozaHtmlV1Parser {
    fn parse(&self, bytes: &[u8]) -> anyhow::Result<Vec<ParsedRecord>> {
        use scraper::{Html, Selector};
        use tracing::{debug, info, warn};
        use chrono::{Datelike, NaiveDate};

        debug!("BarbozaHtmlV1Parser: start bytes_len={}", bytes.len());
        let html = String::from_utf8_lossy(bytes).to_string();
        let document = Html::parse_document(&html);
        
        // Parse events using the actual HTML structure: div.eventItem
        let event_selector = Selector::parse("div.eventItem").unwrap();
        let title_selector = Selector::parse("h3.title a").unwrap();
        let tagline_selector = Selector::parse("h4.tagline").unwrap();
        let promotion_selector = Selector::parse("div.promotion-text").unwrap();
        let month_selector = Selector::parse(".m-date__month").unwrap();
        let day_selector = Selector::parse(".m-date__day").unwrap();
        let time_selector = Selector::parse(".meta .time").unwrap();
        let age_selector = Selector::parse(".meta .age").unwrap();
        let location_selector = Selector::parse(".meta .location").unwrap();
        let ticket_link_selector = Selector::parse("a.tickets").unwrap();
        let image_selector = Selector::parse(".thumb img").unwrap();

        let mut out = Vec::new();
        let current_year = chrono::Local::now().year();
        
        for event_element in document.select(&event_selector) {
            let mut record = serde_json::json!({});
            
            // Extract main title (headliner)
            if let Some(title_elem) = event_element.select(&title_selector).next() {
                let title = title_elem.text().collect::<String>().trim().to_string();
                record["title"] = serde_json::json!(title);
                
                // Also get the event detail URL
                if let Some(href) = title_elem.value().attr("href") {
                    record["detail_url"] = serde_json::json!(href);
                    // Extract event ID from URL if possible
                    if let Some(id_match) = href.split('/').last() {
                        record["id"] = serde_json::json!(id_match.to_string());
                    }
                }
            }

            // Extract tagline (supporting acts)
            if let Some(tagline_elem) = event_element.select(&tagline_selector).next() {
                let tagline = tagline_elem.text().collect::<String>().trim().to_string();
                if !tagline.is_empty() {
                    record["supporting_acts"] = serde_json::json!(tagline);
                }
            }

            // Extract promotion text (e.g., "Barboza Presents")
            if let Some(promo_elem) = event_element.select(&promotion_selector).next() {
                let promo = promo_elem.text().collect::<String>().trim().to_string();
                record["promoter"] = serde_json::json!(promo);
            }

            // Extract date (month and day)
            let mut month_str = String::new();
            let mut day_str = String::new();
            
            if let Some(month_elem) = event_element.select(&month_selector).next() {
                month_str = month_elem.text().collect::<String>().trim().to_string();
            }
            
            if let Some(day_elem) = event_element.select(&day_selector).next() {
                day_str = day_elem.text().collect::<String>().trim().to_string();
            }
            
            // Parse date and format as YYYY-MM-DD
            if !month_str.is_empty() && !day_str.is_empty() {
                let month = match month_str.to_lowercase().as_str() {
                    "jan" => 1,
                    "feb" => 2,
                    "mar" => 3,
                    "apr" => 4,
                    "may" => 5,
                    "jun" => 6,
                    "jul" => 7,
                    "aug" => 8,
                    "sep" => 9,
                    "oct" => 10,
                    "nov" => 11,
                    "dec" => 12,
                    _ => 0,
                };
                
                if let Ok(day) = day_str.parse::<u32>() {
                    if month > 0 {
                        if let Some(date) = NaiveDate::from_ymd_opt(current_year, month, day) {
                            record["event_day"] = serde_json::json!(date.to_string());
                            record["date_text"] = serde_json::json!(format!("{} {}", month_str, day_str));
                        }
                    }
                }
            }

            // Extract time (e.g., "Doors: 6:00 PM")
            if let Some(time_elem) = event_element.select(&time_selector).next() {
                let time_text = time_elem.text().collect::<String>().trim().to_string();
                record["time_text"] = serde_json::json!(time_text);
                // Try to extract just the time part
                if time_text.contains(":") {
                    let cleaned = time_text.replace("Doors: ", "").replace("doors: ", "");
                    record["event_time"] = serde_json::json!(cleaned);
                }
            }

            // Extract age restriction
            if let Some(age_elem) = event_element.select(&age_selector).next() {
                let age_text = age_elem.text().collect::<String>().trim().to_string();
                record["age_restriction"] = serde_json::json!(age_text);
            }

            // Extract location/venue
            if let Some(location_elem) = event_element.select(&location_selector).next() {
                let location = location_elem.text().collect::<String>().trim().to_string();
                record["venue"] = serde_json::json!(location);
            } else {
                record["venue"] = serde_json::json!("The Barboza");
            }

            // Extract ticket purchase link
            if let Some(ticket_elem) = event_element.select(&ticket_link_selector).next() {
                if let Some(href) = ticket_elem.value().attr("href") {
                    record["ticket_url"] = serde_json::json!(href);
                }
                // Check if tickets are on sale
                let class_attr = ticket_elem.value().attr("class").unwrap_or("");
                record["tickets_on_sale"] = serde_json::json!(class_attr.contains("onsalenow"));
            }

            // Extract event image
            if let Some(img_elem) = event_element.select(&image_selector).next() {
                if let Some(src) = img_elem.value().attr("src") {
                    record["image_url"] = serde_json::json!(src);
                }
            }

            // Mark all events as public
            record["public"] = serde_json::json!(true);
            record["source"] = serde_json::json!("barboza");

            // Only add if we have at least a title or date
            if record.get("title").is_some() || record.get("event_day").is_some() {
                out.push(ParsedRecord {
                    source_id: self.source_id.clone(),
                    envelope_id: self.envelope_id.clone(),
                    payload_ref: self.payload_ref.clone(),
                    record_path: "div.eventItem".to_string(),
                    record,
                });
            }
        }
        
        if out.is_empty() {
            warn!("BarbozaHtmlV1Parser: no events extracted; emitting fallback record with html_len={}", html.len());
            out.push(ParsedRecord {
                source_id: self.source_id.clone(),
                envelope_id: self.envelope_id.clone(),
                payload_ref: self.payload_ref.clone(),
                record_path: "html".to_string(),
                record: serde_json::json!({"html_len": html.len()}),
            });
        } else {
            info!("BarbozaHtmlV1Parser: extracted events count={}", out.len());
        }
        Ok(out)
    }
}
