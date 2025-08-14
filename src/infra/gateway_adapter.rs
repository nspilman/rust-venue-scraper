use crate::app::ports::GatewayPort;
use async_trait::async_trait;

pub struct GatewayAdapter {
    pub root: std::path::PathBuf,
}

#[async_trait]
impl GatewayPort for GatewayAdapter {
    async fn accept(&self, env: crate::envelope::EnvelopeSubmissionV1, bytes: Vec<u8>) -> Result<crate::envelope::StampedEnvelopeV1, String> {
        // Gateway::accept is sync; wrap if we later need blocking behavior.
        let gw = crate::gateway::Gateway::new(self.root.clone());
        gw.accept(env, &bytes).map_err(|e| e.to_string())
    }
}

