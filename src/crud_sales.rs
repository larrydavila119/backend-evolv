use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use serde::Deserialize;
use serde::Serialize;
use serde::Deserializer;
use serde::de::{self, Visitor, SeqAccess};
use std::collections::HashMap;
use surrealdb::engine::remote::ws::Client;
use surrealdb::Surreal; 
use log::{info, error}; 
use surrealdb::sql::Thing;
use std::collections::HashSet;
use chrono::{Utc, FixedOffset, DateTime};
use serde_json::Value as JsonValue;
use surrealdb::sql::{Value as SurrealValue, Object};
use crate::crud_inventory::get_product_by_id;
use std::fmt;
use chrono::NaiveDate;

#[derive(Serialize, Deserialize, Debug)]
pub struct ProductWithQuantity {
    pub id: String,
    pub qnt: u32,   
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EnrichedProduct {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SimplifiedSales {
    pub cashier: Option<String>,
    pub change: Option<f64>,
    pub currency: Option<String>,
    pub customer: Option<String>,
    pub date: Option<String>,
    pub id: Thing, // Mantén `Thing` si prefieres usar el tipo original
    pub payment_ref: Option<String>,
    pub products_names: Option<Vec<String>>, // Solo los nombres de los productos
    pub promocode: Option<String>,
    pub total_paid: Option<f64>,
    pub type_: Option<String>, // Usamos `type_` para evitar conflictos con palabras reservadas
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SalesAsRecord {
    pub id: Thing,
    products: Option<Vec<Thing>>,
    total_paid: Option<f64>,
    customer: Option<String>,
    cashier: Option<String>,
    promocode: Option<String>,
    payment_ref: Option<String>,
    date: Option<String>,
    change: Option<f64>,
    type_: Option<String>, // Usamos `type_` porque `type` es una palabra reservada
    currency: Option<String>,
}

fn thing_to_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = surrealdb::sql::Value::deserialize(deserializer)?;
    match value {
        surrealdb::sql::Value::Thing(thing) => Ok(thing.to_raw()), // Convierte Thing a String
        surrealdb::sql::Value::Strand(s) => Ok(s.to_string()),    // Convierte Strand a String
        _ => Err(serde::de::Error::custom("Formato de Thing inválido")),
    }
}

fn things_to_strings<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::{self, Visitor, SeqAccess};
    use surrealdb::sql::Value as SurrealValue;

    struct ThingsToStrings;

    impl<'de> Visitor<'de> for ThingsToStrings {
        type Value = Option<Vec<String>>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a list of surrealdb::sql::Thing or strings")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut strings = Vec::new();
            while let Some(value) = seq.next_element::<SurrealValue>()? {
                log::info!("Procesando valor: {:?}", value); // Debugging
                match value {
                    SurrealValue::Thing(thing) => {
                        strings.push(thing.to_raw()); // Convierte Thing a String
                    }
                    SurrealValue::Strand(s) => {
                        strings.push(s.to_string()); // Convierte Strand a String
                    }
                    _ => return Err(de::Error::custom("Unexpected value type")),
                }
            }
            Ok(Some(strings))
        }
    }

    deserializer.deserialize_seq(ThingsToStrings)
}


