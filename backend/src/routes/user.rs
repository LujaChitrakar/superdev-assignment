use actix_web::{HttpResponse, Result, web};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct SignUpRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct SignInRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: String,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
}

#[derive(Serialize)]
pub struct SignupResponse {
    message: String,
}

#[actix_web::post("/signup")]
pub async fn sign_up(req: web::Json<SignUpRequest>) -> Result<HttpResponse> {
    let password_hash = argon2::hash_encoded(
        req.password.as_bytes(),
        &Uuid::new_v4().as_bytes(),
        &Config::default(),
    )
    .unwrap();

    let user_id = store
        .create_user(&req.email, &password_hash)
        .await
        .map_err(|_| actix_web::error::ErrorInternalServerError("DB insert failed"))?;

    Ok(HttpResponse::Created().json(SignupResponse {
        message: format!("User {} created successfully", req.email),
    }))
}

#[actix_web::post("/signin")]
pub async fn sign_in(req: web::Json<SignInRequest>) -> Result<HttpResponse> {
    if let Some(user) = store.find_user_by_email(&req.email).await.unwrap() {
        if argon2::verify_encoded(&user.password_hash, req.password.as_bytes()).unwrap_or(false) {
            let claims = Claims {
                sub: user.id.to_string(),
                exp: (Utc::now().timestamp() + 3600) as usize,
            };
            let token = encode(
                &Header::default(),
                &claims,
                &EncodingKey::from_secret("secret".as_ref()),
            )
            .unwrap();

            return Ok(HttpResponse::Ok().json(AuthResponse { token }));
        }
    }

    Err(actix_web::error::ErrorUnauthorized("Invalid credentials"))
}

#[actix_web::get("/user/{id}")]
pub async fn get_user(path: web::Path<u32>) -> Result<HttpResponse> {
    let user_id = path.into_inner();

    if let Some(user) = store.find_user_by_id(user_id).await.unwrap() {
        let response = UserResponse {
            id: user.id,
            email: user.email,
            created_at: user.created_at,
        };
        Ok(HttpResponse::Ok().json(response))
    } else {
        Err(actix_web::error::ErrorNotFound("User not found"))
    }
}
