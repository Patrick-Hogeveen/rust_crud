use axum::http;
use axum::routing::{get, post, put, delete, Router};
use sqlx::postgres::PgPoolOptions;
use std::env;
use dotenv::dotenv;
mod handlers;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std:: error::Error>> {
    dotenv().ok();
    let port = 3000;
    let addr = format!("0.0.0.0:{}", port);

    let database_url = env::var("DATABASE_URL").expect("missing DATABASE_URL env");

    let pool = PgPoolOptions::new()
                .max_connections(5)
                .connect(&database_url)
                .await?;

    let app = Router::new().route("/", get(handlers::health))
                           .route("/recipe", post(handlers::create_recipe))
                           .route("/recipe", get(handlers::read_recipes))
                           .route("/ingredients", post(handlers::get_ingredients))
                           .route("/recipe/:id", put(handlers::update))
                           .route("/recipe/:id", delete(handlers::delete_recipe))
                           .with_state(pool);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app)
        .await
        .unwrap();

    Ok(())
}

