use actix_web::{web, post, HttpResponse};
use mongodb::{ bson::doc, options::{ ClientOptions, ServerApi, ServerApiVersion }};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct History {
    video_path: String
}

#[post("/viewed")]
async fn post_viewed(req: web::Json<History>) -> HttpResponse {
    // Connect to the DB
    let db_client = match connect_to_db().await {
        Ok(db) => db,
        Err(_) => return HttpResponse::InternalServerError().finish()
    };

    let db = db_client.database(crate::get_db_name());

    // This collection will include the videos that have been viewed
    let history_collection = db.collection::<History>("history");

    let req: History  = req.into_inner();
    let video_doc = History {
        video_path: req.video_path
    };

    let insert_res = match history_collection.insert_one(&video_doc).await {
        Ok(val) => val,
        Err(_)  => return HttpResponse::InternalServerError().finish()
    };

    println!("Added video {} with id: {} to history", video_doc.video_path, insert_res.inserted_id);

    return HttpResponse::Ok().finish()
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