use actix_web::HttpResponse;
use mongodb::{ bson::doc, options::{ ClientOptions, ServerApi, ServerApiVersion }};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use lapin::{message::Delivery, options::*, types::FieldTable, Channel, Connection, ConnectionProperties, Consumer};
use futures_lite::stream::StreamExt;
use std::{sync::Arc, error::Error as StdError, fmt::Debug};
use tokio::sync::Mutex;

use crate::get_rabbit;

#[derive(Serialize, Deserialize)]
struct History {
    video_path: String
}

pub async fn connect_to_msg_channel() -> Result<Channel, lapin::Error> {

    // Connect to RabbitMQ server
    let addr = get_rabbit();
    let conn = Connection::connect(
        &addr,
        ConnectionProperties::default(),
    ).await?;

    println!("Connected to RabbitMQ from History Microservice.");

    // Create a channel
    conn.create_channel().await
}

pub async fn create_queue(msg_channel: &Channel, queue_name: &str) -> Result<(), lapin::Error> {

    let create_options = QueueDeclareOptions {
        durable: true,
        ..QueueDeclareOptions::default()
    };

    if let Err(e) = msg_channel.queue_declare(queue_name, create_options, FieldTable::default()).await {
        return Err(e)
    }
    Ok(())
}

pub async fn consume_viewed_msg(msg_channel: Arc<Mutex<Channel>>) -> Result<(), lapin::Error> {
    let msg_channel_clone = msg_channel.clone();
    let channel_lock = msg_channel_clone.lock().await;

    // Set up consumer for the `viewed` queue
    let mut consumer: Consumer = channel_lock
                    .basic_consume(
                        "viewed",
                        "viewed_consumer",
                        BasicConsumeOptions::default(),
                        FieldTable::default()
                    )
                    .await?;

    println!("Started consuming messages from `viewed` queue");

    // Release the lock so other parts of the code can use the channel
    drop(channel_lock);

    // Process incoming messages in a loop
    while let Some(delivery) = consumer.next().await {
        if let Ok(delivery) = delivery {
            // Get the channel again for this op
            let lock = msg_channel_clone.lock().await;
            if let Err(e) = proccess_viewed_msg::<lapin::Error>(delivery).await {
                return Err(e)
            }
            drop(lock);
        }
    }

    Ok(())
}

async fn proccess_viewed_msg<E>(delivery: Delivery) -> Result<(), E> 
where E: StdError + Debug + 'static
{
    println!("Received a `viewed` message");

    // Parse the JSON msg
    match serde_json::from_slice::<Value>(&delivery.data) {
        Ok(parsed_msg) => {
            if let Some(in_video_path) = parsed_msg.get("video_path").and_then(|v| v.as_str()) {

                // Get history collection
                let db_client = connect_to_db().await
                            .map_err(|mongo_err| {
                                return Err::<E, String>(format!("Cannot connect to MongoDB: {:?}", mongo_err));
                            }).unwrap();

                let history_collection = get_history_collection(&db_client);

                let video_doc = History {
                    video_path: in_video_path.to_string()
                };
                
                // Record the "view" in the database
                history_collection.insert_one(&video_doc).await
                    .map_err(|mongo_err| {
                        return Err::<E, String>(format!("Cannot insert video_path to history collection: {:?}", mongo_err));
                    }).unwrap();

                println!("Acknowledging message was handled.");
                
                delivery.ack(BasicAckOptions::default()).await
                        .expect("Failed to acknowledge message");
            } else {
                eprintln!("Message missing `video_path` field");
                delivery.nack(BasicNackOptions::default()).await
                        .expect("Failed to NACK message");
            }
        },
        Err(e) => {
            eprintln!("Error parsing JSON: {}", e);
            delivery.nack(BasicNackOptions::default()).await
                        .expect("Failed to NACK message");
        }
    }

    Ok(())
} 

async fn connect_to_db() -> Result<mongodb::Client, HttpResponse> {
    let mut client_options = match ClientOptions::parse(crate::get_db_host()).await {
        Ok(c_options) => c_options,
        Err(_) => return Err(HttpResponse::InternalServerError().finish())
    };

    // Set the server_api field of the client_options to Stable API version 1
    let server_api = ServerApi::builder().version(ServerApiVersion::V1).build();
    client_options.server_api = Some(server_api);

    // Create new client and connect to the server
    let client = match mongodb::Client::with_options(client_options) {
        Ok(client) => client,
        Err(_) => return Err(HttpResponse::InternalServerError().finish())
    };

    Ok(client)
}

fn get_history_collection(db_client: &mongodb::Client) -> mongodb::Collection<History> {
    let db = db_client.database(crate::get_db_name());

    // This collection will include the videos that have been viewed
    db.collection::<History>("history")
}