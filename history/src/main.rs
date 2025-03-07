use std::{env, io, sync::OnceLock};
use actix_web::{App, HttpServer};

static PORT: OnceLock<u16> = OnceLock::new();
static DBHOST: OnceLock<String> = OnceLock::new();
static DBNAME: OnceLock<String> = OnceLock::new();

mod api;

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

#[tokio::main(flavor="current_thread")]
async fn main() -> io::Result<()> {
    HttpServer::new(|| {
        println!("History online.");
        App::new()
            .service(api::post_viewed)
    })
    .bind(format!("0.0.0.0:{}", get_port()))?
    .run()
    .await
}