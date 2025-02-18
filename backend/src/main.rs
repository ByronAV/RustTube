use axum::{Router, routing::get, body::Body, response::Response, http::{StatusCode, header}};
use std::{net::SocketAddr, env};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

async fn get_video(path: &str) -> Result<Response, StatusCode> {
    // Open the file
    let file = File::open(path)
            .await
            .map_err(|_| StatusCode::NOT_FOUND)?;

    // Get file metadata
    let metadata = file.metadata()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let content_type = "video/mp4";

    // Create stream
    let stream = ReaderStream::new(file);
    let body = Body::from_stream(stream);

    let res = Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, content_type)
                        .header(header::CONTENT_LENGTH, metadata.len())
                        .body(body)
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(res)

}

#[tokio::main(flavor="current_thread")]
async fn main() {
    let port = "PORT";

    // Check for env for port number
    let port: u16 = match env::var(port) {
        Ok(val) => val.parse::<u16>().unwrap(),
        Err(e) => panic!("Error {}: {}", port, e)
    };
    println!("Microservice listening on port {port}, point your browser at http://localhost:{port}/video");

    let video_router: Router<_> = Router::new().route("/video", get( || async { get_video("./videos/file_example_MP4_640_3MG.mp4").await }));

    let addr = SocketAddr::from(([127,0,0,1], port));

    axum_server::bind(addr)
        .serve(video_router.into_make_service())
        .await
        .unwrap();
}

