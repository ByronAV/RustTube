use std::{env, io, sync::OnceLock};
use actix_web::{web, App, HttpServer};
use mongodb::options::{ ClientOptions, ServerApi, ServerApiVersion };
use lapin::{Connection, ConnectionProperties};

mod api;

// We're retrieving the necessary env vars before beginning the service
static PORT: OnceLock<u16> = OnceLock::new();
static RABBIT: OnceLock<String> = OnceLock::new();
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

fn get_rabbit() -> &'static str {
    RABBIT.get_or_init(|| {
        env::var("RABBIT")
            .expect("Please specify the server for the RabbitMQ microservice in variable RABBIT.")
    }).as_str()
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
    println!("Connecting to MongoDB at {}:{} ...", get_db_host(), get_db_name());

    let mut client_options = ClientOptions::parse(get_db_host()).await.expect("Failed to parse MongoDB client options");
    client_options.server_api = Some(ServerApi::builder().version(ServerApiVersion::V1).build());

    let mongo_client = mongodb::Client::with_options(client_options)
        .expect("Failed to create MongoDB client with the provided options");

    let mongo_data = web::Data::new(mongo_client);

    println!("Connecting to RabbitMQ at {} ...", get_rabbit());
    let rabbit_conn = Connection::connect(get_rabbit(), ConnectionProperties::default())
        .await
        .expect("Failed to connect to RabbitMQ");
    let rabbit_channel = rabbit_conn.create_channel()
        .await
        .expect("Failed to create RabbitMQ channel");

    let rabbit_data = web::Data::new(rabbit_channel);

    HttpServer::new(move || {
        println!("Backend online.");
        App::new()
            .app_data(rabbit_data.clone())
            .app_data(mongo_data.clone())
            .service(api::get_video)
            .service(api::health_check)
    })
    .bind(format!("0.0.0.0:{}", get_port()))?
    .run()
    .await
}





#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_get_port() {
        std::env::set_var("PORT", "8080");
        assert_eq!(get_port(), 8080);
    }

    #[test]
    #[serial]
    fn test_get_rabbit() {
        std::env::set_var("RABBIT", "amqp://localhost:5672");
        assert_eq!(get_rabbit(), "amqp://localhost:5672");
    }

    #[test]
    #[serial]
    fn test_get_video_storage_host() {
        std::env::set_var("VIDEO_STORAGE_HOST", "localhost");
        assert_eq!(get_video_storage_host(), "localhost");
    }

    #[test]
    #[should_panic(expected = "Please specify the port number")]
    #[serial]
    fn test_get_port_missing() {
        std::env::remove_var("PORT");
        get_port();
    }

    #[test]
    #[serial]
    fn test_get_db_host_and_name() {
        std::env::set_var("DBHOST", "mongodb://localhost:27017");
        std::env::set_var("DBNAME", "rusttube");
        assert_eq!(get_db_host(), "mongodb://localhost:27017");
        assert_eq!(get_db_name(), "rusttube");
    }
}