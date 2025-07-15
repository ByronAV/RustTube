use actix_web::{web, post, get, HttpRequest, HttpResponse, Error};
use actix_multipart::Multipart;
use futures::{TryStreamExt, StreamExt};
use mongodb::{ Client as MongoClient, bson::{doc, oid::ObjectId, DateTime as BsonDateTime}};
use serde::{Serialize, Deserialize};
use lapin::{options::*, types::FieldTable, BasicProperties, Channel};
use uuid::Uuid;
use chrono::Utc;
use awc::Client as AwcClient;
use reqwest::Client as ReqwestClient;
use reqwest::multipart;

use crate::get_db_name;

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Video {
    #[serde(skip_serializing_if = "Option::is_none")]
    _id: Option<ObjectId>,
    video_path: String,
    duration: Option<i64>,
    created_at: Option<BsonDateTime>,
    user_id: Option<String>
}

#[derive(Deserialize)]
struct VideoRequest {
    id: String,
}

#[get("/health")]
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().body("OK")
}

#[get("/video")]
pub async fn get_video(req: HttpRequest, query: web::Query<VideoRequest>, db_client: web::Data<MongoClient>, rabbit_channel: web::Data<Channel>) -> HttpResponse {
    // Retrieve the DB and fetch the `videos` collection
    let db = db_client.database(crate::get_db_name());
    // This collection contains path to videos, so `String` arguments
    let videos_collection= db.collection::<Video>("videos");

    // The URI query part should contain the ID of the video
    let video_record = match get_video_record(&videos_collection, &query.id).await {
        Ok(record) => record,
        Err(_) => {
            eprintln!("Failed to retrieve video record from the database");
            return HttpResponse::NotFound().finish()
        }
    };

    println!("Translated id {} to path {}", query.id, video_record.video_path);

    let client = AwcClient::default();

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

            tokio::spawn(async move {
                if let Err(e) = broadcast_viewed_message(rabbit_channel, &video_record.video_path, "viewed").await {
                    eprintln!("Failed to send `viewed` message: {:?}", e);
                }
            });

            // Stream the response body
            client_resp.streaming(res.into_stream().map(|result| {
                result.map_err(|_| actix_web::error::ErrorInternalServerError("Error streaming video"))
            }))
        },
        Err(e) => {
            eprintln!("Failed to connect to video service: {:?}", e);
            HttpResponse::InternalServerError().body("Failed to connect to video service.")
        }
    }

}

#[post("/upload")]
pub async fn upload_video(mut payload: Multipart, db_client: web::Data<MongoClient>) -> Result<HttpResponse, Error> {
    let mut file_bytes = Vec::new();
    let mut filename = None;
    let mut duration = None;
    let mut created_at = None;
    let mut user_id = None;

    // Get fields out of request
    while let Some(field_res) = payload.next().await {
        let mut field = field_res?;
        let content_disposition = field.content_disposition();

        let name = content_disposition
            .get_name()
            .map(|s| s.to_string())
            .unwrap_or_default();
            
        if name == "file" {
            while let Some(chunk) = field.next().await {
                let data = chunk?;
                file_bytes.extend_from_slice(&data);
            }
        } else {
            let mut text = Vec::new();
            while let Some(chunk) = field.next().await {
                text.extend_from_slice(&chunk?);
            }
            let value = String::from_utf8_lossy(&text).to_string();
            match name.as_str() {
                "filename" => filename = Some(value),
                "duration" => duration = Some(value),
                "created_at" => created_at = Some(value),
                "user_id" => user_id = Some(value),
                _ => {}
            }
        }
    }

    // Pass the actual file to storage service to be moved to Azure blob
    println!("Saving video {} to `videos` collection", &filename.as_ref().unwrap());

    let video_path = filename.unwrap_or_else(|| Uuid::new_v4().to_string() + ".mp4");

    let form = multipart::Form::new()
        .text("filename", video_path.clone())
        .part(
            "file",
            multipart::Part::bytes(file_bytes.clone())
                .file_name(video_path.clone())
                .mime_str("application/octet-stream")
                .map_err(|e| {
                    eprintln!("Reqwest error: {:?}", e);
                    actix_web::error::ErrorInternalServerError("Failed to contact storage service")
                })?
        );

    // Send video to storage service
    let client = ReqwestClient::default();
    let res = client
        .post(format!("http://{}:{}/store", crate::get_video_storage_host(), crate::get_video_storage_port()))
        .multipart(form)
        .send()
        .await;

    match res {
        Ok(resp) if resp.status().is_success() => {
            let duration = duration
                .and_then(|d| d.parse::<i64>().ok())
                .unwrap_or(0);

            let created_at = created_at
                .and_then(|dt| chrono::DateTime::parse_from_rfc3339(&dt).ok())
                .map(|dt| BsonDateTime::from_system_time(dt.with_timezone(&Utc).into()))
                .unwrap_or_else(|| BsonDateTime::from_system_time(std::time::SystemTime::now()));

            let user_id = user_id.unwrap_or_else(|| "anonymous".to_string());

            let db_client_clone = db_client.clone();
            let video_path_clone = video_path.clone();
            let user_id_clone = user_id.clone();

            // Add to DB asynchronously
            tokio::spawn(async move {
                let new_video = Video {
                    _id: None,
                    video_path: video_path_clone,
                    duration: Some(duration),
                    created_at: Some(created_at),
                    user_id: Some(user_id_clone)
                };

                let collection = db_client_clone
                .database(get_db_name())
                .collection("videos");
                
                if let Err(e) = collection.insert_one(new_video).await {
                    eprintln!("Failed to insert video record: {:?}", e);
                } else {
                    println!("Successfully stored video to `videos` collection.");
                }
            });
            Ok(HttpResponse::Ok().json(doc!{"video_path": video_path}))
        }
        Ok(resp) => {
            eprintln!("Storage returned error status: {:?}", resp.status());
            Ok(HttpResponse::InternalServerError().body("Storage failed"))
        }
        Err(err) => {
            eprintln!("AWC client error: {:?}", err);
            Ok(HttpResponse::InternalServerError().body("Failed to contact storage service"))
        }
    }
}
async fn get_video_record(collection: &mongodb::Collection<Video>, query_str: &str) -> Result<Video, HttpResponse> {
    // Craft the video id from the URI query

    let video_id = match ObjectId::parse_str(query_str) {
        Ok(id) => id,
        Err(e) => {
            eprintln!("Failed to parse video ID from query: {}. Error: {:?}", query_str, e);
            return Err(HttpResponse::InternalServerError().finish())
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
        Err(e) => {
            eprintln!("Failed to fetch video record with ID: {} Error: {:?}", video_id, e);
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