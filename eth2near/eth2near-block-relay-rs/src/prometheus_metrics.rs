use prometheus::{
    IntCounter, Registry,
};

use warp::Rejection;
use warp::Reply;
use warp::Filter;
use lazy_static::lazy_static;
use tokio::runtime::Runtime;

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();

    pub static ref SUBMITTED_HEADERS: IntCounter =
        IntCounter::new("submitted_headers", "Submitted headers").expect("metric can be created");
}

fn register_custom_metrics() {
    REGISTRY
        .register(Box::new(SUBMITTED_HEADERS.clone()))
        .expect("collector can be registered");
}

async fn metrics_handler() -> Result<impl Reply, Rejection> {
    use prometheus::Encoder;
    let encoder = prometheus::TextEncoder::new();

    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&REGISTRY.gather(), &mut buffer) {
        eprintln!("could not encode custom metrics: {}", e);
    };
    let mut res = match String::from_utf8(buffer.clone()) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("custom metrics could not be from_utf8'd: {}", e);
            String::default()
        }
    };
    buffer.clear();

    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&prometheus::gather(), &mut buffer) {
        eprintln!("could not encode prometheus metrics: {}", e);
    };
    let res_custom = match String::from_utf8(buffer.clone()) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("prometheus metrics could not be from_utf8'd: {}", e);
            String::default()
        }
    };
    buffer.clear();

    res.push_str(&res_custom);
    Ok(res)
}

pub fn run_prometheus_service() {
    register_custom_metrics();

    let metrics_route = warp::path!("metrics").and_then(metrics_handler);

    let rt = Runtime::new().unwrap();
    let handle = rt.handle();

    println!("Started on port 32221");
    handle.block_on(warp::serve(metrics_route).run(([0, 0, 0, 0], 32221)));
}