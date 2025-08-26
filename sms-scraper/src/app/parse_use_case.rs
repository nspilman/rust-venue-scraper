use crate::app::ports::{ParserFactory, PayloadStorePort, RegistryPort};

pub struct ParseUseCase<R: RegistryPort + ?Sized, S: PayloadStorePort + ?Sized, F: ParserFactory + ?Sized> {
    pub registry: Box<R>,
    pub payloads: Box<S>,
    pub parsers: Box<F>,
}

impl<R: RegistryPort + ?Sized, S: PayloadStorePort + ?Sized, F: ParserFactory + ?Sized> ParseUseCase<R, S, F> {
    pub fn new(registry: Box<R>, payloads: Box<S>, parsers: Box<F>) -> Self {
        Self { registry, payloads, parsers }
    }

    // Given a single ingest log item (source_id, envelope_id, payload_ref), resolve and parse.
    pub async fn parse_one(&self, source_id: &str, envelope_id: &str, payload_ref: &str) -> Result<Vec<String>, String> {
        let plan = self.registry.load_parse_plan(source_id).await?;
        let bytes = self.payloads.get(payload_ref).await?;
        let parser = self
            .parsers
            .for_plan(&plan)
            .ok_or_else(|| format!("no_parser_for_plan:{}", plan))?;
        parser.parse(source_id, envelope_id, payload_ref, &bytes).await
    }
}

