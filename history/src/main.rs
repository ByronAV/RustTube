use std::{env, io, sync::OnceLock, sync::Arc};
use actix_web::{web, App, HttpServer};
use tokio::sync::Mutex;
use mongodb::options::{ ClientOptions, ServerApi, ServerApiVersion };

static PORT: OnceLock<u16> = OnceLock::new();
static RABBIT: OnceLock<String> = OnceLock::new();
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

fn get_rabbit() -> &'static str {
    RABBIT.get_or_init(|| {
        env::var("RABBIT")
            .expect("Please specify the server for the RabbitMQ microservice in variable RABBIT.")
    }).as_str()
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

    println!("Connecting to MongoDB at {}:{} ...", get_db_host(), get_db_name());

    let mut client_options = ClientOptions::parse(get_db_host()).await.expect("Failed to parse MongoDB client options");
    client_options.server_api = Some(ServerApi::builder().version(ServerApiVersion::V1).build());

    let mongo_client = mongodb::Client::with_options(client_options)
        .expect("Failed to create MongoDB client with the provided options");

    let mongo_data = web::Data::new(mongo_client);

    // Create the msg channel and queue
    let msg_channel = api::connect_to_msg_channel().await.unwrap();
    let queue = match api::assert_exchange(&msg_channel, "viewed").await {
        Ok(q) => {
            // Ok, got the queue
            q
        },
        Err(e) => {
            // Here we are panicking because without a queue nothing *should* work
            panic!("Error in creating message exchange or queue: {e}");
        }
    };

    // Wrap the channel in Arc<Mutex<>> for thread-safe sharing
    let shared_channel = Arc::new(Mutex::new(msg_channel));

    // Clone the channel for the consumer task
    let consumer_channel = shared_channel.clone();

    // Spawn a task to consume messages
    tokio::spawn(async move {
        if let Err(e) = api::consume_viewed_msg(consumer_channel, queue.name().as_str(), mongo_data.clone()).await {
            eprintln!("Error consuming `viewed` messages: {}", e);
        }
    });

    HttpServer::new(|| {
        println!("History online.");
        App::new()
            .service(api::health_check)
    })
    .bind(format!("0.0.0.0:{}", get_port()))?
    .run()
    .await
}