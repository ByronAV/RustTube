use actix_web::{get, HttpRequest, HttpResponse};
use futures::{TryStreamExt, StreamExt};
use mongodb::{ bson::{doc, oid::ObjectId}, options::{ ClientOptions, ServerApi, ServerApiVersion }};
use serde::{Serialize, Deserialize};
use lapin::{options::*, BasicProperties, Channel, Connection, ConnectionProperties};

use crate::get_rabbit;

#[derive(Serialize, Deserialize)]
struct Video {
    _id: ObjectId,
    video_path: String
}

#[get("/video")]
async fn get_video(req: HttpRequest) -> HttpResponse {
    // Connect to the DB
    let db_client = match connect_to_db().await {
        Ok(db) => db,
        Err(_) => return HttpResponse::InternalServerError().finish()
    };

    // Retrieve the DB and fetch the `videos` collection
    let db = db_client.database(crate::get_db_name());
    // This collection contains path to videos, so `String` arguments
    let videos_collection= db.collection::<Video>("videos");

    // The URI query part should contain the ID of the video
    let video_record = match get_video_record(&videos_collection, &req.uri().query()).await {
        Ok(record) => record,
        Err(_) => return HttpResponse::NotFound().finish()
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

            match send_viewed_message(&video_record.video_path).await {
                Ok(_) => println!("Sent `viewed` message to history microservice queue."),
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

async fn get_video_record(collection: &mongodb::Collection<Video>, query_str: &Option<&str>) -> Result<Video, HttpResponse> {
    // Craft the video id from the URI query
    let video_id = match query_str {
        Some(s) => match ObjectId::parse_str(s) {
            Ok(id) => id,
            Err(_) => return Err(HttpResponse::InternalServerError().finish())
        },
        None => return Err(HttpResponse::BadRequest().finish())
    };

    // the record returned should have the same structure as `Video`
    let video_record = match collection.find_one(doc!{"_id": video_id}).await {
        Ok(res) => match res {
            Some(val) => val,
            None => return Err(HttpResponse::NotFound().finish())
        },
        Err(_) => return Err(HttpResponse::InternalServerError().finish())
    };

    Ok(video_record)
}

async fn connect_to_msg_channel() -> Result<Channel, lapin::Error> {

    // Connect to RabbitMQ server
    let addr = get_rabbit();
    let conn = Connection::connect(
        &addr,
        ConnectionProperties::default(),
    ).await?;

    println!("Connected to RabbitMQ from Backend Microservice.");

    // Create a channel
    conn.create_channel().await
}

async fn send_viewed_message(video_path: &str) -> Result<(), lapin::Error> {

    // We are sending a POST request to the history MS
    // in order to mark the video as viewed. We need
    // a json file that contains the video path and 
    // the content-type header

    let req = serde_json::json!({
        "video_path": video_path
    });

    println!("Publishing message on 'viewed' queue");

    let msg_channel = match connect_to_msg_channel().await {
        Ok(channel) => channel,
        Err(e) => return Err(e)
    };

    msg_channel.basic_publish(
        "",
        "viewed",
        BasicPublishOptions::default(),
        &serde_json::to_vec(&req).expect("Failed to serialize Value json")[..],
        BasicProperties::default()
    ).await?
     .await?;

    Ok(())
}