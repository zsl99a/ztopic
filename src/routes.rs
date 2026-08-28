use axum::{
    extract::State,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};

use crate::TopicManager;

pub fn topic_routes<S>(manager: TopicManager<S>) -> Router
where
    S: Send + Sync + 'static,
{
    Router::new().route("/", get(get_topics)).with_state(manager)
}

pub use topic_routes as helium_routes;

async fn get_topics<S>(State(manager): State<TopicManager<S>>) -> impl IntoResponse
where
    S: Send + Sync + 'static,
{
    let topics = manager.topics();

    Html(format!("<html><body><h1>Topics</h1><pre>{}</pre></body></html>", topics.join("\n")))
}
