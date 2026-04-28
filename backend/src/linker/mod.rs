use async_trait::async_trait;
use bytes::Bytes;
use futures_core::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::pin::Pin;

pub mod couchdb;
pub mod mongodb;

pub fn slugify(s: &str) -> String {
    let mut result = String::new();
    let mut prev_under = true;
    for c in s.to_lowercase().chars() {
        if c.is_alphanumeric() {
            result.push(c);
            prev_under = false;
        } else if !prev_under {
            result.push('_');
            prev_under = true;
        }
    }
    if result.ends_with('_') {
        result.pop();
    }
    result
}

pub fn normalized_db_name(
    app_name: &str,
    app_id: &uuid::Uuid,
    db_scope: &str,
    user_email: Option<&str>,
    user_id: &uuid::Uuid,
) -> String {
    let app_slug = slugify(app_name);
    let short_app: String = app_id.simple().to_string().chars().take(8).collect();
    if db_scope == "shared" {
        format!("{}_{}", app_slug, short_app)
    } else {
        let user_slug = user_email
            .and_then(|e| e.split('@').next())
            .map(slugify)
            .unwrap_or_else(|| {
                format!(
                    "u{}",
                    user_id.simple().to_string().chars().take(8).collect::<String>()
                )
            });
        format!("{}_{}_{}", app_slug, short_app, user_slug)
    }
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

#[async_trait]
pub trait Linker: Send + Sync {
    async fn ensure_db(&self, db_name: &str) -> anyhow::Result<()>;
    async fn delete_db(&self, db_name: &str) -> anyhow::Result<()>;
    async fn list_dbs(&self, prefix: &str) -> anyhow::Result<Vec<String>>;
    async fn get_changes(
        &self,
        db_name: &str,
        since: Option<&str>,
        limit: u32,
    ) -> anyhow::Result<ChangesResult>;
    async fn bulk_docs(&self, db_name: &str, docs: Vec<Value>) -> anyhow::Result<BulkDocsResult>;
    fn changes_stream(
        &self,
        db_name: String,
    ) -> Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send + 'static>>;
    async fn list_docs(&self, db_name: &str, limit: u32, skip: u32)
        -> anyhow::Result<(Vec<Value>, u64)>;
    async fn get_doc(&self, db_name: &str, doc_id: &str) -> anyhow::Result<Option<Value>>;
    async fn put_doc(&self, db_name: &str, doc_id: &str, doc: Value) -> anyhow::Result<Value>;
    async fn delete_doc(
        &self,
        db_name: &str,
        doc_id: &str,
        rev: &str,
    ) -> anyhow::Result<Value>;
    async fn delete_all_docs(&self, db_name: &str) -> anyhow::Result<usize>;
}