#[derive(Serialize, Deserialize, Debug)]
pub struct SalesAsString {
    pub id: String,
    #[serde(deserialize_with = "things_to_strings")] // Aplica el deserializador aquí
    pub products: Option<Vec<String>>,
    pub total_paid: Option<f64>,
    pub customer: Option<String>,
    pub cashier: Option<String>,
    pub promocode: Option<String>,
    pub payment_ref: Option<String>,
    pub date: Option<String>,
    pub change: Option<f64>,
    #[serde(rename = "type")]
    pub type_: Option<String>,
    pub currency: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Sales {
    #[serde(deserialize_with = "deserialize_products")]
    pub products: Vec<Thing>,
    pub total_paid: f64,
    pub customer: Option<String>,
    pub cashier: String,
    pub promocode: String,
    pub payment_ref: String,
    pub date: Option<String>,
    pub change: f64, // Campo obligatorio
    pub type_: String, // Campo obligatorio
    pub currency: String, // Campo obligatorio
}

impl From<SalesAsRecord> for SalesAsString {
    fn from(record: SalesAsRecord) -> Self {
        SalesAsString {
            id: record.id.to_string(), // Convierte Thing a String
            products: Some(record.products
                .map(|things| {
                    things
                        .into_iter()
                        .map(|thing| thing.to_string()) // Convierte Thing a String
                        .collect::<Vec<String>>()
                })
                .unwrap_or_default()),
            total_paid: record.total_paid,
            customer: record.customer,
            cashier: record.cashier,
            promocode: record.promocode,
            payment_ref: record.payment_ref,
            date: record.date,
            change: record.change,
            type_: record.type_,
            currency: record.currency,
        }
    }
}

fn deserialize_products<'de, D>(deserializer: D) -> Result<Vec<Thing>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw: Vec<JsonValue> = serde::Deserialize::deserialize(deserializer)?;
    raw.into_iter()
        .map(|value| {
            if let Some(id) = value.get("id").and_then(|v| v.as_str()) {
                let parts: Vec<&str> = id.split(":").collect();
                if parts.len() == 2 {
                    Ok(Thing::from((parts[0], parts[1])))
                } else {
                    Err(serde::de::Error::custom(
                        "Invalid format for Thing; expected `table:id`",
                    ))
                }
            } else {
                Err(serde::de::Error::custom(
                    "Expected object with `id` field as string",
                ))
            }
        })
        .collect()
}

pub async fn get_sales(
    database: &State<Surreal<Client>>,
) -> Result<Json<Vec<SimplifiedSales>>, Status> {
    match database
        .query(
            "SELECT 
                cashier,
                change,
                currency,
                customer,
                date,
                id,
                payment_ref,
                products.map(|$product| (
                    SELECT name FROM products WHERE id = $product.id
                )[0].name) AS products_names,
                promocode,
                total_paid,
                type AS type_
             FROM sales",
        )
        .await
    {
        Ok(mut results) => {
            let sales: Vec<SimplifiedSales> = match results.take(0) {
                Ok(data) => data,
                Err(err) => {
                    log::error!("Error al extraer resultados como SimplifiedSales: {:?}", err);
                    return Err(Status::InternalServerError);
                }
            };

            log::info!("Ventas obtenidas: {:?}", sales);
            Ok(Json(sales))
        }
        Err(err) => {
            log::error!("Error al consultar la base de datos: {:?}", err);
            Err(Status::InternalServerError)
        }
    }
}

pub async fn create_sales(
    database: &State<Surreal<Client>>, 
    new_sale: Json<Sales>,
) -> Result<Status, Status> {
    let sale = new_sale.into_inner();

    // Registrar venta
    let products_str = sale
        .products
        .iter()
        .map(|thing| thing.to_string())
        .collect::<Vec<String>>()
        .join(", ");

    let query = format!(
        "CREATE sales CONTENT {{
            products: [{}],
            total_paid: {},
            customer: '{}',
            cashier: '{}',
            promocode: '{}',
            payment_ref: '{}',
            date: '{}',
            change: {},
            type: '{}',
            currency: '{}'
        }} RETURN *;",
        products_str,
        sale.total_paid,
        sale.customer.clone().unwrap_or_default(),
        sale.cashier,
        sale.promocode,
        sale.payment_ref,
        sale.date.clone().unwrap_or_default(),
        sale.change,
        sale.type_,
        sale.currency,
    );

    log::info!("Ejecutando el query: {}", query);

    match database.query(&query).await {
        Ok(mut results) => {
            if let Ok(Some((sale_id, _))) = results.take::<Vec<(Thing, Sales)>>(0).map(|mut v| v.pop()) {
                log::info!("Venta creada correctamente con ID: {}", sale_id.to_string());
            } else {
                log::warn!("Venta creada, pero no se pudo procesar el ID de la venta.");
            }            

            Ok(Status::Created)
        }
        Err(err) => {
            log::error!("Error al ejecutar el query: {:?}", err);
            Err(Status::InternalServerError)
        }
    }
}

