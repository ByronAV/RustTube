//extern crate actix_web;

use azure_storage::prelude::*;
use azure_storage_blobs::prelude::*;
use serde::Deserialize;
use actix_web::{get, http::{header::ContentLength, StatusCode}, web, HttpRequest, HttpResponse};

#[derive(Debug, Deserialize)]
pub struct QueryParams {
    path: String
}

#[get("/video")]
pub async fn get_video(req: HttpRequest) -> HttpResponse {
    let video_path: &str = &web::Query::<QueryParams>::from_query(req.query_string()).unwrap().path;
    // In case this is empty, return response (why no semicolon here ??)
    if video_path.is_empty() {
        eprintln!("The video path is empty");
        return HttpResponse::new(StatusCode::NOT_FOUND);
    }
    println!("Streaming video from path {video_path}");

    let blob_service = create_blob_service(crate::get_storage_account_name(), crate::get_storage_access_key());

    let container_name = "videos";
    let container_client = blob_service.container_client(container_name);
    let blob_client = container_client.blob_client(video_path);

    match blob_client.get_content().await {
        Ok(content) => {
            let content_length = content.len() as usize;
            let content_type = "video/mp4";

            // Return the content as HTTP response
            return HttpResponse::Ok()
                        .insert_header(ContentLength(content_length))
                        .content_type(content_type)
                        .body(content)
        }
        Err(e) => {
            eprintln!("Error fetching blob content: {}", e);
            HttpResponse::InternalServerError().finish()
        }
    }
}

pub fn create_blob_service(storage_account_name: &'static str, storage_access_key: &'static str) -> BlobServiceClient{
    let shared_key_credentials = StorageCredentials::access_key(storage_account_name, storage_access_key);
    let blob_service = BlobServiceClient::new(storage_account_name, shared_key_credentials);

    blob_service
}