use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use surrealdb::engine::remote::ws::Client;
use surrealdb::sql::Thing;
use surrealdb::Surreal;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use log::{info, error};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdatePayment {
    pub months: Option<Vec<HashMap<String, bool>>>, // Actualización de meses
    pub year: Option<u16>,                    // Actualización de año
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimplifiedPayment {
    pub client_name: Option<String>, // Nombre del cliente
    pub id: Thing,           // ID de la tabla de pagos del cliente
    pub months: Vec<HashMap<String, bool>>,         // Mes del pago
    pub schedule: Option<String>,
    pub year: u16,           // Año del pago
    //pub status: bool,        // Estado del pago
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SimplifiedPaymentAsString {
    pub client_name: String, // Nombre del cliente
    pub id: String,          // ID convertido a String
    pub months: Vec<HashMap<String, bool>>,         // Mes del pago
    pub schedule: Option<String>,
    pub year: u16,           // Año del pago
    //pub status: bool,        // Estado del pago
}

impl From<SimplifiedPayment> for SimplifiedPaymentAsString {
    fn from(payment: SimplifiedPayment) -> Self {
        SimplifiedPaymentAsString {
            client_name: payment.client_name.unwrap_or_else(|| "Unknown".to_string()), // Asignar valor por defecto
            id: payment.id.to_string(),
            months: payment.months,
            schedule: Some(payment.schedule.unwrap_or_else(|| "No Schedule".to_string())), // Manejo de None
            year: payment.year,
        }
    }
}

pub async fn get_history_payments(
    database: &State<Surreal<Client>>,
) -> Result<Json<Vec<SimplifiedPaymentAsString>>, Status> {
    match database
        .query(
            "SELECT client_id.fullname ?? 'Unknown' AS client_name,id, months, schedule.name ?? 'No Schedule' AS schedule, year FROM payments FETCH client_id, schedule;",
        )
        .await
    {
        Ok(mut results) => {
            let payments: Vec<SimplifiedPayment> = match results.take(0) {
                Ok(data) => data,
                Err(err) => {
                    log::error!("Error al extraer resultados como SimplifiedPayment: {:?}", err);
                    return Err(Status::InternalServerError);
                }
            };

            // Convertir a SimplifiedPaymentAsString
            let payments_as_string: Vec<SimplifiedPaymentAsString> = payments
                .into_iter()
                .map(SimplifiedPaymentAsString::from)
                .collect();

            log::info!("Pagos mensuales obtenidos: {:?}", payments_as_string);
            Ok(Json(payments_as_string))
        }
        Err(err) => {
            log::error!("Error al consultar la base de datos: {:?}", err);
            Err(Status::InternalServerError)
        }
    }
}

pub async fn update_payment(
    database: &State<Surreal<Client>>,
    payment_id: String,
    updated_data: Json<Value>, // Usamos `Value` para manejar datos dinámicos
) -> Result<Status, Status> {
    let updated_data = updated_data.into_inner();

    // Validar que se proporcionen campos para actualizar
    if updated_data.as_object().is_none() || updated_data.as_object().unwrap().is_empty() {
        log::error!("No se enviaron campos válidos para actualizar.");
        return Err(Status::BadRequest);
    }

    // Serializar los datos proporcionados como JSON
    let updated_content = match serde_json::to_string(&updated_data) {
        Ok(json) => json,
        Err(err) => {
            log::error!("Error al serializar los datos de actualización: {:?}", err);
            return Err(Status::BadRequest);
        }
    };

    // Construir la consulta SQL dinámica
    let query = format!(
        "UPDATE {} MERGE {};",
        payment_id, updated_content
    );

    log::info!("Ejecutando query de actualización dinámica: {}", query);

    // Ejecutar la consulta
    match database.query(&query).await {
        Ok(_) => {
            log::info!("Pago actualizado correctamente: {}", payment_id);
            Ok(Status::Ok)
        }
        Err(err) => {
            log::error!("Error al actualizar el pago: {:?}", err);
            Err(Status::InternalServerError)
        }
    }
}

