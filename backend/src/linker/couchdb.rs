use anyhow::Context;
use async_trait::async_trait;
use bytes::Bytes;
use futures_core::Stream;
use serde_json::Value;
use std::pin::Pin;

use crate::linker::{BulkDocsResult, ChangesResult, Linker};

pub struct CouchDbLinker(pub crate::couchdb::CouchDbClient);

#[async_trait]
impl Linker for CouchDbLinker {
    async fn ensure_db(&self, db_name: &str) -> anyhow::Result<()> {
        self.0
            .provision_db(db_name)
            .await
            .with_context(|| format!("failed to ensure CouchDB database {db_name}"))
    }

    async fn delete_db(&self, db_name: &str) -> anyhow::Result<()> {
        self.0
            .delete_db(db_name)
            .await
            .with_context(|| format!("failed to delete CouchDB database {db_name}"))
    }

    async fn list_dbs(&self, prefix: &str) -> anyhow::Result<Vec<String>> {
        self.0
            .list_dbs_with_prefix(prefix)
            .await
            .with_context(|| format!("failed to list CouchDB databases with prefix {prefix}"))
    }

    async fn get_changes(
        &self,
        db_name: &str,
        since: Option<&str>,
        limit: u32,
    ) -> anyhow::Result<ChangesResult> {
        let result = self
            .0
            .get_changes(db_name, since, limit)
            .await
            .with_context(|| format!("failed to get changes for CouchDB database {db_name}"))?;

        Ok(ChangesResult {
            documents: result.documents,
            checkpoint: result.checkpoint,
        })
    }

    async fn bulk_docs(&self, db_name: &str, docs: Vec<Value>) -> anyhow::Result<BulkDocsResult> {
        let result = self
            .0
            .bulk_docs(db_name, docs)
            .await
            .with_context(|| format!("failed to bulk write CouchDB database {db_name}"))?;

        Ok(BulkDocsResult {
            written: result.written,
            conflicts: result.conflicts,
        })
    }

    fn changes_stream(
        &self,
        db_name: String,
    ) -> Pin<Box<dyn Stream<Item = Result<Bytes, std::io::Error>> + Send + 'static>> {
        self.0.changes_stream(db_name)
    }

    async fn list_docs(
        &self,
        db_name: &str,
        limit: u32,
        skip: u32,
    ) -> anyhow::Result<(Vec<Value>, u64)> {
        self.0
            .list_docs(db_name, limit, skip)
            .await
            .with_context(|| format!("failed to list CouchDB docs for {db_name}"))
    }

    async fn get_doc(&self, db_name: &str, doc_id: &str) -> anyhow::Result<Option<Value>> {
        self.0
            .get_doc(db_name, doc_id)
            .await
            .with_context(|| format!("failed to get CouchDB doc {doc_id} from {db_name}"))
    }

    async fn put_doc(&self, db_name: &str, doc_id: &str, doc: Value) -> anyhow::Result<Value> {
        self.0
            .put_doc(db_name, doc_id, doc)
            .await
            .with_context(|| format!("failed to put CouchDB doc {doc_id} into {db_name}"))
    }

    async fn delete_doc(
        &self,
        db_name: &str,
        doc_id: &str,
        rev: &str,
    ) -> anyhow::Result<Value> {
        self.0
            .delete_doc(db_name, doc_id, rev)
            .await
            .with_context(|| format!("failed to delete CouchDB doc {doc_id} from {db_name}"))
    }

    async fn delete_all_docs(&self, db_name: &str) -> anyhow::Result<usize> {
        self.0
            .delete_all_docs(db_name)
            .await
            .with_context(|| format!("failed to delete all CouchDB docs from {db_name}"))
    }
}
