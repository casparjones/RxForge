use sqlx::PgPool;
use std::sync::Arc;

use crate::{analytics::AnalyticsSender, config::Config, jwt::JwtManager, linker::Linker};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Config,
    pub jwt: JwtManager,
    pub linker: Arc<dyn Linker + Send + Sync>,
    pub analytics: AnalyticsSender,
}
