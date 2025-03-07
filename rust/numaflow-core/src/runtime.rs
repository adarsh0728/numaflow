use chrono::Utc;
use hyper::{Body, Method};
use hyper::{Client, Request};
use hyper_tls::HttpsConnector;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::error::Error;
use tracing::info;

use std::str;
use tonic::Status;

use crate::config::{get_vertex_name, get_vertex_replica};
pub struct Runtime {}

#[derive(Serialize, Deserialize, Debug)]
pub struct RuntimeErrorEntry {
    pub container_name: String,
    pub timestamp: String,
    pub code: String,
    pub message: String,
    pub details: String,
    pub mvtx_name: String,
    pub replica: String,
}

impl Runtime {
    /// Creates a new Runtime instance with the specified emptyDir path.
    pub fn new() -> Self {
        Runtime {}
    }

    // Call daemon server
    pub async fn persist_application_error(
        &self,
        grpc_status: Status,
    ) -> Result<(), Box<dyn Error>> {
        // we can extract the type of udf based on the error message
        let container_name = match get_container_name(grpc_status.message()) {
            Ok(name) => name,
            Err(_err) => String::from(""),
        };

        let timestamp = Utc::now().to_rfc3339();
        let vertex_name = get_vertex_name().to_string();
        let replica = get_vertex_replica().to_string();
        let code = grpc_status.code().to_string();
        let message = grpc_status.message().to_string();
        let details_bytes = grpc_status.details();
        let details_str = String::from_utf8_lossy(details_bytes).to_string();
        let daemon_errors_url =
            "http://simple-mono-vertex-mv-daemon-svc.default.svc:4327/api/v1/runtime/errors";
        let daemon_metrics_url =
            "http://simple-mono-vertex-mv-daemon-svc.default.svc:4327/api/v1/metrics";

        let error_entry = RuntimeErrorEntry {
            container_name,
            timestamp,
            code,
            message,
            details: details_str,
            mvtx_name: vertex_name,
            replica,
        };

        let https = HttpsConnector::new();
        let client = Client::builder()
            .http2_only(true)
            .build::<_, hyper::Body>(https);

        info!(
            "Reporting runtime error to daemon server client: {:?}",
            client
        );

        let json_body = serde_json::to_string(&error_entry)?;
        let req = Request::builder()
            .method(Method::POST)
            .uri(daemon_errors_url)
            .header("Content-Type", "application/json")
            .body(Body::from(json_body))?;

        let metrics_req = Request::builder()
            .method(Method::GET)
            .uri(daemon_metrics_url)
            .body(Body::empty())?;

        let metrics_res = client.request(metrics_req).await?;
        let errors_response = client.request(req).await?;

        if metrics_res.status().is_success() {
            println!("Metrics req success!")
        } else {
            println!("Failed to get metrics!: {}", errors_response.status());
        }
        if errors_response.status().is_success() {
            println!("Runtime error reported successfully");
        } else {
            println!(
                "Failed to report runtime error: {}",
                errors_response.status()
            );
        }

        Ok(())
    }
}

fn get_container_name(error_message: &str) -> Result<String, String> {
    extract_container_name(error_message)
        .ok_or_else(|| "Failed to extract container name from error message".to_string())
}

fn extract_container_name(error_message: &str) -> Option<String> {
    let re = Regex::new(r"\((.*?)\)").unwrap();
    re.captures(error_message)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
}
