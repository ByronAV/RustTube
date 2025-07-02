extern crate actix_web;

use std::{env, io, sync::OnceLock};
use actix_web::{App, HttpServer};

mod api;

// We're retrieving the necessary env vars before beginning the service
static PORT: OnceLock<u16> = OnceLock::new();
static STORAGE_ACCOUNT_NAME : OnceLock<String> = OnceLock::new();
static STORAGE_ACCESS_KEY: OnceLock<String> = OnceLock::new();

fn get_port() -> u16 {
    *PORT.get_or_init(|| {
        env::var("PORT")
            .ok()
            .and_then(|val| val.parse::<u16>().ok())
            .expect("Please specify the port number for the HTTP server with the environment variable PORT.")
    })
}

fn get_storage_account_name() -> &'static str {
    STORAGE_ACCOUNT_NAME.get_or_init(|| {
        env::var("STORAGE_ACCOUNT_NAME")
            .expect("Please specify the name of an Azure storage account in environment variable STORAGE_ACCOUNT_NAME.")
    }).as_str()
}

fn get_storage_access_key() -> &'static str {
    STORAGE_ACCESS_KEY.get_or_init(|| {
        env::var("STORAGE_ACCESS_KEY")
            .expect("Please specify the access key to an Azure storage account in environment variable STORAGE_ACCESS_KEY.")
    }).as_str()
}

#[tokio::main(flavor="current_thread")]
async fn main() -> io::Result<()> {
    println!("Serving videos from Azure storage account {}", get_storage_account_name());

    HttpServer::new(|| {
        println!("Storage Online.");
        App::new()
            .service(api::get_video)
            .service(api::store_video)
            .service(api::health_check)
        })
        .bind(format!("0.0.0.0:{}", get_port()))?
        .run()
        .await
}