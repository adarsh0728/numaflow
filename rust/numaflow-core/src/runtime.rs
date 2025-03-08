use chrono::Utc;
use regex::Regex;
use reqwest::{Client, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use std::str;
use tonic::Status;

use crate::config::get_vertex_replica;
use crate::get_vertex_name;

#[derive(Serialize, Deserialize, Debug)]
struct RuntimeError {
    container_name: String,
    timestamp: String,
    code: String,
    message: String,
    details: String,
    mvtx_name: String,
    replica: String,
}

pub struct Runtime {
    host_url: String,
    client: Client,
}

impl Runtime {
    /// Creates a new Runtime instance with host address
    pub fn new(address: &str) -> Result<Runtime> {
        let host_url = if !address.starts_with("https://") {
            format!("https://{}", address)
        } else {
            address.to_string()
        };

        let client = Client::builder()
            .timeout(Duration::from_secs(1))
            .use_rustls_tls()
            .danger_accept_invalid_certs(true)
            .build()?;

        Ok(Runtime { host_url, client })
    }

    // Call daemon server
    pub async fn persist_application_error(&self, grpc_status: Status) -> Result<()> {
        let url = format!("{}/api/v1/runtime/errors", self.host_url);

        let container_name =
            extract_container_name(grpc_status.message()).expect("container name not found");

        let timestamp = Utc::now().to_rfc3339();

        let replica = get_vertex_replica().to_string();
        let code = grpc_status.code().to_string();
        let message = grpc_status.message().to_string();
        let details = String::from_utf8_lossy(grpc_status.details()).to_string();

        let app_error = RuntimeError {
            container_name,
            timestamp,
            code,
            message,
            details,
            mvtx_name: get_vertex_name().to_string(),
            replica,
        };
        match self.client.post(&url).json(&app_error).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    println!("Error persisted successfully");
                    Ok(())
                } else {
                    println!("Res status not successful {:?}", response.status());
                    Ok(())
                }
            }
            Err(e) => {
                println!("Persisr Error POST Request failed: {:?}", e);
                Err(e)
            }
        }
    }
}

fn extract_container_name(error_message: &str) -> Option<String> {
    let re = Regex::new(r"\((.*?)\)").unwrap();
    re.captures(error_message)
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()))
}
