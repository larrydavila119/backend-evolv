use crate::crud_inventory::{get_product, update_product, get_category, get_product_by_id, Product, ProductAsString, Category, UpdateProduct};
use crate::promos::{get_discount_codes, DiscountCode};
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::Route;
use rocket::State;
use rocket_basicauth::BasicAuth;
use surrealdb::engine::remote::ws::Client;
use surrealdb::Surreal;
use crate::crud_sales::{SimplifiedSales, create_sales, get_sales, Sales, update_products_for_new_quantities, ProductWithQuantity, SalesAsString, SalesAsRecord};
use chrono::{FixedOffset, Utc, DateTime};
use crate::crud_clients::*;
use crate::exams::*;
use crate::auth::*;
use crate::receipts::*;
use crate::schedules::*;
use serde_json::Value;
use crate::crud_bundles::{get_bundles, get_bundle_by_id, update_bundle, Bundle};

pub fn routes() -> Vec<Route> {
    routes![update_product_route,
        get_product_route,
        get_product_by_id_route,
        get_discount_codes_route,
        get_categories_route,
        create_sales_route,
        get_sales_route,
        get_clients_route,
        update_clients_route,
        create_clients_route,
        update_products_for_new_quantities_route,
        //get_tournaments_route,
        //get_inscription_route,
        //create_inscription_route,
        print_receipt_route,
        get_cashier_payments_route,
        update_cashier_payment_route,
        get_bundles_route,
        update_bundle_route] }

fn get_current_date_utc_minus_6() -> String {
    let utc_minus_6 = FixedOffset::west(6 * 3600);
    let now = Utc::now();
    now.with_timezone(&utc_minus_6)
        .format("%d-%m-%y %H:%M")
        .to_string()
}

#[post("/clients", format = "json", data = "<new_client>")]
pub async fn create_clients_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
    new_client: Json<NewCliente>,
) -> Result<Status, Status> {
    if user.has_role("usuario") || user.is_admin(){
        create_client(database, new_client).await
            .map(|_| Status::Created)
            .map_err(|err| err)
    } else {
        Err(Status::Forbidden)
    }
}

#[get("/clients")]
pub async fn get_clients_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<ClienteAsString>>, Status> {
    if user.has_role("usuario") || user.is_admin() {
        get_clients(database).await
    } else {
        Err(Status::Forbidden)
    }
}

#[put("/clients/<client_id>", format = "json", data = "<updated_data>")]
pub async fn update_clients_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
    client_id: String,
    updated_data: Json<UpdateCliente>,
) -> Result<Status, Status> {
    if user.has_role("usuario") || user.is_admin(){
        update_client(database, client_id, updated_data).await
    } else {
        Err(Status::Forbidden)
    }
}

#[get("/sales")]
pub async fn get_sales_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<SimplifiedSales>>, Status> {
    if user.has_role("usuario") || user.is_admin(){
        get_sales(database).await
    } else {
        Err(Status::Forbidden)
    }
}

#[post("/sales", format = "json", data = "<new_sale>")]
pub async fn create_sales_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
    new_sale: Json<Sales>,
) -> Result<Status, Status> {
    if user.has_role("usuario") || user.is_admin() {
        let current_date = get_current_date_utc_minus_6();
        let mut sale = new_sale.into_inner();
        sale.date = Some(current_date);
        create_sales(database, Json(sale)).await
            .map(|_| Status::Created)
            .map_err(|err| err)
    } else {
        Err(Status::Forbidden)
    }
}

#[get("/promos")]
pub async fn get_discount_codes_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<DiscountCode>>, Status> {
    if user.has_role("usuario") || user.is_admin() {
        get_discount_codes(database).await
    } else {
        Err(Status::Forbidden)
    }
}

#[get("/inventory/categories")]
pub async fn get_categories_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<String>>, Status> {
    if user.has_role("usuario") || user.is_admin() {
        get_category(database).await
    } else {
        Err(Status::Forbidden)
    }
}

#[get("/inventory")]
pub async fn get_product_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<ProductAsString>>, Status> {
    if user.has_role("usuario") || user.is_admin() {
        get_product(database).await
    } else {
        Err(Status::Forbidden)
    }
}

#[put("/inventory/<product_id>", format = "json", data = "<update_data>")]
pub async fn update_product_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
    product_id: String,
    update_data: Json<UpdateProduct>,
) -> Result<Status, Status> {
    if user.has_role("usuario") || user.is_admin(){
        update_product(database, product_id, update_data).await
    } else {
        Err(Status::Forbidden)
    }
}

#[get("/inventory/<product_id>")]
pub async fn get_product_by_id_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
    product_id: String,
) -> Result<Json<ProductAsString>, Status> {
    if user.has_role("usuario") || user.is_admin() {
        get_product_by_id(database, product_id).await
    } else {
        Err(Status::Forbidden)
    }
}

#[post("/update_inventory", format = "json", data = "<products>")]
pub async fn update_products_for_new_quantities_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
    products: Json<Vec<ProductWithQuantity>>,
) -> Result<Status, Status> {
    if user.has_role("usuario") || user.is_admin() {
        update_products_for_new_quantities(database, products).await
    } else {
        Err(Status::Forbidden)
    }
}


// Impresion
#[post("/receipt", format = "json", data = "<sale>")]
pub async fn print_receipt_route(
    sale: Json<PrintedSales>,
    user: AuthenticatedUser,
    database: &State<Surreal<Client>>,
) -> Result<Json<ReceiptJson>, Status> {
    if user.has_role("usuario") || user.is_admin() {
        let receipt = generate_receipt(sale.into_inner(), database).await;
        Ok(Json(receipt))
    } else {
        Err(Status::Forbidden)
    }
}


//Tabla de pagos 
#[get("/payments")]
pub async fn get_cashier_payments_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<SimplifiedPaymentAsString>>, Status> {
    if user.has_role("usuario") || user.is_admin() {
        get_history_payments(database).await
    } else {
        Err(Status::Forbidden)
    }
}

#[put("/payments/<payment_id>", format = "json", data = "<updated_payment>")]
pub async fn update_cashier_payment_route(
    database: &State<Surreal<Client>>,
    payment_id: String,
    updated_payment: Json<Value>,
) -> Result<Status, Status> {
    update_payment(database, payment_id, updated_payment).await
}

//Obtener todos los bundles
#[get("/bundles")]
pub async fn get_bundles_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<Bundle>>, Status> {
    if user.has_role("usuario") || user.is_admin() {
        get_bundles(database).await
    } else {
        Err(Status::Forbidden)
    }
}

// Actualizar un Bundle
#[put("/bundles/<bundle_id>", format = "json", data = "<updated_bundle>")]
pub async fn update_bundle_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
    bundle_id: String,
    updated_bundle: Json<Bundle>,
) -> Result<Status, Status> {
    if user.has_role("usuario") || user.is_admin() {
        update_bundle(database, bundle_id, updated_bundle).await
    } else {
        Err(Status::Forbidden)
    }
}

