use sqlx::PgPool;

use crate::{analytics::AnalyticsSender, config::Config, couchdb::CouchDbClient, jwt::JwtManager};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub config: Config,
    pub jwt: JwtManager,
    pub couchdb: CouchDbClient,
    pub analytics: AnalyticsSender,
}
