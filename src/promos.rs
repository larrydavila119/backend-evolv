use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use surrealdb::engine::remote::ws::Client;
use surrealdb::Surreal;
use std::collections::HashMap;
use log::{info, error};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct DiscountCode {
    pub code: String,
    pub discount_type: String,
    pub discount_value: f64,
    pub active: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateDiscountCode {
    pub code: Option<String>,
    pub discount_type: Option<String>,
    pub discount_value: Option<f64>,
    pub active: Option<bool>,
}

pub async fn get_discount_codes(
    database: &State<Surreal<Client>>,
) -> Result<Json<Vec<DiscountCode>>, Status> {
    match database.select::<Vec<DiscountCode>>("discount_codes").await {
        Ok(codes) => {
            log::info!("Códigos de descuento obtenidos exitosamente: {:?}", codes);
            Ok(Json(codes))
        }
        Err(err) => {
            log::error!("No se pudieron obtener los códigos de descuento: {:?}", err);
            Err(Status::InternalServerError)
        }
    }
}

pub async fn create_discount_code(
    database: &State<Surreal<Client>>,
    new_code: Json<DiscountCode>
) -> Result<Status, Status> {
    let code = new_code.into_inner();
    let query_check = format!(
        "SELECT * FROM discount_codes WHERE code = '{}';",
        code.code
    );
    let result_check: Result<_, surrealdb::Error> = database.query(&query_check).await;

    if let Ok(mut results) = result_check {
        if let Ok(Some(_)) = results.take::<Option<DiscountCode>>(0) {
            log::error!(
                "El código de promoción '{}' ya existe, use otro nombre por favor.",
                code.code
            );
            return Err(Status::Conflict);
        }
    }

    let query = format!(
        "CREATE discount_codes CONTENT {{
            code: '{}',
            discount_type: '{}',
            discount_value: {},
            active: {}
        }}",
        code.code,
        code.discount_type,
        code.discount_value,
        code.active
    );

    match database.query(&query).await {
        Ok(_) => {
            info!("Código de descuento creado exitosamente.");
            Ok(Status::Created)
        }
        Err(_) => {
            error!("Error al crear el código de descuento.");
            Err(Status::InternalServerError)
        }
    }
}

pub async fn update_discount_code(
    database: &State<Surreal<Client>>,
    discount_id: String,
    update_data: Json<UpdateDiscountCode>
) -> Result<Status, Status> {
    let mut updates = HashMap::new();
    let discount_value_str;
    let active_str;
    if let Some(code) = &update_data.code {
        updates.insert("code", code);
    }
    if let Some(discount_type) = &update_data.discount_type {
        updates.insert("discount_type", discount_type);
    }
    if let Some(discount_value) = &update_data.discount_value {
        discount_value_str = discount_value.to_string();
        updates.insert("discount_value", &discount_value_str);
    }
    if let Some(active) = &update_data.active {
        active_str = active.to_string();
        updates.insert("active", &active_str);
    }

    if updates.is_empty() {
        return Err(Status::BadRequest);
    }

    let update_statements: Vec<String> = updates
        .iter()
        .map(|(key, value)| format!("{} = '{}'", key, value))
        .collect();
    let query = format!(
        "UPDATE discount_codes:{} SET {}",
        discount_id,
        update_statements.join(", ")
    );

    match database.query(&query).await {
        Ok(_) => Ok(Status::Ok),
        Err(_) => Err(Status::InternalServerError),
    }
}

pub async fn delete_discount_code(
    database: &State<Surreal<Client>>,
    discount_id: String
) -> Result<Status, Status> {
    let query = format!("DELETE discount_codes WHERE code = '{}';", discount_id);

    match database.query(&query).await {
        Ok(_) => Ok(Status::Ok),
        Err(err) => {
            error!("Error al eliminar el código de descuento: {:?}", err);
            Err(Status::InternalServerError)
        }
    }
}

