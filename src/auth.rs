use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome, Request};
use serde::{Deserialize, Serialize};
use log::{error, info, warn}; // Importamos para agregar logs

// Estructura que representa los claims del JWT
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,   // Usuario
    pub role: String,  // Rol
    pub exp: usize,    // Expiración
}

// Estructura para representar un usuario autenticado
pub struct AuthenticatedUser {
    pub username: String,
    pub role: String,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthenticatedUser {
    type Error = &'static str; // Tipo de error simplificado

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        // Obtener el encabezado de autorización
        let auth_header = request.headers().get_one("Authorization");
        if let Some(header) = auth_header {
            info!("Authorization header found: {}", header);
            if let Some(token) = header.strip_prefix("Bearer ") {
                info!("Extracted token: {}", token);

                // Decodificar y validar el JWT
                match validate_jwt(token) {
                    Ok(claims) => {
                        info!(
                            "Token valid. Username: {}, Role: {}, Exp: {}",
                            claims.sub, claims.role, claims.exp
                        );
                        Outcome::Success(AuthenticatedUser {
                            username: claims.sub,
                            role: claims.role,
                        })
                    }
                    Err(err) => {
                        error!("JWT validation failed: {:?}", err);
                        Outcome::Error((Status::Unauthorized, "Invalid or expired token"))
                    }
                }
            } else {
                warn!("Authorization header format invalid: {}", header);
                Outcome::Error((Status::Unauthorized, "Invalid Authorization header format"))
            }
        } else {
            warn!("Authorization header missing");
            Outcome::Error((Status::Unauthorized, "Authorization header missing"))
        }
    }
}

// Función para validar el JWT
fn validate_jwt(token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let secret = "secret-key-for-jwt"; // Esta clave debe coincidir con la usada en FastAPI
    let validation = Validation::new(Algorithm::HS256);
    let decoding_key = DecodingKey::from_secret(secret.as_ref());
    decode::<Claims>(token, &decoding_key, &validation).map(|data| data.claims)
}

impl AuthenticatedUser {
    pub fn has_role(&self, role: &str) -> bool {
        self.role == role
    }

    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

