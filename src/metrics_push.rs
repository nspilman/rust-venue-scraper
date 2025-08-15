use std::time::Duration;
use tracing::{info, warn};

/// Start a temporary HTTP server to collect metrics and push them to the gateway
pub async fn collect_and_push_metrics(instance: &str) -> Result<(), Box<dyn std::error::Error>> {
    let pushgateway_url = std::env::var("SMS_PUSHGATEWAY_URL")
        .unwrap_or_else(|_| "http://localhost:9091".to_string());
    
    // Start a temporary server on a random port to collect metrics
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let metrics_url = format!("http://{}/metrics", addr);
    
    info!("Starting temporary metrics server on {} for collection", addr);
    
    // Create a simple HTTP server that serves metrics
    let server = async move {
        loop {
            let (stream, _) = listener.accept().await.unwrap();
            tokio::spawn(handle_metrics_request(stream));
        }
    };
    
    // Start server in background
    let server_handle = tokio::spawn(server);
    
    // Give server a moment to start
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Fetch metrics from our temporary server
    let client = reqwest::Client::new();
    let response = client.get(&metrics_url).send().await?;
    let metrics_text = response.text().await?;
    
    info!("Collected {} bytes of metrics", metrics_text.len());
    
    // Push to pushgateway
    let push_url = format!(
        "{}/metrics/job/sms_scraper/instance/{}",
        pushgateway_url.trim_end_matches('/'),
        instance
    );
    
    let push_response = client
        .post(&push_url)
        .header("Content-Type", "text/plain; version=0.0.4")
        .body(metrics_text)
        .send()
        .await?;
    
    if !push_response.status().is_success() {
        let status = push_response.status();
        let body = push_response.text().await.unwrap_or_default();
        return Err(format!("Pushgateway returned status {}: {}", status, body).into());
    }
    
    info!("Successfully pushed metrics to Pushgateway for instance={}", instance);
    
    // Abort the server
    server_handle.abort();
    
    Ok(())
}

async fn handle_metrics_request(mut stream: tokio::net::TcpStream) {
    use tokio::io::AsyncWriteExt;
    
    // Get metrics from the global registry
    let metrics = if let Some(handle) = crate::metrics::get_handle() {
        handle.render()
    } else {
        String::new()
    };
    
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/plain; version=0.0.4\r\nContent-Length: {}\r\n\r\n{}",
        metrics.len(),
        metrics
    );
    
    let _ = stream.write_all(response.as_bytes()).await;
}
