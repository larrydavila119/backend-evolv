#[macro_use]
extern crate rocket;
mod crud;
mod database;
mod routers;
mod crud_inventory;
mod promos;
mod crud_sales;
mod crud_clients;
mod auth;
mod receipts;
mod exams;
mod crud_bundles;
mod schedules;
//mod android_printer;

use crate::routers::admin::routes;
use surrealdb::Surreal;
use surrealdb::engine::remote::ws::Client;
use rocket::Request;
use crate::rocket::yansi::Paint;

#[catch(500)]
fn internal_error(req: &Request) -> String {
    format!("Internal error: something went wrong with your request at: {}", req.uri())
}
#[launch]
async fn rocket() -> _ {
    let db: Surreal<Client> = database::connect_db().await.expect("fallo de conexión a la DB");
    rocket::build()
        .manage(db)
        .mount("/admin", routes())
        .mount("/cashier", routers::cashier::routes())
        .register("/", catchers![internal_error]) 
    }   
