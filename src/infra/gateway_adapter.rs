use crate::app::ports::GatewayPort;
use async_trait::async_trait;

pub struct GatewayAdapter {
    pub root: std::path::PathBuf,
}

#[async_trait]
impl GatewayPort for GatewayAdapter {
    async fn accept(&self, env: crate::pipeline::ingestion::envelope::EnvelopeSubmissionV1, bytes: Vec<u8>) -> Result<crate::pipeline::ingestion::envelope::StampedEnvelopeV1, String> {
        // Use the ingestion Gateway directly
        let gw = crate::pipeline::ingestion::gateway::Gateway::new(self.root.clone());
        gw.accept(env, &bytes).map_err(|e| e.to_string())
    }
}

