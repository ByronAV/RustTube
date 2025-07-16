use std::{env, io, sync::OnceLock};
use actix_web::{web, App, HttpServer};
use jsonwebtoken::EncodingKey;
use mongodb::Client;

mod api;

static PORT: OnceLock<u16> = OnceLock::new();
static DBHOST: OnceLock<String> = OnceLock::new();
static DBNAME: OnceLock<String> = OnceLock::new();
static JWT_SECRET: OnceLock<String> = OnceLock::new();

fn get_port() -> u16 {
    *PORT.get_or_init(|| {
        env::var("PORT")
            .ok()
            .and_then(|val| val.parse::<u16>().ok())
            .expect("Please specify the port number for the HTTP server with the environment variable PORT.")
    })
}

fn get_db_host() -> &'static str {
    DBHOST.get_or_init(|| {
        env::var("DBHOST")
            .expect("Please specify the variable for the database host in variable DBHOST.")
    }).as_str()
}

fn get_db_name() -> &'static str {
    DBNAME.get_or_init(|| {
        env::var("DBNAME")
            .expect("Please specify the variable for the database name in variable DBNAME.")
    }).as_str()
}

fn get_jwt_secret() -> &'static str {
    JWT_SECRET.get_or_init(|| {
        env::var("JWT_SECRET")
            .expect("Please specify the variable for the database name in variable JWT_SECRET.")
    }).as_str()
}

#[tokio::main(flavor="current_thread")]
async fn main() -> io::Result<()> {
    // Connect to MongoDB
    let client = Client::with_uri_str(get_db_host()).await.expect("Failed to connect to MongoDB");
    let users_col = client.database(get_db_name()).collection::<api::User>("users");

    let state = api::AppState {
        users: users_col,
        jwt_key: EncodingKey::from_secret(get_jwt_secret().as_bytes())
    };

    println!("Users microservice online...");
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(state.clone()))
            .service(api::login)
            .service(api::register)
    })
    .bind(format!("0.0.0.0:{}", get_port()))?
    .run()
    .await
}