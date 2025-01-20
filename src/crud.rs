use bcrypt::{hash, DEFAULT_COST};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use surrealdb::engine::remote::ws::Client;
use surrealdb::Surreal; // Asegúrate de que estás importando el cliente correcto
use log::{info, error}; 
use serde::Deserializer;
use surrealdb::sql::Thing;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
    pub id: Option<Thing>,         // Convertido a String
    pub fullname: String,
    pub roles: String,
    pub username: String,
    pub password: String,
    pub branch: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserAsRecord {
    pub id: Thing,          // Se obtiene como Thing desde la base de datos
    pub fullname: String,
    pub roles: String,
    pub username: String,
    pub password: String,
    pub branch: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserForQuery {
    pub id: Thing,
    pub fullname: String,
    pub roles: String,
    pub username: String,
    pub branch: String,
}

#[derive(Serialize, Debug)]
pub struct UserAsString {
    pub id: String,
    pub fullname: String,
    pub roles: String,
    pub username: String,
    pub branch: String,
}

impl From<UserAsRecord> for UserAsString {
    fn from(record: UserAsRecord) -> Self {
        UserAsString {
            id: record.id.id.to_string(),
            fullname: record.fullname,
            roles: record.roles,
            username: record.username,
            branch: record.branch,
        }
    }
}

impl From<User> for UserAsString {
    fn from(user: User) -> Self {
        UserAsString {
            id: user.id.map_or_else(|| "Sin ID".to_string(), |id| id.to_string()), // Convierte `Option<Thing>` a `String`
            fullname: user.fullname,
            roles: user.roles,
            username: user.username,
            branch: user.branch,
        }
    }
}

fn thing_to_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = surrealdb::sql::Value::deserialize(deserializer)?;
    match value {
        surrealdb::sql::Value::Thing(thing) => Ok(thing.to_string()),
        surrealdb::sql::Value::Strand(s) => Ok(s.to_string()),
        _ => Err(serde::de::Error::custom("Formato de ID inválido")),
    }
}

#[derive(Serialize, Deserialize)]
pub struct UpdateUser {
    fullname: Option<String>,
    roles: Option<String>,
    username: Option<String>,
    password: Option<String>,
    branch: Option<String>,
}

pub async fn create_user(database: &State<Surreal<Client>>, new_user: Json<User>) -> Result<Status, Status> {
    let mut user = new_user.into_inner();

    // Verificar si el usuario ya existe
    let query_check = format!(
        "SELECT * FROM users WHERE username = '{}';",
        user.username
    );
    let result_check = database.query(&query_check).await;

    if let Ok(mut results) = result_check {
        if let Ok(Some(_)) = results.take::<Option<User>>(0) {
            error!("El nombre de usuario '{}' ya existe.", user.username);
            return Err(Status::Conflict);
        }
    }

    // Hashear la contraseña
    let hashed_password = hash(&user.password, DEFAULT_COST).expect("Failed to hash password");
    user.password = hashed_password;

    // Crear el usuario con `RETURN *`
    let query = format!(
        "CREATE users CONTENT {{
            fullname: '{}',
            roles: '{}',
            username: '{}',
            password: '{}',
            branch: '{}'
        }} RETURN *;",
        user.fullname, user.roles, user.username, user.password, user.branch
    );

    info!("Ejecutando el query: {}", query);

    let Ok(mut results) = database.query(&query).await else {
        error!("Petición a la base de datos falló.");
        return Err(Status::InternalServerError);
    };

    // Log de los resultados completos
    debug!("Resultados del query: {:?}", results);

    // Procesar los resultados del query
    match results.take::<Vec<User>>(0) {
        Ok(vec) if !vec.is_empty() => {
            let user = vec[0].clone();
            let user_id = user
                .id
                .as_ref()
                .map_or_else(|| "Sin ID".to_string(), |id| id.to_string());
            info!("Usuario creado correctamente: {:?}", vec[0]);
            Ok(Status::Created)
        }
        Ok(vec) if vec.is_empty() => {
            error!("El resultado del query está vacío.");
            Err(Status::InternalServerError)
        }
        Ok(_) => {
            error!("El resultado del query no coincide con el formato esperado");
            Err(Status::InternalServerError)
        }
        Err(e) => {
            error!("Error al procesar el resultado del query: {:?}", e);
            Err(Status::InternalServerError)
        }
    }
}

pub async fn update_user(
    database: &State<Surreal<Client>>,
    user_id: String,
    update_data: Json<UpdateUser>,
) -> Result<Status, Status> {
    let mut updates = HashMap::new();
    
    if let Some(fullname) = &update_data.fullname {
        updates.insert("fullname", fullname.clone());
    }
    if let Some(username) = &update_data.username {
        updates.insert("username", username.clone());
    }
    if let Some(branch) = &update_data.branch {
        updates.insert("branch", branch.clone());
    }
    if let Some(password) = &update_data.password {
        // Hashear la contraseña
        let hashed_password = hash(password, DEFAULT_COST)
            .map_err(|_| Status::InternalServerError)?; // Manejar error al hashear
        updates.insert("password", hashed_password); // Insertar contraseña hasheada
    }

    if updates.is_empty() {
        return Err(Status::BadRequest); // Si no hay campos a actualizar
    }

    // Construir la consulta SQL de actualización
    let update_statements: Vec<String> = updates
        .iter()
        .map(|(key, value)| format!("{} = '{}'", key, value))
        .collect();

    let query = format!(
        "UPDATE users:{} SET {};",
        user_id,
        update_statements.join(", ")
    );

    info!("Ejecutando query de actualización: {}", query);

    // Ejecutar la consulta en la base de datos
    let result = database.query(&query).await;

    match result {
        Ok(_) => Ok(Status::Ok),
        Err(err) => {
            error!("Error al actualizar el usuario: {:?}", err);
            Err(Status::InternalServerError)
        }
    }
}


pub async fn get_users(database: &State<Surreal<Client>>) -> Result<Json<Vec<UserAsString>>, Status> {
    let query = "SELECT id, fullname, roles, username, branch FROM users;";
    let result = database.query(query).await;

    match result {
        Ok(mut results) => {
            let raw_records: Vec<UserForQuery> = match results.take(0) {
                Ok(data) => data,
                Err(err) => {
                    error!("Error al deserializar usuarios: {:?}", err);
                    return Err(Status::InternalServerError);
                }
            };

            // Transformar UserForQuery a UserAsString
            let users: Vec<UserAsString> = raw_records
                .into_iter()
                .map(|record| UserAsString {
                    id: record.id.id.to_string(),
                    fullname: record.fullname,
                    roles: record.roles,
                    username: record.username,
                    branch: record.branch,
                })
                .collect();

            info!("Usuarios convertidos exitosamente: {:?}", users);
            Ok(Json(users))
        }
        Err(err) => {
            error!("Error al consultar la base de datos: {:?}", err);
            Err(Status::InternalServerError)
        }
    }
}

pub async fn delete_user(
    database: &State<Surreal<Client>>,
    user_id: String,
) -> Result<Status, Status> {
    let query = format!("DELETE users:{};", user_id);

    match database.query(&query).await {
        Ok(_) => Ok(Status::Ok),
        Err(err) => {
            error!("Error al eliminar el producto: {:?}", err);
            Err(Status::InternalServerError)
        }
    }
}

