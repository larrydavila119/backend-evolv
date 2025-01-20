use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
use surrealdb::engine::remote::ws::Client;
use surrealdb::Surreal;
use log::{info, error};
use surrealdb::sql::Thing;
use std::collections::HashSet;

#[derive(Serialize, Deserialize, Debug)]
pub struct ProductAsRecord {
    id: Thing,
    name: Option<String>,
    price: Option<f64>,
    bar_code: Option<String>,
    quantity: Option<u32>,
    category: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct ProductAsString {
    pub id: String,
    pub name: Option<String>,
    pub price: Option<f64>,
    pub bar_code: Option<String>,
    pub quantity: Option<u32>,
    pub category: Option<String>,
}

impl From<ProductAsRecord> for ProductAsString {
    fn from(record: ProductAsRecord) -> Self {
        ProductAsString {
            id: record.id.to_string(),
            name: record.name,
            price: record.price,
            bar_code: record.bar_code,
            quantity: record.quantity,
            category: record.category,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Category {
    name: String,
}

#[derive(Serialize, Deserialize)]
pub struct Product {
    name: String,
    price: f64,
    bar_code: String,
    quantity: u32,
    category: String,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateProduct {
    name: Option<String>,
    price: Option<f64>,
    bar_code: Option<String>,
    quantity: Option<u32>,
    category: Option<String>,
}

pub async fn get_product(database: &State<Surreal<Client>>) -> Result<Json<Vec<ProductAsString>>, Status> {
    let result: Result<Vec<ProductAsRecord>, surrealdb::Error> = database.select("products").await;

    match result {
        Ok(raw_products) => {
            let products: Vec<ProductAsString> = raw_products
                .into_iter()
                .map(ProductAsString::from)
                .collect();
            info!("Productos obtenidos exitosamente: {:?}", products);
            Ok(Json(products))
        }
        Err(err) => {
            error!("Error al consultar la base de datos: {:?}", err);
            Err(Status::InternalServerError)
        }
    }
}

pub async fn get_product_by_id(
    database: &State<Surreal<Client>>,
    product_id: String,
) -> Result<Json<ProductAsString>, Status> {
    // Obtener el prefijo y el ID base
    let parts: Vec<&str> = product_id.split(':').collect();
    if parts.len() != 2 {
        log::error!("Formato de ID inválido: {}", product_id);
        return Err(Status::BadRequest);
    }
    let (table_name, id_base) = (parts[0], parts[1]);

    // Función auxiliar para realizar una consulta
    async fn query_table(
        database: &State<Surreal<Client>>,
        table: &str,
        id_base: &str,
    ) -> Result<Option<ProductAsRecord>, Status> {
        let thing_id = Thing::from((table, id_base));
        let query = format!("SELECT * FROM {} WHERE id = $id;", table);
        match database.query(&query).bind(("id", thing_id)).await {
            Ok(mut results) => {
                if let Ok(Some(record)) = results.take::<Option<ProductAsRecord>>(0) {
                    return Ok(Some(record));
                }
            }
            Err(err) => {
                log::error!("Error al consultar la tabla '{}': {:?}", table, err);
            }
        }
        Ok(None)
    }

    // Buscar en "products"
    if let Ok(Some(product)) = query_table(database, "products", id_base).await {
        log::info!("Producto encontrado en 'products': {:?}", product);
        return Ok(Json(ProductAsString::from(product)));
    }

    // Buscar en "exams"
    if let Ok(Some(product)) = query_table(database, "exams", id_base).await {
        log::info!("Producto encontrado en 'exams': {:?}", product);
        return Ok(Json(ProductAsString::from(product)));
    }

    // Buscar en "monthly"
    if let Ok(Some(product)) = query_table(database, "monthly", id_base).await {
        log::info!("Producto encontrado en 'monthly': {:?}", product);
        return Ok(Json(ProductAsString::from(product)));
    }

    // Si no se encuentra en ninguna tabla
    log::error!(
        "Producto no encontrado en 'products', 'exams' o 'monthly' para el ID: {}",
        product_id
    );
    Err(Status::NotFound)
}


pub async fn create_product(
    database: &State<Surreal<Client>>, 
    new_product: Json<Product>,
) -> Result<Status, Status> {
    let product = new_product.into_inner();
    let query_check = format!(
        "SELECT * FROM products WHERE name = ' ' category = '{}';",
        product.category
    );
    let result_check = database.query(&query_check).await;

    if let Ok(mut results) = result_check {
        if let Ok(Some(_)) = results.take::<Option<Product>>(0) {
            log::error!("El producto ya existe en la categoria '{}', utilice otra categoria porfavor.", product.category);
            return Err(Status::Conflict);
        }
    }
    let query = format!(
        "CREATE products CONTENT {{
            name: '{}',
            price: {},
            bar_code: '{}',
            quantity: {},
            category: '{}'
        }};",
        product.name, product.price, product.bar_code, product.quantity, product.category
    );

    log::info!("Ejecutando el query: {}", query);

    let Ok(mut results) = database.query(&query).await else {
        log::error!("Peticion a la base de datos ha fallado.");
        return Err(Status::InternalServerError);
    };
    if let Ok(Some(_)) = results.take::<Option<Product>>(0) {
        log::info!("Producto creado correctamente creado");
        Ok(Status::Created)
    } else {
        log::error!("Creacion del producto fallida: Sin resultados por retornar");
        Err(Status::InternalServerError)
    }
}

pub async fn update_product(
    database: &State<Surreal<Client>>,
    product_id: String,
    update_data: Json<UpdateProduct>,
) -> Result<Status, Status> {
    let mut updates = Vec::new();

    if let Some(name) = &update_data.name {
        updates.push(format!("name = '{}'", name));
    }
    if let Some(price) = &update_data.price {
        updates.push(format!("price = {}", price));
    }
    if let Some(bar_code) = &update_data.bar_code {
        updates.push(format!("bar_code = '{}'", bar_code));
    }
    if let Some(quantity) = &update_data.quantity {
        updates.push(format!("quantity = {}", quantity));
    }
    if let Some(category) = &update_data.category {
        updates.push(format!("category = '{}'", category));
    }

    if updates.is_empty() {
        return Err(Status::BadRequest);
    }

    let query = format!(
        "UPDATE {} SET {};",
        product_id,
        updates.join(", ")
    );

    log::info!("Ejecutando query: {}", query);

    match database.query(&query).await {
        Ok(_) => Ok(Status::Ok),
        Err(err) => {
            log::error!("Error al actualizar el producto: {:?}", err);
            Err(Status::InternalServerError)
        }
    }
}

pub async fn delete_product(
    database: &State<Surreal<Client>>,
    product_id: String,
) -> Result<Status, Status> {
    let query = format!("DELETE {};", product_id);

    match database.query(&query).await {
        Ok(_) => Ok(Status::Ok),
        Err(err) => {
            error!("Error al eliminar el producto: {:?}", err);
            Err(Status::InternalServerError)
        }
    }
}

pub async fn create_category(
    database: &State<Surreal<Client>>,
    new_category: Json<Category>,
) -> Result<Status, Status> {
    let category = new_category.into_inner();
    let query_check = format!(
        "SELECT * FROM categories WHERE name = '{}';", category.name
    );
    let result_check = database.query(&query_check).await;

    if let Ok(mut results) = result_check {
        if let Ok(Some(_)) = results.take::<Option<Product>>(0) {
            log::error!("La categoria '{}' ya existe, utilice otra categoria porfavor.", category.name);
            return Err(Status::Conflict);
        }
    }
    let query = format!(
        "CREATE categories CONTENT {{
            name: '{}'
        }};",
        category.name
    );

