use anyhow::{Context, Result};
use bytes::Bytes;
use futures_core::Stream;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::pin::Pin;

#[derive(Clone)]
pub struct CouchDbClient {
    client: Client,
    base_url: String,
    user: String,
    password: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ChangesResult {
    pub documents: Vec<Value>,
    pub checkpoint: Value,
}

#[derive(Debug)]
pub struct BulkDocsResult {
    pub written: usize,
    pub conflicts: Vec<String>,
}

impl CouchDbClient {
    pub fn new(base_url: &str, user: &str, password: &str) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            user: user.to_string(),
            password: password.to_string(),
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.base_url, path.trim_start_matches('/'))
    }

    /// Provision a new CouchDB database for a user/app combination.
    pub async fn provision_db(&self, db_name: &str) -> Result<()> {
        let response = self
            .client
            .put(&self.url(db_name))
            .basic_auth(&self.user, Some(&self.password))
            .send()
            .await
            .context("Failed to connect to CouchDB")?;

        match response.status() {
            StatusCode::CREATED => {
                tracing::info!("CouchDB database '{}' created", db_name);
                Ok(())
            }
            StatusCode::PRECONDITION_FAILED => {
                // Database already exists – that's fine
                tracing::debug!("CouchDB database '{}' already exists", db_name);
                Ok(())
            }
            status => {
                let body = response.text().await.unwrap_or_default();
                anyhow::bail!("CouchDB PUT /{}  → {}: {}", db_name, status, body);
            }
        }
    }

    /// Delete a CouchDB database.
    pub async fn delete_db(&self, db_name: &str) -> Result<()> {
        let response = self
            .client
            .delete(&self.url(db_name))
            .basic_auth(&self.user, Some(&self.password))
            .send()
            .await?;

        match response.status() {
            StatusCode::OK | StatusCode::NOT_FOUND => Ok(()),
            status => {
                let body = response.text().await.unwrap_or_default();
                anyhow::bail!("CouchDB DELETE /{}  → {}: {}", db_name, status, body);
            }
        }
    }

    /// Fetch documents from the _changes feed with checkpoint-based pagination.
    pub async fn get_changes(
        &self,
        db_name: &str,
        since: Option<&str>,
        limit: u32,
    ) -> Result<ChangesResult> {
        let since = since.unwrap_or("0");
        let url = format!(
            "{}/{}/_changes?include_docs=true&since={}&limit={}",
            self.base_url, db_name, since, limit
        );

        let response = self
            .client
            .get(&url)
            .basic_auth(&self.user, Some(&self.password))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("CouchDB _changes error {}: {}", status, body);
        }

        let changes: Value = response.json().await?;

        let last_seq = changes
            .get("last_seq")
            .cloned()
            .unwrap_or(Value::String("0".to_string()));

        let documents = changes
            .get("results")
            .and_then(|r| r.as_array())
            .map(|results| {
                results
                    .iter()
                    .filter_map(|row| row.get("doc").cloned())
                    .filter(|doc| {
                        // Filter out design documents and deleted docs
                        doc.get("_id")
                            .and_then(|id| id.as_str())
                            .map(|id| !id.starts_with('_'))
                            .unwrap_or(false)
                            && doc.get("_deleted").is_none()
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(ChangesResult {
            documents,
            checkpoint: last_seq,
        })
    }

    /// Bulk write documents to CouchDB.
    pub async fn bulk_docs(&self, db_name: &str, documents: Vec<Value>) -> Result<BulkDocsResult> {
        let url = format!("{}/{}/_bulk_docs", self.base_url, db_name);

        let body = serde_json::json!({ "docs": documents });

        let response = self
            .client
            .post(&url)
            .basic_auth(&self.user, Some(&self.password))
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("CouchDB _bulk_docs error {}: {}", status, body);
        }

        let results: Vec<Value> = response.json().await?;

        let mut written = 0;
        let mut conflicts = Vec::new();

        for result in &results {
            if result.get("error").is_some() {
                let id = result
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();
                conflicts.push(id);
            } else {
                written += 1;
            }
        }

        Ok(BulkDocsResult { written, conflicts })
    }

    /// Stream _changes feed as SSE-compatible byte stream.
    pub async fn changes_stream(
        &self,
        db_name: String,
    ) -> Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>> {
        let url = format!(
            "{}/{}/_changes?feed=continuous&include_docs=true&heartbeat=30000",
            self.base_url, db_name
        );

        let client = self.client.clone();
        let user = self.user.clone();
        let password = self.password.clone();

        // Build a stream that wraps the CouchDB continuous feed
        let stream = async_stream::stream! {
            let response = client
                .get(&url)
                .basic_auth(&user, Some(&password))
                .send()
                .await;

            match response {
                Ok(resp) => {
                    let mut byte_stream = resp.bytes_stream();
                    use futures_util::StreamExt;
                    while let Some(chunk) = byte_stream.next().await {
                        match chunk {
                            Ok(bytes) => {
                                // Wrap as SSE event
                                let event = format!("data: {}\n\n", String::from_utf8_lossy(&bytes));
                                yield Ok(Bytes::from(event));
                            }
                            Err(e) => {
                                tracing::error!("CouchDB stream error: {e}");
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to connect to CouchDB stream: {e}");
                    let event = format!("event: error\ndata: {e}\n\n");
                    yield Ok(Bytes::from(event));
                }
            }
        };

        Box::pin(stream)
    }

    /// Configure CouchDB to accept RxForge JWTs directly.
    ///
    /// Writes the RSA public key and required JWT claims to
    /// `/_node/_local/_config/jwt_auth/…`. Requires CouchDB admin credentials.
    pub async fn configure_jwt_auth(&self, public_key_pem: &str) -> Result<()> {
        // CouchDB's JWT auth config lives under a compound key. We set the
        // RSA public key, required claims, and the authentication handler.
        let entries: [(&str, &str); 2] = [
            ("jwt_keys/rsa:rxforge", public_key_pem),
            ("jwt_auth/required_claims", "exp"),
        ];

        for (key, value) in entries {
            let url = format!("{}/_node/_local/_config/{}", self.base_url, key);
            let response = self
                .client
                .put(&url)
                .basic_auth(&self.user, Some(&self.password))
                .json(&serde_json::Value::String(value.to_string()))
                .send()
                .await
                .with_context(|| format!("Failed to PUT {url}"))?;

            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                anyhow::bail!("CouchDB JWT config PUT {key} failed {status}: {body}");
            }
        }

        tracing::info!("CouchDB JWT auth configured for public key");
        Ok(())
    }

    /// Provision a per-user CouchDB database for an app.
    ///
    /// Follows the couch_peruser-style naming convention:
    ///   `app_{app_id}_user_{user_id}`.
    /// Idempotent: an existing database (412 Precondition Failed) is treated
    /// as success.
    pub async fn provision_user_db(&self, app_id: &str, user_id: &str) -> Result<String> {
        let db_name = format!("app_{}_user_{}", app_id, user_id);
        self.provision_db(&db_name).await?;
        Ok(db_name)
    }

    /// Check CouchDB health.
    pub async fn health_check(&self) -> Result<()> {
        let response = self
            .client
            .get(&self.url(""))
            .basic_auth(&self.user, Some(&self.password))
            .send()
            .await
            .context("Cannot reach CouchDB")?;

        if response.status().is_success() {
            Ok(())
        } else {
            anyhow::bail!("CouchDB health check failed: {}", response.status())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_couchdb_client_url() {
        let client = CouchDbClient::new("http://localhost:5984", "admin", "password");
        assert_eq!(client.url("mydb"), "http://localhost:5984/mydb");
        assert_eq!(client.url("/mydb"), "http://localhost:5984/mydb");
    }

    #[test]
    fn test_db_name_format() {
        let app_id = "abc123";
        let user_id = "def456";
        let db_name = format!("app_{}_user_{}", app_id, user_id);
        assert_eq!(db_name, "app_abc123_user_def456");
    }
}
