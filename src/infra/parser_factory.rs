use crate::app::ports::{ParserFactory, ParserPort};
use crate::pipeline::processing::parser::{Parser, MetricsParser};
use crate::observability::metrics;
use async_trait::async_trait;

pub struct DefaultParserFactory;

impl ParserFactory for DefaultParserFactory {
    fn for_plan(&self, plan: &str) -> Option<Box<dyn ParserPort>> {
        match plan {
            "parse_plan:wix_calendar_v1" => Some(Box::new(WixCalendarAdapter)),
            "parse_plan:wix_warmup_v1" => Some(Box::new(WixWarmupAdapter)),
            "parse_plan:darrells_html_v1" => Some(Box::new(DarrellsHtmlAdapter)),
            "parse_plan:kexp_html_v1" => Some(Box::new(KexpHtmlAdapter)),
            "parse_plan:barboza_html_v1" => Some(Box::new(BarbozaHtmlAdapter)),
            _ => None,
        }
    }
}

struct WixCalendarAdapter;
struct WixWarmupAdapter;
struct DarrellsHtmlAdapter;
struct KexpHtmlAdapter;
struct BarbozaHtmlAdapter;

#[async_trait]
impl ParserPort for WixCalendarAdapter {
    async fn parse(&self, source_id: &str, envelope_id: &str, payload_ref: &str, bytes: &[u8]) -> Result<Vec<String>, String> {
        metrics::parser::batch_size(1); // Single parse operation
        let inner_parser = crate::pipeline::processing::parser::WixCalendarV1Parser::new(
            source_id.to_string(), 
            envelope_id.to_string(), 
            payload_ref.to_string()
        );
        let p = MetricsParser::new(inner_parser);
        let recs = p.parse(bytes).map_err(|e| e.to_string())?;
        recs.into_iter().map(|r| serde_json::to_string(&r).map_err(|e| e.to_string())).collect()
    }
}

#[async_trait]
impl ParserPort for WixWarmupAdapter {
    async fn parse(&self, source_id: &str, envelope_id: &str, payload_ref: &str, bytes: &[u8]) -> Result<Vec<String>, String> {
        metrics::parser::batch_size(1); // Single parse operation
        let inner_parser = crate::pipeline::processing::parser::WixWarmupV1Parser::new(
            source_id.to_string(), 
            envelope_id.to_string(), 
            payload_ref.to_string()
        );
        let p = MetricsParser::new(inner_parser);
        let recs = p.parse(bytes).map_err(|e| e.to_string())?;
        recs.into_iter().map(|r| serde_json::to_string(&r).map_err(|e| e.to_string())).collect()
    }
}

#[async_trait]
impl ParserPort for DarrellsHtmlAdapter {
    async fn parse(&self, source_id: &str, envelope_id: &str, payload_ref: &str, bytes: &[u8]) -> Result<Vec<String>, String> {
        metrics::parser::batch_size(1); // Single parse operation
        let inner_parser = crate::pipeline::processing::parser::DarrellsHtmlV1Parser::new(
            source_id.to_string(), 
            envelope_id.to_string(), 
            payload_ref.to_string()
        );
        let p = MetricsParser::new(inner_parser);
        let recs = p.parse(bytes).map_err(|e| e.to_string())?;
        recs.into_iter().map(|r| serde_json::to_string(&r).map_err(|e| e.to_string())).collect()
    }
}

#[async_trait]
impl ParserPort for KexpHtmlAdapter {
    async fn parse(&self, source_id: &str, envelope_id: &str, payload_ref: &str, bytes: &[u8]) -> Result<Vec<String>, String> {
        metrics::parser::batch_size(1); // Single parse operation
        let inner_parser = crate::pipeline::processing::parser::KexpHtmlV1Parser::new(
            source_id.to_string(), 
            envelope_id.to_string(), 
            payload_ref.to_string()
        );
        let p = MetricsParser::new(inner_parser);
        let recs = p.parse(bytes).map_err(|e| e.to_string())?;
        recs.into_iter().map(|r| serde_json::to_string(&r).map_err(|e| e.to_string())).collect()
    }
}

#[async_trait]
impl ParserPort for BarbozaHtmlAdapter {
    async fn parse(&self, source_id: &str, envelope_id: &str, payload_ref: &str, bytes: &[u8]) -> Result<Vec<String>, String> {
        metrics::parser::batch_size(1); // Single parse operation
        let inner_parser = crate::pipeline::processing::parser::BarbozaHtmlV1Parser::new(
            source_id.to_string(), 
            envelope_id.to_string(), 
            payload_ref.to_string()
        );
        let p = MetricsParser::new(inner_parser);
        let recs = p.parse(bytes).map_err(|e| e.to_string())?;
        recs.into_iter().map(|r| serde_json::to_string(&r).map_err(|e| e.to_string())).collect()
    }
}

