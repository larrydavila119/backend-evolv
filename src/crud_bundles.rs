use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use serde::{Deserialize, Serialize};
use surrealdb::engine::remote::ws::Client;
use surrealdb::Surreal;
use crate::crud_sales::ProductWithQuantity; // Usa la estructura definida en el inventario
use log::{info, error};



// Estructura para representar un Bundle
#[derive(Serialize, Deserialize, Debug)]
pub struct Bundle {
    pub name: String,
    pub products: Vec<ProductWithQuantity>, // Productos y cantidades
    pub discount: Option<f64>,             // Descuento opcional
}

pub async fn create_bundle(
    database: &State<Surreal<Client>>,
    new_bundle: Json<Bundle>,
) -> Result<Status, Status> {
    let bundle = new_bundle.into_inner();

    // Validar que los productos existan en el inventario
    for product in &bundle.products {
        let product_id = format!("{}", product.id);
        if crate::crud_inventory::get_product_by_id(database, product_id.clone()).await.is_err() {
            log::error!("Producto con ID {} no existe en el inventario.", product_id);
            return Err(Status::BadRequest);
        }
    }

    // Calcular el precio total del Bundle (sin descuento)
    let mut total_price = 0.0;
    for product in &bundle.products {
        let product_data = crate::crud_inventory::get_product_by_id(database, product.id.clone())
            .await
            .map_err(|_| Status::InternalServerError)?;
        if let Some(price) = product_data.price {
            total_price += price * product.qnt as f64;
        }
    }

    // Aplicar descuento si es necesario
    let final_price = if let Some(discount) = bundle.discount {
        total_price * (1.0 - discount / 100.0)
    } else {
        total_price
    };

    // Guardar el Bundle en la base de datos
    let query = format!(
        "CREATE bundles CONTENT {{
            name: '{}',
            products: {},
            discount: {},
            final_price: {}
        }};",
        bundle.name,
        serde_json::to_string(&bundle.products).unwrap(),
        bundle.discount.unwrap_or(0.0),
        final_price
    );

    match database.query(&query).await {
        Ok(_) => {
            log::info!("Bundle '{}' creado correctamente.", bundle.name);
            Ok(Status::Created)
        }
        Err(err) => {
            log::error!("Error al crear el Bundle: {:?}", err);
            Err(Status::InternalServerError)
        }
    }
}

pub async fn get_bundles(
    database: &State<Surreal<Client>>,
) -> Result<Json<Vec<Bundle>>, Status> {
    let query = "SELECT * FROM bundles;";

    match database.query(query).await {
        Ok(mut results) => {
            let bundles: Vec<Bundle> = results.take(0).unwrap_or_default();
            log::info!("Bundles obtenidos: {:?}", bundles);
            Ok(Json(bundles))
        }
        Err(err) => {
            log::error!("Error al obtener los Bundles: {:?}", err);
            Err(Status::InternalServerError)
        }
    }
}

pub async fn get_bundle_by_id(
    database: &State<Surreal<Client>>,
    bundle_id: String,
) -> Result<Json<Bundle>, Status> {
    let query = format!("SELECT * FROM bundles WHERE id = '{}';", bundle_id);

    match database.query(&query).await {
        Ok(mut results) => {
            if let Some(bundle) = results.take::<Vec<Bundle>>(0).ok().and_then(|mut b| b.pop()) {
                Ok(Json(bundle))
            } else {
                Err(Status::NotFound)
            }
        }
        Err(err) => {
            log::error!("Error al obtener el Bundle: {:?}", err);
            Err(Status::InternalServerError)
        }
    }
}

pub async fn update_bundle(
    database: &State<Surreal<Client>>,
    bundle_id: String,
    update_data: Json<Bundle>,
) -> Result<Status, Status> {
    let bundle = update_data.into_inner();

    // Serializar los datos actualizados
    let update_content = serde_json::to_string(&bundle).map_err(|_| Status::BadRequest)?;

    let query = format!(
        "UPDATE bundles:{} MERGE {};",
        bundle_id, update_content
    );

    match database.query(&query).await {
        Ok(_) => {
            log::info!("Bundle '{}' actualizado correctamente.", bundle_id);
            Ok(Status::Ok)
        }
        Err(err) => {
            log::error!("Error al actualizar el Bundle: {:?}", err);
            Err(Status::InternalServerError)
        }
    }
}

pub async fn delete_bundle(
    database: &State<Surreal<Client>>,
    bundle_id: String,
) -> Result<Status, Status> {
    let query = format!("DELETE bundles:{};", bundle_id);

    match database.query(&query).await {
        Ok(_) => {
            log::info!("Bundle '{}' eliminado correctamente.", bundle_id);
            Ok(Status::Ok)
        }
        Err(err) => {
            log::error!("Error al eliminar el Bundle: {:?}", err);
            Err(Status::InternalServerError)
        }
    }
}