pub async fn update_products_for_new_quantities(
    database: &State<Surreal<Client>>,
    products: Json<Vec<ProductWithQuantity>>,
) -> Result<Status, Status> {
    let product_updates = products.into_inner();

    log::info!("Iniciando actualización de inventario.");
    log::info!("Datos recibidos: {:?}", product_updates);

    for product in product_updates {
        let product_id = format!("{}", product.id);

        log::info!("Procesando producto: {}", product_id);

        let query = format!("SELECT quantity FROM {}", product_id);
        log::info!("Ejecutando query: {}", query);

        match database.query(&query).await {
            Ok(mut results) => {
                log::info!("Resultados del query: {:?}", results);

                if let Some(quantity_obj) = results
                    .take::<Vec<HashMap<String, JsonValue>>>(0)
                    .ok()
                    .and_then(|mut res| res.pop())
                {
                    if let Some(current_quantity) = quantity_obj.get("quantity").and_then(|v| v.as_u64()) {
                        log::info!(
                            "Cantidad actual para {}: {}. Requerida: {}",
                            product_id, current_quantity, product.qnt
                        );

                        if current_quantity < product.qnt as u64 {
                            error!(
                                "Stock insuficiente para el producto {}: disponible {}, requerido {}",
                                product_id, current_quantity, product.qnt
                            );
                            return Err(Status::BadRequest);
                        }

                        let new_quantity = current_quantity - product.qnt as u64;
                        log::info!(
                            "Nueva cantidad calculada para {}: {}",
                            product_id, new_quantity
                        );

                        let update_query = format!(
                            "UPDATE {} SET quantity = {}",
                            product_id, new_quantity
                        );
                        log::info!("Ejecutando query de actualización: {}", update_query);

                        if let Err(err) = database.query(&update_query).await {
                            error!(
                                "Error al actualizar el inventario para el producto {}: {:?}",
                                product_id, err
                            );
                            return Err(Status::InternalServerError);
                        }
                    }
                }
            }
            Err(err) => {
                log::error!(
                    "Error al consultar el inventario para el producto {}: {:?}",
                    product_id, err
                );
                return Err(Status::InternalServerError);
            }
        }
    }

    // Agregar `Ok(Status::Ok)` al final de la función
    Ok(Status::Ok)
}

pub async fn delete_sales(
    database: &State<Surreal<Client>>,
    sales_id: String,
    ) -> Result<Status, Status> {
    let query = format!("DELETE sales:{}", sales_id);

    match database.query(&query).await {
        Ok(_) => Ok(Status::Ok),
        Err(err) => {
            error!("Error al eliminar el producto");
            Err(Status::InternalServerError)
        }
    }
}

pub async fn get_sales_by_date_range(
    database: &State<Surreal<Client>>,
    start_date: String,
    end_date: String,
) -> Result<Json<Vec<SimplifiedSales>>, Status> {
    // Validar formato de fechas
    let parsed_start_date = NaiveDate::parse_from_str(&start_date, "%d-%m-%Y")
        .map_err(|_| Status::BadRequest)?;
    let parsed_end_date = NaiveDate::parse_from_str(&end_date, "%d-%m-%Y")
        .map_err(|_| Status::BadRequest)?;

    // Ajustar el formato para incluir horas y minutos
    let start_datetime = format!("{} 00:00", parsed_start_date.format("%d-%m-%y"));
    let end_datetime = format!("{} 23:59", parsed_end_date.format("%d-%m-%y"));

    // Query para buscar ventas en el rango de fechas
    let query = format!(
        "SELECT id, products, payment_method, payment_ref, promocode, date
         FROM sales
         WHERE date >= '{}' AND date <= '{}';",
        start_datetime,
        end_datetime
    );

    match database.query(&query).await {
        Ok(mut results) => {
            let sales: Vec<SimplifiedSales> = results.take(0).unwrap_or_default();
            Ok(Json(sales))
        }
        Err(err) => {
            error!("Error al buscar ventas por rango de fechas: {:?}", err);
            Err(Status::InternalServerError)
        }
    }
}