    log::info!("Ejecutando el query: {}", query);

    let Ok(mut results) = database.query(&query).await else {
        log::error!("Peticion a la base de datos ha fallado.");
        return Err(Status::InternalServerError);
    };
    if let Ok(Some(_)) = results.take::<Option<Product>>(0) {
        log::info!("Categoria creada correctamente.");
        Ok(Status::Created)
    } else {
        Ok(Status::Created)
    }
}

pub async fn get_category(
    database: &State<Surreal<Client>>,
) -> Result<Json<Vec<String>>, Status> {
    let query = "SELECT name FROM categories;";

    match database.query(query).await {
        Ok(mut results) => {
            let raw_categories: Vec<serde_json::Value> = results.take(0).unwrap_or_default();
            let categories: Vec<String> = raw_categories
                .into_iter()
                .filter_map(|category| category.get("name").and_then(|name| name.as_str()).map(|name| name.to_string()))
                .collect();

            log::info!("Nombres de categorías extraídos: {:?}", categories);
            Ok(Json(categories))
        }
        Err(err) => {
            error!("Error al obtener las categorías: {:?}", err);
            Err(Status::InternalServerError)
        }
    }
}

pub async fn delete_category(
    database: &State<Surreal<Client>>,
    category: String,
) -> Result<Status, Status> {
    let query = format!("DELETE FROM categories WHERE name = '{}';", category);

    match database.query(&query).await {
        Ok(_) => Ok(Status::Ok),
        Err(err) => {
            error!("Error al eliminar el producto: {:?}", err);
            Err(Status::InternalServerError)
        }
    }
}

