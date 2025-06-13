use actix_web::{web, get, HttpRequest, HttpResponse};
use futures::{TryStreamExt, StreamExt};
use mongodb::{ Client, bson::{doc, oid::ObjectId}};
use serde::{Serialize, Deserialize};
use lapin::{options::*, types::FieldTable, BasicProperties, Channel};

#[derive(Serialize, Deserialize)]
struct Video {
    _id: ObjectId,
    video_path: String
}

#[get("/health")]
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().body("OK")
}

#[get("/video")]
pub async fn get_video(req: HttpRequest, db_client: web::Data<Client>, rabbit_channel: web::Data<Channel>) -> HttpResponse {
    // Retrieve the DB and fetch the `videos` collection
    let db = db_client.database(crate::get_db_name());
    // This collection contains path to videos, so `String` arguments
    let videos_collection= db.collection::<Video>("videos");

    // The URI query part should contain the ID of the video
    let video_record = match get_video_record(&videos_collection, &req.uri().query()).await {
        Ok(record) => record,
        Err(_) => {
            eprintln!("Failed to retrieve video record from the database.");
            return HttpResponse::NotFound().finish()
        }
    };

    println!("Translated id {} to path {}", req.uri().query().unwrap(), video_record.video_path);

    let client = awc::Client::default();

    // This is the URL for the video storage microservice
    let target_url = format!("http://{}:{}/video?{}", crate::get_video_storage_host(), crate::get_video_storage_port(), video_record.video_path);

    // Create new request for the video storage
    let mut forward_request = client.get(target_url);

    // Copy the headers of the original request
    for (key, value) in req.headers().into_iter() {
        forward_request = forward_request.insert_header((key, value));
    }

    // Send the request to video storage and handle response
    match forward_request.send().await {
        Ok(res) => {
            let mut client_resp = HttpResponse::build(res.status());

            // Copy headers from forwarded message
            for header in res.headers() {
                client_resp.append_header(header);
            }

            match broadcast_viewed_message(rabbit_channel, &video_record.video_path, "viewed").await {
                Ok(_) => println!("Sent `viewed` message to 'viewed' exchange."),
                Err(_) => eprintln!("Failed to send `viewed` message.")
            };
            // Stream the response body
            client_resp.streaming(res.into_stream().map(|result| {
                result.map_err(|_| actix_web::error::ErrorInternalServerError("Error streaming video"))
            }))
        },
        Err(_) => HttpResponse::InternalServerError().body("Failed to connect to video service.")
    }

}

async fn get_video_record(collection: &mongodb::Collection<Video>, query_str: &Option<&str>) -> Result<Video, HttpResponse> {
    // Craft the video id from the URI query
    let video_id = match query_str {
        Some(s) => match ObjectId::parse_str(s) {
            Ok(id) => id,
            Err(_) => {
                eprintln!("Failed to parse video ID from query: {}", s);
                return Err(HttpResponse::InternalServerError().finish())
            }
        },
        None => {
            println!("No video ID provided in the query.");
            return Err(HttpResponse::BadRequest().finish())
        }
    };

    // the record returned should have the same structure as `Video`
    let video_record = match collection.find_one(doc!{"_id": video_id}).await {
        Ok(res) => match res {
            Some(val) => val,
            None => {
                eprintln!("No video found with ID: {}", video_id);
                return Err(HttpResponse::NotFound().finish())
            }
        },
        Err(_) => {
            eprintln!("Failed to fetch video record with ID: {}", video_id);
            return Err(HttpResponse::InternalServerError().finish())
        }
    };

    Ok(video_record)
}

async fn broadcast_viewed_message(rabbit_channel: web::Data<Channel>, video_path: &str, exchange_name: &str) -> Result<(), lapin::Error> {

    // Here we are broadcasting the `viewed` message
    // to the `viewed` exchange.

    let req = serde_json::json!({
        "video_path": video_path
    });

    println!("Publishing message on '{}' exchange ...", exchange_name);

    // We first need to check that the exchange exists
    rabbit_channel.exchange_declare(exchange_name, lapin::ExchangeKind::Fanout, ExchangeDeclareOptions {
        passive: true,
        durable: true,
        auto_delete: false,
        internal: false,
        nowait: false
    }, FieldTable::default()).await?; // This will throw if it doesn't exist

    rabbit_channel.basic_publish(
        exchange_name,
        "",
        BasicPublishOptions::default(),
        &serde_json::to_vec(&req).expect("Failed to serialize Value json")[..],
        BasicProperties::default()
    ).await?
     .await?;

    Ok(())
}