use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
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
