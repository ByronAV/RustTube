use std::{env, io, sync::OnceLock};
use actix_web::{get, HttpRequest, HttpResponse, App, HttpServer};
use awc::Client;
use futures::{TryStreamExt, StreamExt};

// We're retrieving the necessary env vars before beginning the service
static PORT: OnceLock<u16> = OnceLock::new();
static VIDEO_STORAGE_HOST : OnceLock<String> = OnceLock::new();
static VIDEO_STORAGE_PORT: OnceLock<u16> = OnceLock::new();

fn get_port() -> u16 {
    *PORT.get_or_init(|| {
        env::var("PORT")
            .ok()
            .and_then(|val| val.parse::<u16>().ok())
            .expect("Please specify the port number for the HTTP server with the environment variable PORT.")
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
#[get("/video")]
async fn get_video(req: HttpRequest) -> HttpResponse {
    let client = Client::default();

    // This is the URL for the video storage microservice
    let target_url = format!("http://{}:{}/video?file_example_MP4_640_3MG.mp4", get_video_storage_host(), get_video_storage_port());

    // Create new request for the video storage
    let mut forward_request = client.get(target_url);

    // Copy the headers of the original request
    for (key, value) in req.headers().iter() {
        forward_request = forward_request.insert_header((key.clone(), value.clone()));
    }

    // Send the request to video storage and handle response
    match forward_request.send().await {
        Ok(res) => {
            let mut client_resp = HttpResponse::build(res.status());

            // Copy headers from forwarded message
            for header in res.headers() {
                client_resp.append_header(header);
            }

            // Stream the response body
            client_resp.streaming(res.into_stream().map(|result| {
                result.map_err(|_| actix_web::error::ErrorInternalServerError("Error streaming video"))
            }))
        },
        Err(_) => HttpResponse::InternalServerError().body("Failed to connect to video service.")
    }

}

#[tokio::main(flavor="current_thread")]
async fn main() -> io::Result<()> {
    println!("Forwarding video requests to {}:{}", get_video_storage_host(), get_video_storage_port());

    HttpServer::new(|| {
        println!("Microservice online.");
        App::new()
            .service(get_video)
    })
    .bind(format!("0.0.0.0:{}", get_port()))?
    .run()
    .await
}

