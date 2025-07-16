use mongodb::{bson::{doc, oid::ObjectId}, Collection};
use serde::{Serialize, Deserialize};
use jsonwebtoken::{encode, EncodingKey, Header};
use actix_web::{post, web, HttpResponse, Responder};
use argon2::{password_hash::{PasswordHasher, SaltString}, Argon2, PasswordHash, PasswordVerifier};
use chrono;

#[derive(Clone)]
pub struct AppState {
    pub users: Collection<User>,
    pub jwt_key: EncodingKey,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    username: String,
    email: String,
    password_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>
}

#[derive(Debug, Deserialize)]
struct RegisterPayload {
    username: String,
    email: String,
    password: String
}

#[post("/register")]
async fn register(data: web::Data<AppState>, body: web::Json<RegisterPayload>) -> impl Responder {
    // Check if user already exists 
    if data.users.find_one(doc! {"email": &body.email}).await.unwrap().is_some() {
        return HttpResponse::Conflict().body("Email already registered");
    }

    let salt = SaltString::generate(&mut rand::thread_rng());
    let argon = Argon2::default();
    let hash = argon.hash_password(body.password.as_bytes(), &salt).unwrap().to_string();

    let user = User {
        id: None,
        username: body.username.clone(),
        email: body.email.clone(),
        password_hash: hash,
        role: Some("user".into())
    };

    let insert_res = data.users.insert_one(user).await;
    match insert_res {
        Ok(ins) => HttpResponse::Created().json(doc! {"id": ins.inserted_id}),
        Err(e) => {
            eprintln!("Failed to insert user: {e}");
            HttpResponse::InternalServerError().finish()
        }
    }
}

#[derive(Debug, Deserialize)]
struct LoginPayload { email: String, password: String }

#[derive(Debug, Serialize)]
struct Claims { sub: String, exp: usize, role: String }

#[post("/login")]
async fn login(data: web::Data<AppState>, body: web::Json<LoginPayload>) -> impl Responder {
    let Some(user) = data.users.find_one(doc! {"email": &body.email}).await.unwrap() else {
        return HttpResponse::Unauthorized().finish();
    };

    let parsed_hash = PasswordHash::new(&user.password_hash).unwrap();
    if Argon2::default().verify_password(body.password.as_bytes(), &parsed_hash).is_err() {
        return HttpResponse::Unauthorized().finish();
    }

    let exp = chrono::Utc::now().timestamp() + 60 * 60; // 1h expiry
    let claims = Claims{ sub: user.id.unwrap().to_hex(), exp: exp as usize, role: user.role.unwrap_or("user".into()) };
    let token = encode(&Header::default(), &claims, &data.jwt_key).unwrap();

    HttpResponse::Ok().json(doc! {"token": token})
}

