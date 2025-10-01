use actix_web::{App, HttpServer, web};
use dotenvy::dotenv;
use std::env;

mod routes;
use store::Store;

use routes::*;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set in .env");

    let store = Store::new(&database_url)
        .await
        .expect("Failed to connect to database");

    store.migrate().await.expect("Failed to run migrations");

    HttpServer::new(|| {
        App::new()
            .service(sign_up)
            .service(sign_in)
            .service(get_user)
            .service(quote)
            .service(swap)
            .service(sol_balance)
            .service(token_balance)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
