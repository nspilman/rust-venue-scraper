use std::net::SocketAddr;

pub fn init_metrics() {
    let port: u16 = std::env::var("SMS_METRICS_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(9898);
    let addr: SocketAddr = ([0, 0, 0, 0], port).into();
    let builder = metrics_exporter_prometheus::PrometheusBuilder::new().with_http_listener(addr);
    println!("[metrics] Attempting to install Prometheus exporter on {}", addr);
    match builder.install() {
        Ok(()) => {
            println!("[metrics] Prometheus exporter installed and listening on http://{}/metrics", addr);
        }
        Err(e) => {
            println!("[metrics] Prometheus exporter install failed (possibly already installed): {}", e);
        }
    }
}
