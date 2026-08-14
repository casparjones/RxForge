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
    /// Separate client without a request timeout, used for long-lived streams.
    stream_client: Client,
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

        let stream_client = Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            // No overall timeout — streams are long-lived by design
            .build()
            .expect("Failed to build streaming HTTP client");

        Self {
            client,
            stream_client,
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
            .put(self.url(db_name))
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

    /// List all CouchDB databases whose name starts with `prefix`.
    pub async fn list_dbs_with_prefix(&self, prefix: &str) -> Result<Vec<String>> {
        let url = format!(
            "{}/_all_dbs?start_key=\"{}\"&end_key=\"{}{{\"",
            self.base_url, prefix, prefix
        );
        let response = self
            .client
            .get(&url)
            .basic_auth(&self.user, Some(&self.password))
            .send()
            .await?;
        let dbs: Vec<String> = response.json().await?;
        Ok(dbs.into_iter().filter(|d| d.starts_with(prefix)).collect())
    }

    /// Delete a CouchDB database.
    pub async fn delete_db(&self, db_name: &str) -> Result<()> {
        let response = self
            .client
            .delete(self.url(db_name))
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
                        // Filter out CouchDB design documents only; keep _deleted tombstones
                        doc.get("_id")
                            .and_then(|id| id.as_str())
                            .map(|id| !id.starts_with('_'))
                            .unwrap_or(false)
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

    /// Stream _changes feed as SSE events with `event: change`.
    ///
    /// CouchDB's continuous feed emits newline-delimited JSON.  Each non-empty
    /// line is a change record that we reshape into the same
    /// `{ documents, checkpoint }` envelope used by the pull endpoint, then
    /// emit as a proper SSE event so the browser's EventSource can fire it.
    pub fn changes_stream(
        &self,
        db_name: String,
    ) -> Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send>> {
        let url = format!(
            "{}/{}/_changes?feed=continuous&include_docs=true&heartbeat=15000",
            self.base_url, db_name
        );

        // Use the no-timeout client so the connection isn't killed after 30 s.
        let client = self.stream_client.clone();
        let user = self.user.clone();
        let password = self.password.clone();

        let stream = async_stream::stream! {
            let response = client
                .get(&url)
                .basic_auth(&user, Some(&password))
                .send()
                .await;

            match response {
                Err(e) => {
                    tracing::error!("CouchDB stream connect error: {e}");
                    let event = format!("event: error\ndata: {{\"error\":\"{e}\"}}\n\n");
                    yield Ok(Bytes::from(event));
                }
                Ok(resp) => {
                    // Buffer incomplete lines across chunks
                    let mut line_buf = String::new();
                    let mut byte_stream = resp.bytes_stream();
                    use futures_util::StreamExt;

                    while let Some(chunk) = byte_stream.next().await {
                        match chunk {
                            Err(e) => {
                                tracing::error!("CouchDB stream read error: {e}");
                                break;
                            }
                            Ok(bytes) => {
                                line_buf.push_str(&String::from_utf8_lossy(&bytes));

                                // Process all complete newline-terminated lines
                                while let Some(pos) = line_buf.find('\n') {
                                    let line = line_buf[..pos].trim().to_string();
                                    line_buf = line_buf[pos + 1..].to_string();

                                    if line.is_empty() {
                                        // CouchDB heartbeat — send SSE comment to keep connection alive
                                        yield Ok(Bytes::from(": heartbeat\n\n"));
                                        continue;
                                    }

                                    if let Ok(record) = serde_json::from_str::<Value>(&line) {
                                        let doc = record.get("doc").cloned();
                                        let seq = record.get("seq").cloned()
                                            .unwrap_or(Value::String("0".to_string()));

                                        // Skip only CouchDB design docs; include _deleted tombstones
                                        let skip = doc.as_ref().map(|d| {
                                            d.get("_id")
                                                .and_then(|id| id.as_str())
                                                .map(|id| id.starts_with('_'))
                                                .unwrap_or(false)
                                        }).unwrap_or(true);

                                        if skip { continue; }

                                        let payload = serde_json::json!({
                                            "documents": [doc.unwrap()],
                                            "checkpoint": seq,
                                        });

                                        // Emit as SSE `change` event matching what the client expects
                                        let event = format!(
                                            "event: change\ndata: {}\n\n",
                                            payload
                                        );
                                        yield Ok(Bytes::from(event));
                                    }
                                }
                            }
                        }
                    }
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
            .get(self.url(""))
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

    /// List non-design documents from a database.
    ///
    /// Supports a `deleted` filter (active / deleted / all) and a free-text `search`
    /// matched against each document's JSON representation. `_all_docs` only returns
    /// live documents, so tombstones are gathered from `_changes` when requested.
    /// Returns `(docs, total)` where `total` is the number of documents matching the
    /// filter and search (not the raw CouchDB row count).
    pub async fn list_docs(
        &self,
        db_name: &str,
        limit: u32,
        skip: u32,
        search: Option<&str>,
        deleted: crate::linker::DeletedFilter,
    ) -> Result<(Vec<Value>, u64)> {
        use crate::linker::DeletedFilter;

        let needle = search
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_lowercase);

        // Fast path: live documents, no search — let CouchDB do skip/limit.
        if needle.is_none() && deleted == DeletedFilter::Active {
            let url = format!(
                "{}/{}/_all_docs?include_docs=true&limit={}&skip={}",
                self.base_url, db_name, limit, skip
            );
            let response = self
                .client
                .get(&url)
                .basic_auth(&self.user, Some(&self.password))
                .send()
                .await?;
            if response.status() == StatusCode::NOT_FOUND {
                return Ok((vec![], 0));
            }
            if !response.status().is_success() {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                anyhow::bail!("CouchDB _all_docs error {}: {}", status, body);
            }
            let payload: Value = response.json().await?;
            let total = payload.get("total_rows").and_then(|v| v.as_u64()).unwrap_or(0);
            let documents = payload
                .get("rows")
                .and_then(|rows| rows.as_array())
                .map(|rows| {
                    rows.iter()
                        .filter_map(|row| row.get("doc").cloned())
                        .filter(|doc| Self::is_user_doc(doc))
                        .collect()
                })
                .unwrap_or_default();
            return Ok((documents, total));
        }

        // General path: gather candidate documents, then filter/search/window in memory.
        let mut candidates: Vec<Value> = Vec::new();

        if deleted != DeletedFilter::Deleted {
            let url = format!("{}/{}/_all_docs?include_docs=true", self.base_url, db_name);
            let response = self
                .client
                .get(&url)
                .basic_auth(&self.user, Some(&self.password))
                .send()
                .await?;
            if response.status() != StatusCode::NOT_FOUND {
                if !response.status().is_success() {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    anyhow::bail!("CouchDB _all_docs error {}: {}", status, body);
                }
                let payload: Value = response.json().await?;
                if let Some(rows) = payload.get("rows").and_then(|r| r.as_array()) {
                    for doc in rows.iter().filter_map(|row| row.get("doc").cloned()) {
                        if Self::is_user_doc(&doc) {
                            candidates.push(doc);
                        }
                    }
                }
            }
        }

        if deleted != DeletedFilter::Active {
            // Tombstones are not surfaced by `_all_docs`; pull them from `_changes`.
            let url = format!("{}/{}/_changes?include_docs=true", self.base_url, db_name);
            let response = self
                .client
                .get(&url)
                .basic_auth(&self.user, Some(&self.password))
                .send()
                .await?;
            if response.status() != StatusCode::NOT_FOUND {
                if !response.status().is_success() {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    anyhow::bail!("CouchDB _changes error {}: {}", status, body);
                }
                let payload: Value = response.json().await?;
                if let Some(results) = payload.get("results").and_then(|r| r.as_array()) {
                    for doc in results.iter().filter_map(|row| row.get("doc").cloned()) {
                        let is_deleted = doc
                            .get("_deleted")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        if is_deleted && Self::is_user_doc(&doc) {
                            candidates.push(doc);
                        }
                    }
                }
            }
        }

        if let Some(needle) = &needle {
            candidates.retain(|doc| {
                serde_json::to_string(doc)
                    .unwrap_or_default()
                    .to_lowercase()
                    .contains(needle)
            });
        }

        let total = candidates.len() as u64;
        let documents = candidates
            .into_iter()
            .skip(skip as usize)
            .take(limit as usize)
            .collect();

        Ok((documents, total))
    }

    /// A user document is any non-design document (id does not start with `_`).
    fn is_user_doc(doc: &Value) -> bool {
        doc.get("_id")
            .and_then(|id| id.as_str())
            .map(|id| !id.starts_with('_'))
            .unwrap_or(false)
    }

    /// Delete all non-design documents using bulk tombstones.
    pub async fn delete_all_docs(&self, db_name: &str) -> Result<usize> {
        let url = format!("{}/{}/_all_docs", self.base_url, db_name);
        let response = self.client.get(&url).basic_auth(&self.user, Some(&self.password)).send().await?;
        if response.status() == StatusCode::NOT_FOUND { return Ok(0); }
        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("CouchDB _all_docs: {}", body);
        }
        let data: Value = response.json().await?;
        let tombstones: Vec<Value> = data.get("rows")
            .and_then(|r| r.as_array())
            .map(|rows| rows.iter().filter_map(|row| {
                let id = row.get("id")?.as_str()?;
                if id.starts_with('_') { return None; }
                let rev = row.get("value")?.get("rev")?.as_str()?;
                Some(serde_json::json!({ "_id": id, "_rev": rev, "_deleted": true }))
            }).collect())
            .unwrap_or_default();
        let count = tombstones.len();
        if count == 0 { return Ok(0); }
        let bulk_url = format!("{}/{}/_bulk_docs", self.base_url, db_name);
        let res = self.client.post(&bulk_url).basic_auth(&self.user, Some(&self.password))
            .json(&serde_json::json!({ "docs": tombstones })).send().await?;
        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            anyhow::bail!("CouchDB bulk delete: {}", body);
        }
        Ok(count)
    }

    /// Collect `(id, rev, deleted)` for every leaf change from `_changes`.
    async fn changes_index(&self, db_name: &str) -> Result<Vec<(String, String, bool)>> {
        let url = format!("{}/{}/_changes", self.base_url, db_name);
        let response = self.client.get(&url).basic_auth(&self.user, Some(&self.password)).send().await?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(vec![]);
        }
        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("CouchDB _changes: {}", body);
        }
        let data: Value = response.json().await?;
        let mut out = Vec::new();
        if let Some(rows) = data.get("results").and_then(|r| r.as_array()) {
            for row in rows {
                let id = match row.get("id").and_then(|v| v.as_str()) {
                    Some(id) if !id.starts_with('_') => id.to_string(),
                    _ => continue,
                };
                let rev = row
                    .get("changes")
                    .and_then(|c| c.as_array())
                    .and_then(|c| c.first())
                    .and_then(|c| c.get("rev"))
                    .and_then(|v| v.as_str());
                let deleted = row.get("deleted").and_then(|v| v.as_bool()).unwrap_or(false);
                if let Some(rev) = rev {
                    out.push((id, rev.to_string(), deleted));
                }
            }
        }
        Ok(out)
    }

    async fn purge_by_revs(&self, db_name: &str, body: serde_json::Map<String, Value>) -> Result<usize> {
        if body.is_empty() {
            return Ok(0);
        }
        let count = body.len();
        let url = format!("{}/{}/_purge", self.base_url, db_name);
        let res = self
            .client
            .post(&url)
            .basic_auth(&self.user, Some(&self.password))
            .json(&Value::Object(body))
            .send()
            .await?;
        if !res.status().is_success() {
            let body = res.text().await.unwrap_or_default();
            anyhow::bail!("CouchDB _purge: {}", body);
        }
        Ok(count)
    }

    /// Permanently remove the given documents (hard delete via `_purge`).
    pub async fn purge_docs(&self, db_name: &str, ids: &[String]) -> Result<usize> {
        let wanted: std::collections::HashSet<&str> = ids.iter().map(String::as_str).collect();
        let mut body = serde_json::Map::new();
        for (id, rev, _deleted) in self.changes_index(db_name).await? {
            if wanted.contains(id.as_str()) {
                body.insert(id, serde_json::json!([rev]));
            }
        }
        self.purge_by_revs(db_name, body).await
    }

    /// Permanently remove every tombstone (hard delete via `_purge`).
    pub async fn purge_deleted(&self, db_name: &str) -> Result<usize> {
        let mut body = serde_json::Map::new();
        for (id, rev, deleted) in self.changes_index(db_name).await? {
            if deleted {
                body.insert(id, serde_json::json!([rev]));
            }
        }
        self.purge_by_revs(db_name, body).await
    }

    /// Fetch a single document by id.
    pub async fn get_doc(&self, db_name: &str, doc_id: &str) -> Result<Option<Value>> {
        let url = format!(
            "{}/{}/{}",
            self.base_url,
            db_name,
            urlencoding::encode(doc_id)
        );

        let response = self
            .client
            .get(&url)
            .basic_auth(&self.user, Some(&self.password))
            .send()
            .await?;

        match response.status() {
            StatusCode::OK => Ok(Some(response.json().await?)),
            StatusCode::NOT_FOUND => Ok(None),
            status => {
                let body = response.text().await.unwrap_or_default();
                anyhow::bail!("CouchDB GET /{}/{} -> {}: {}", db_name, doc_id, status, body);
            }
        }
    }

    /// Create or update a document with a caller-provided id.
    pub async fn put_doc(&self, db_name: &str, doc_id: &str, mut document: Value) -> Result<Value> {
        if let Some(object) = document.as_object_mut() {
            object.insert("_id".to_string(), Value::String(doc_id.to_string()));
        }

        let url = format!(
            "{}/{}/{}",
            self.base_url,
            db_name,
            urlencoding::encode(doc_id)
        );

        let response = self
            .client
            .put(&url)
            .basic_auth(&self.user, Some(&self.password))
            .json(&document)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("CouchDB PUT /{}/{} -> {}: {}", db_name, doc_id, status, body);
        }

        Ok(response.json().await?)
    }

    /// Delete a document by id and revision.
    pub async fn delete_doc(&self, db_name: &str, doc_id: &str, rev: &str) -> Result<Value> {
        let url = format!(
            "{}/{}/{}?rev={}",
            self.base_url,
            db_name,
            urlencoding::encode(doc_id),
            urlencoding::encode(rev)
        );

        let response = self
            .client
            .delete(&url)
            .basic_auth(&self.user, Some(&self.password))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("CouchDB DELETE /{}/{} -> {}: {}", db_name, doc_id, status, body);
        }

        Ok(response.json().await?)
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
