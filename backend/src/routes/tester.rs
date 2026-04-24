use axum::{response::Html, routing::get, Router};

use crate::state::AppState;

static TESTER_HTML: &str = include_str!("../../../test-app/index.html");

pub fn router() -> Router<AppState> {
    Router::new().route("/sync-tester", get(serve_tester))
}

async fn serve_tester() -> Html<&'static str> {
    Html(TESTER_HTML)
}
