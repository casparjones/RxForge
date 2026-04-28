use anyhow::{anyhow, Context};
use async_trait::async_trait;
use bytes::Bytes;
use futures_core::Stream;
use mongodb::{
    bson::{self, doc, Document},
    options::ReturnDocument,
    Client, Collection, Database,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{pin::Pin, time::Duration};

use crate::linker::{BulkDocsResult, ChangesResult, Linker};

pub struct MongoDbLinker {
    db: Database,
}

impl MongoDbLinker {
    pub async fn new(uri: &str, db_name: &str) -> anyhow::Result<Self> {
        let client = Client::with_uri_str(uri)
            .await
            .with_context(|| format!("failed to connect to MongoDB at {uri}"))?;
        Ok(Self {
            db: client.database(db_name),
        })
    }

    fn collection(&self, db_name: &str) -> Collection<Document> {
        self.db.collection(db_name)
    }

    fn meta_collection(&self) -> Collection<Document> {
        self.db.collection("__rxf_meta")
    }

    async fn next_seq(&self, db_name: &str) -> anyhow::Result<i64> {
        let meta_id = format!("seq_{db_name}");
        let updated = self
            .meta_collection()
            .find_one_and_update(doc! { "_id": meta_id }, doc! { "$inc": { "value": 1_i64 } })
            .upsert(true)
            .return_document(ReturnDocument::After)
            .await
            .with_context(|| format!("failed to allocate sequence for {db_name}"))?
            .ok_or_else(|| anyhow!("MongoDB did not return sequence document for {db_name}"))?;

        updated
            .get_i64("value")
            .context("MongoDB sequence document missing numeric value")
    }

    async fn existing_doc(
        &self,
        db_name: &str,
        doc_id: &str,
    ) -> anyhow::Result<Option<Document>> {
        self.collection(db_name)
            .find_one(doc! { "_id": doc_id })
            .await
            .with_context(|| format!("failed to fetch existing MongoDB doc {doc_id} from {db_name}"))
    }

    fn revision_number(existing: Option<&str>) -> i64 {
        existing
            .and_then(|rev| rev.split('-').next())
            .and_then(|n| n.parse::<i64>().ok())
            .unwrap_or(0)
    }

    fn make_rev(doc: &Value, current_rev: Option<&str>) -> anyhow::Result<String> {
        let mut for_hash = doc.clone();
        if let Some(obj) = for_hash.as_object_mut() {
            obj.remove("_rev");
            obj.remove("_seq");
        }
        let bytes = serde_json::to_vec(&for_hash).context("failed to serialize doc for revision")?;
        let digest = Sha256::digest(bytes);
        let hex = hex::encode(digest);
        Ok(format!(
            "{}-{}",
            Self::revision_number(current_rev) + 1,
            &hex[..16]
        ))
    }

    fn strip_seq(mut doc: Value) -> Value {
        if let Some(obj) = doc.as_object_mut() {
            obj.remove("_seq");
        }
        doc
    }

    fn doc_id(doc: &Value) -> Option<String> {
        doc.get("_id").and_then(|v| v.as_str()).map(str::to_string)
    }

    fn doc_rev(doc: &Value) -> Option<String> {
        doc.get("_rev").and_then(|v| v.as_str()).map(str::to_string)
    }

    async fn write_doc(
        &self,
        db_name: &str,
        doc_id: &str,
        doc: Value,
        expected_rev: Option<&str>,
    ) -> anyhow::Result<Value> {
        let existing = self.existing_doc(db_name, doc_id).await?;
        let existing_rev = existing
            .as_ref()
            .and_then(|doc| doc.get_str("_rev").ok())
            .map(str::to_string);

        if existing.is_some() && existing_rev.as_deref() != expected_rev {
            return Err(anyhow!(
                "revision conflict for doc {doc_id}: expected {:?}, found {:?}",
                expected_rev,
                existing_rev
            ));
        }

        let seq = self.next_seq(db_name).await?;
        let mut stored = doc;
        let new_rev = Self::make_rev(&stored, existing_rev.as_deref())?;
        if let Some(obj) = stored.as_object_mut() {
            obj.insert("_id".to_string(), Value::String(doc_id.to_string()));
            obj.insert("_rev".to_string(), Value::String(new_rev));
            obj.insert("_seq".to_string(), Value::Number(seq.into()));
        }
        let result = stored.clone();
        let bson_doc = bson::to_document(&stored).context("failed to convert JSON doc to BSON")?;

        self.collection(db_name)
            .replace_one(doc! { "_id": doc_id }, bson_doc)
            .upsert(true)
            .await
            .with_context(|| format!("failed to upsert MongoDB doc {doc_id} into {db_name}"))?;

        Ok(Self::strip_seq(result))
    }
}

#[async_trait]
impl Linker for MongoDbLinker {
    async fn ensure_db(&self, _db_name: &str) -> anyhow::Result<()> {
        Ok(())
    }

    async fn delete_db(&self, db_name: &str) -> anyhow::Result<()> {
        self.collection(db_name)
            .drop()
            .await
            .with_context(|| format!("failed to drop MongoDB collection {db_name}"))
    }

    async fn get_changes(
        &self,
        db_name: &str,
        since: Option<&str>,
        limit: u32,
    ) -> anyhow::Result<ChangesResult> {
        let since_seq = since.and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
        let mut cursor = self
            .collection(db_name)
            .find(doc! { "_seq": { "$gt": since_seq }, "_id": { "$ne": "__meta" } })
            .sort(doc! { "_seq": 1_i32 })
            .limit(limit as i64)
            .await
            .with_context(|| format!("failed to query MongoDB changes for {db_name}"))?;

        let mut documents = Vec::new();
        let mut last_seq = since_seq;
        while cursor.advance().await.context("failed to advance MongoDB changes cursor")? {
            let doc: Document = cursor.deserialize_current()?;
            if let Ok(seq) = doc.get_i64("_seq") {
                last_seq = seq;
            }
            let value: Value =
                bson::from_document(doc).context("failed to convert MongoDB change doc to JSON")?;
            documents.push(Self::strip_seq(value));
        }

        Ok(ChangesResult {
            documents,
            checkpoint: Value::String(last_seq.to_string()),
        })
    }

    async fn bulk_docs(&self, db_name: &str, docs: Vec<Value>) -> anyhow::Result<BulkDocsResult> {
        let mut written = 0;
        let mut conflicts = Vec::new();

        for mut doc in docs {
            let Some(doc_id) = Self::doc_id(&doc) else {
                conflicts.push("unknown".to_string());
                continue;
            };
            let expected_rev = Self::doc_rev(&doc);
            if let Some(obj) = doc.as_object_mut() {
                obj.remove("_seq");
            }

            match self.write_doc(db_name, &doc_id, doc, expected_rev.as_deref()).await {
                Ok(_) => written += 1,
                Err(err) if err.to_string().contains("revision conflict") => conflicts.push(doc_id),
                Err(err) => return Err(err),
            }
        }

        Ok(BulkDocsResult { written, conflicts })
    }

    fn changes_stream(
        &self,
        db_name: String,
    ) -> Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send + 'static>> {
        let db = self.db.clone();
        let stream = async_stream::stream! {
            let linker = MongoDbLinker { db };
            let mut checkpoint = "0".to_string();
            let mut interval = tokio::time::interval(Duration::from_millis(2000));
            loop {
                interval.tick().await;
                yield Ok(Bytes::from(": heartbeat\n\n"));
                match linker.get_changes(&db_name, Some(&checkpoint), 100).await {
                    Ok(changes) => {
                        if !changes.documents.is_empty() {
                            checkpoint = changes
                                .checkpoint
                                .as_str()
                                .map(str::to_string)
                                .unwrap_or_else(|| changes.checkpoint.to_string());
                            let payload = serde_json::json!({
                                "documents": changes.documents,
                                "checkpoint": changes.checkpoint,
                            });
                            let event = format!("event: change\ndata: {}\n\n", payload);
                            yield Ok(Bytes::from(event));
                        }
                    }
                    Err(err) => {
                        let event = format!(
                            "event: error\ndata: {{\"error\":{}}}\n\n",
                            serde_json::Value::String(err.to_string())
                        );
                        yield Ok(Bytes::from(event));
                    }
                }
            }
        };

        Box::pin(stream)
    }

    async fn list_docs(
        &self,
        db_name: &str,
        limit: u32,
        skip: u32,
    ) -> anyhow::Result<(Vec<Value>, u64)> {
        let filter = doc! { "_id": { "$ne": "__meta" } };
        let total = self
            .collection(db_name)
            .count_documents(filter.clone())
            .await
            .with_context(|| format!("failed to count MongoDB docs for {db_name}"))?;

        let mut cursor = self
            .collection(db_name)
            .find(filter)
            .skip(skip as u64)
            .limit(limit as i64)
            .await
            .with_context(|| format!("failed to list MongoDB docs for {db_name}"))?;

        let mut docs = Vec::new();
        while cursor.advance().await.context("failed to advance MongoDB list cursor")? {
            let doc: Document = cursor.deserialize_current()?;
            let value: Value =
                bson::from_document(doc).context("failed to convert MongoDB doc to JSON")?;
            docs.push(Self::strip_seq(value));
        }

        Ok((docs, total))
    }

    async fn get_doc(&self, db_name: &str, doc_id: &str) -> anyhow::Result<Option<Value>> {
        let result = self
            .collection(db_name)
            .find_one(doc! { "_id": doc_id })
            .await
            .with_context(|| format!("failed to fetch MongoDB doc {doc_id} from {db_name}"))?;

        result
            .map(|doc| {
                bson::from_document(doc)
                    .map(Self::strip_seq)
                    .context("failed to convert MongoDB doc to JSON")
            })
            .transpose()
    }

    async fn put_doc(&self, db_name: &str, doc_id: &str, doc: Value) -> anyhow::Result<Value> {
        let expected_rev = Self::doc_rev(&doc);
        self.write_doc(db_name, doc_id, doc, expected_rev.as_deref()).await
    }

    async fn delete_doc(
        &self,
        db_name: &str,
        doc_id: &str,
        rev: &str,
    ) -> anyhow::Result<Value> {
        let existing = self.existing_doc(db_name, doc_id).await?;
        let existing_rev = existing
            .as_ref()
            .and_then(|doc| doc.get_str("_rev").ok())
            .map(str::to_string);

        if existing.is_some() && existing_rev.as_deref() != Some(rev) {
            return Err(anyhow!(
                "revision conflict for doc {doc_id}: expected {:?}, found {:?}",
                rev,
                existing_rev
            ));
        }

        let seq = self.next_seq(db_name).await?;
        let tombstone_rev = Self::make_rev(
            &serde_json::json!({ "_id": doc_id, "_deleted": true }),
            Some(rev),
        )?;
        let tombstone = serde_json::json!({
            "_id": doc_id,
            "_rev": tombstone_rev,
            "_deleted": true,
            "_seq": seq,
        });

        self.collection(db_name)
            .replace_one(
                doc! { "_id": doc_id },
                bson::to_document(&tombstone).context("failed to convert tombstone to BSON")?,
            )
            .upsert(true)
            .await
            .with_context(|| format!("failed to delete MongoDB doc {doc_id} from {db_name}"))?;

        Ok(serde_json::json!({ "ok": true, "id": doc_id, "rev": tombstone["_rev"] }))
    }

    async fn delete_all_docs(&self, db_name: &str) -> anyhow::Result<usize> {
        let result = self
            .collection(db_name)
            .delete_many(doc! { "_id": { "$ne": "__meta" } })
            .await
            .with_context(|| format!("failed to delete all MongoDB docs from {db_name}"))?;
        Ok(result.deleted_count as usize)
    }
}
