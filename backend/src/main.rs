use std::{env, io, sync::OnceLock};
use actix_web::{App, HttpServer};

mod api;

// We're retrieving the necessary env vars before beginning the service
static PORT: OnceLock<u16> = OnceLock::new();
static HISTORY_PORT: OnceLock<u16> = OnceLock::new();
static VIDEO_STORAGE_HOST : OnceLock<String> = OnceLock::new();
static VIDEO_STORAGE_PORT: OnceLock<u16> = OnceLock::new();
static DBHOST: OnceLock<String> = OnceLock::new();
static DBNAME: OnceLock<String> = OnceLock::new();

fn get_port() -> u16 {
    *PORT.get_or_init(|| {
        env::var("PORT")
            .ok()
            .and_then(|val| val.parse::<u16>().ok())
            .expect("Please specify the port number for the HTTP server with the environment variable PORT.")
    })
}

fn get_history_port() -> u16 {
    *HISTORY_PORT.get_or_init(|| {
        env::var("HISTORY_PORT")
            .ok()
            .and_then(|val| val.parse::<u16>().ok())
            .expect("Please specify the port number for the HTTP server with the environment variable HISTORY_PORT.")
    })
}


fn get_video_storage_host() -> &'static str {
    VIDEO_STORAGE_HOST.get_or_init(|| {
        env::var("VIDEO_STORAGE_HOST")
            .expect("Please specify the host name for the video storage microservice in variable VIDEO_STORAGE_HOST.")
    }).as_str()
}

fn get_video_storage_port() -> u16 {
    *VIDEO_STORAGE_PORT.get_or_init(|| {
        env::var("VIDEO_STORAGE_PORT")
            .ok()
            .and_then(|val| val.parse::<u16>().ok())
            .expect("Please specify the port number for the video storage microservice in variable VIDEO_STORAGE_PORT.")
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
    println!("Forwarding video requests to {}:{}", get_video_storage_host(), get_video_storage_port());

    HttpServer::new(|| {
        println!("Backend online.");
        App::new()
            .service(api::get_video)
    })
    .bind(format!("0.0.0.0:{}", get_port()))?
    .run()
    .await
}

