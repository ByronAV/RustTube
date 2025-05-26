use actix_web::{get, HttpResponse};
use mongodb::{ bson::doc/*, options::{ ClientOptions, ServerApi, ServerApiVersion }*/};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use lapin::{message::Delivery, options::*, types::FieldTable, Channel, Connection, ConnectionProperties, Consumer, ExchangeKind, Queue};
use futures_lite::stream::StreamExt;
use std::{sync::Arc, error::Error as StdError, fmt::Debug};
use tokio::sync::Mutex;

use crate::get_rabbit;

#[derive(Serialize, Deserialize)]
struct History {
    video_path: String
}

#[get("/health")]
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().body("OK")
}

pub async fn connect_to_msg_channel() -> Result<Channel, lapin::Error> {

    println!("Connecting to RabbitMQ from Recommendations Microservice at {} ...", get_rabbit());

    // Connect to RabbitMQ server
    let addr = get_rabbit();
    let conn = Connection::connect(
        &addr,
        ConnectionProperties::default(),
    ).await?;

    println!("Connected to RabbitMQ from Recommendations Microservice.");

    // Create a channel
    conn.create_channel().await
}

pub async fn assert_exchange(msg_channel: &Channel, exchange_name: &str) -> Result<Queue, Box<dyn StdError>> {
    // Checking and creating exchange
    let create_options = ExchangeDeclareOptions {
        passive: false,
        durable: true,
        auto_delete: false,
        internal: false,
        nowait: false
    };

    println!("Creating exchange '{}' ...", exchange_name);
    msg_channel.exchange_declare(exchange_name, ExchangeKind::Fanout, create_options, FieldTable::default()).await?;
    println!("Exchange '{}' created successfully", exchange_name);

    let queue = match create_and_bind_queue(&msg_channel, "viewed").await {
        Ok(q) => q,
        Err(e) => {
            eprintln!("Error creating and binding queue: {}", e);
            return Err(e)
        }
    };

    Ok(queue)
}

pub async fn create_and_bind_queue(msg_channel: &Channel, exchange_name: &str) -> Result<Queue, Box<dyn StdError>> {

    println!("Creating queue ...");

    let queue_options = QueueDeclareOptions {
        passive: false,
        durable: false, // Typically false for anonymous queues
        exclusive: true,
        auto_delete: true,
        nowait: true
    };

    let queue = msg_channel.queue_declare("", queue_options, FieldTable::default()).await?;

    println!("Queue created successfully");

    // Bind the queue to the exchange
    println!("Binding queue to exchange '{}' ...", exchange_name);

    msg_channel.queue_bind(queue.name().as_str(), exchange_name, "", QueueBindOptions::default(), FieldTable::default()).await?;

    println!("Queue binding successful");

    Ok(queue)
}

pub async fn consume_viewed_msg(msg_channel: Arc<Mutex<Channel>>, queue_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let msg_channel_clone = msg_channel.clone();
    let channel_lock = msg_channel_clone.lock().await;

    // Set up consumer for the `viewed` queue
    let mut consumer: Consumer = channel_lock
                    .basic_consume(
                        queue_name,
                        "viewed_consumer",
                        BasicConsumeOptions::default(),
                        FieldTable::default()
                    )
                    .await?;

    println!("Started consuming messages from queue");

    // Release the lock so other parts of the code can use the channel
    drop(channel_lock);

    // Process incoming messages in a loop
    while let Some(delivery) = consumer.next().await {
        if let Ok(delivery) = delivery {
            // Get the channel again for this op
            let lock = msg_channel_clone.lock().await;
            if let Err(e) = proccess_viewed_msg::<lapin::Error>(delivery).await {
                return {
                    eprintln!("Error processing viewed message: {}", e);
                    Err(Box::new(e))
                }
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

                // For now, just printing the message that we received
                println!("{}", in_video_path);
                
                delivery.ack(BasicAckOptions::default()).await
                        .expect("Failed to acknowledge message");

                println!("Acknowledging message was handled.");
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