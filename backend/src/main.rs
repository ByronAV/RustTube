use axum::{Router, routing::get, body::Body, response::Response, http::{StatusCode, header}};
use std::net::SocketAddr;
use tokio::{net::TcpListener, fs::File};
use tokio_util::io::ReaderStream;

const PORT: u16 = 3000;

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
    println!("Microservice listening on port {PORT}, point your browser at http://localhost:{PORT}/video");

    let video_router: Router<_> = Router::new().route("/video", get( ||get_video("..videos/file_example_MP4_640_3MG.mp4")));

    let addr = SocketAddr::from(([127,0,0,1], PORT));

    let tcp = TcpListener::bind(&addr)
                        .await
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
                        .unwrap();

    axum::serve(tcp, video_router)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
                .unwrap();
}

