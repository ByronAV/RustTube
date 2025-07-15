//extern crate actix_web;

use azure_storage::prelude::*;
use azure_storage_blobs::prelude::*;
use actix_web::{get, post, http::header, HttpRequest, HttpResponse, Error};
use actix_multipart::Multipart;
use futures::StreamExt;

#[get("/health")]
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().body("OK")
}

#[get("/video")]
pub async fn get_video(req: HttpRequest) -> Result<HttpResponse, Error> {
    let video_path = match req.uri().query() {
        Some(query) => query,
        None => {
            eprintln!("The video path was not found");
            return Ok(HttpResponse::NotFound().finish());
        }
    };

    println!("Streaming video from path {video_path}");

    let blob_client = create_blob_client(
        crate::get_storage_account_name(),
        crate::get_storage_access_key(),
        "videos",
        video_path);

    match blob_client.get_content().await {
        Ok(content) => {
            let content_length = content.len() as usize;
            let content_type = "video/mp4";

            // Return the content as HTTP response
            return Ok(HttpResponse::Ok()
                        .insert_header(header::ContentLength(content_length))
                        .content_type(content_type)
                        .body(content))
        }
        Err(e) => {
            eprintln!("Error fetching blob content: {}", e);
            return Ok(HttpResponse::InternalServerError().finish())
        }
    }
}

#[post("/store")]
pub async fn store_video(mut payload: Multipart) -> Result<HttpResponse, Error> {
    let mut file_bytes = Vec::new();
    let mut filename = "video.mp4".to_string();

    while let Some(field) = payload.next().await {
        let mut field = field?;
        let content_disposition = field.content_disposition();
        if let Some(name) = content_disposition.get_name().map(|s| s.to_string()){
            if name == "file" {
                while let Some(chunk) = field.next().await {
                    file_bytes.extend_from_slice(&chunk?);
                }
            } else if name == "filename" {
                let mut text = Vec::new();
                while let Some(chunk) = field.next().await {
                    text.extend_from_slice(&chunk?);
                }
                filename = String::from_utf8_lossy(&text).to_string();
            }
        }
    }

    let blob_client = create_blob_client(crate::get_storage_account_name(),
        crate::get_storage_access_key(),
        "videos",
        &filename);

    let _res = blob_client
        .put_block_blob(file_bytes)
        .content_type("video/mp4")
        .await
        .map_err(|e| {
            eprintln!("Azure upload failed: {:?}", e);
            actix_web::error::ErrorInternalServerError("Upload Failed")
        })?;
    println!("Successfully uploaded video to blob");
    Ok(HttpResponse::Ok().body("Uploaded to Azure"))
}

pub fn create_blob_client(storage_account_name: &'static str, storage_access_key: &'static str, container_name: &str, filename: &str) -> BlobClient {
    let shared_key_credentials = StorageCredentials::access_key(storage_account_name, storage_access_key);
    let blob_service = BlobServiceClient::new(storage_account_name, shared_key_credentials);
    let container_client = blob_service.container_client(container_name);
    let blob_client = container_client.blob_client(filename);

    blob_client
}