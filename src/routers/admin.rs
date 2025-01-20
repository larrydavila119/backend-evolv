use crate::crud::{delete_user, create_user, update_user, get_users, UpdateUser, User, UserAsString};
use crate::crud_inventory::{create_product, get_product, get_product_by_id, update_product, delete_product, get_category, Product, UpdateProduct, ProductAsString, Category, create_category, delete_category};
use crate::promos::{get_discount_codes, create_discount_code, update_discount_code, delete_discount_code, DiscountCode, UpdateDiscountCode};
use crate::crud_sales::{get_sales_by_date_range, SimplifiedSales, get_sales, Sales, SalesAsString, SalesAsRecord};
use crate::crud_clients::*;
use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::Route;
use rocket::State;
use rocket_basicauth::BasicAuth;
use crate::auth::*;
use surrealdb::engine::remote::ws::Client;
use surrealdb::Surreal;
use crate::exams::*;
use crate::crud_bundles::{create_bundle, get_bundles, get_bundle_by_id, update_bundle, delete_bundle, Bundle};


pub fn routes() -> Vec<Route> {
    routes![create_user_route,
        update_user_route,
        get_user_route,
        delete_user_route,
        get_product_route,
        update_product_route,
        create_product_route,
        delete_product_route,
        get_product_by_id_route,
        get_discount_codes_route,
        update_discount_code_route,
        create_discount_code_route,
        delete_discount_code_route,
        get_categories_route,
        create_categories_route,
        delete_categories_route,
        get_sales_route,
        get_sales_by_date_range_route,
        create_clients_route,
        get_clients_route,
        update_clients_route,
        delete_clients_route,
        //create_tournament_route,
        //get_tournaments_route,
        //update_tournament_route,
        //delete_tournament_route,
        create_exam_route,
        get_exams_route,
        update_exam_route,
        delete_exam_route,
        create_bundle_route,
        get_bundles_route,
        get_bundle_by_id_route,
        update_bundle_route,
        delete_bundle_route]}


// CRUD de Clientes
#[post("/clients", format = "json", data = "<new_client>")]
pub async fn create_clients_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
    new_client: Json<NewCliente>,
) -> Result<Status, Status> {
    // Verificar si el usuario tiene el rol de admin
    if user.is_admin() {
        create_client(database, new_client).await
    } else {
        // Si no es admin, denegar el acceso
        Err(Status::Forbidden)
    }
}

#[delete("/clients/<client_id>")]
pub async fn delete_clients_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
    client_id: String,
) -> Result<Status, Status> {
    if user.is_admin() {
        delete_client(database, client_id).await
    } else {
        Err(Status::Forbidden)
    }
}

#[get("/clients")]
pub async fn get_clients_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<ClienteAsString>>, Status> {
    if user.is_admin() {
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
    if user.is_admin() {
        update_client(database, client_id, updated_data).await
    } else {
        Err(Status::Forbidden)
    }
}

//Obtener las ventas:
#[get("/sales")]
pub async fn get_sales_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<SimplifiedSales>>, Status> {
    if user.is_admin() || user.has_role("usuario") {
        get_sales(database).await
    } else {
        Err(Status::Forbidden)
    }
}


//CRUD de los codigos de promoción

#[get("/promos")]
pub async fn get_discount_codes_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<DiscountCode>>, Status> {
    if user.is_admin() {
        get_discount_codes(database).await
    } else {
        Err(Status::Forbidden)
    }
}

#[post("/promos", format = "json", data = "<new_code>")]
pub async fn create_discount_code_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
    new_code: Json<DiscountCode>,
) -> Result<Status, Status> {
    if user.is_admin() {
        create_discount_code(database, new_code).await
    } else {
        Err(Status::Forbidden)
    }
}

#[put("/promos/<discount_id>", format = "json", data = "<update_data>")]
pub async fn update_discount_code_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
    discount_id: String,
    update_data: Json<UpdateDiscountCode>,
) -> Result<Status, Status> {
    if user.is_admin() {
        update_discount_code(database, discount_id, update_data).await
    } else {
        Err(Status::Forbidden)
    }
}

#[delete("/promos/<discount_id>")]
pub async fn delete_discount_code_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
    discount_id: String,
) -> Result<Status, Status> {
    if user.is_admin() {
        delete_discount_code(database, discount_id).await
    } else {
        Err(Status::Forbidden)
    }
}


//CRUD de los usuarios
#[post("/users", format = "json", data = "<new_user>")]
pub async fn create_user_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
    new_user: Json<User>,
) -> Result<Status, Status> {
    if user.is_admin() {
        create_user(database, new_user).await
    } else {
        Err(Status::Forbidden)
    }
}

#[put("/users/<user_id>", format = "json", data = "<update_data>")]
pub async fn update_user_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
    user_id: String,
    update_data: Json<UpdateUser>,
) -> Result<Status, Status> {
    if user.is_admin() {
        update_user(database, user_id, update_data).await
    } else {
        Err(Status::Forbidden)
    }
}

#[get("/users")]
pub async fn get_user_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<UserAsString>>, Status> {
    if user.is_admin() {
        get_users(database).await
    } else {
        Err(Status::Forbidden)
    }
}
#[delete("/users/<user_id>")]
pub async fn delete_user_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
    user_id: String,
) -> Result<Status, Status> {
    if user.is_admin() {
        delete_user(database, user_id).await
    } else {
        Err(Status::Forbidden)
    }
}
// CRUD del Inventariado

#[get("/inventory/categories")]
pub async fn get_categories_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<String>>, Status> {
    if user.is_admin() {
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
    if user.has_role("admin") || user.has_role("usuario") {
        get_product(database).await
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
    if user.has_role("admin") || user.has_role("usuario") {
        get_product_by_id(database, product_id).await
    } else {
        Err(Status::Forbidden)
    }
}

#[post("/inventory/categories", format = "json", data = "<new_category>")]
pub async fn create_categories_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
    new_category: Json<Category>,
) -> Result<Status, Status> {
    if user.is_admin() {
        create_category(database, new_category).await
    } else {
        Err(Status::Forbidden)
    }
}

#[post("/inventory", format = "json", data = "<new_product>")]
pub async fn create_product_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
    new_product: Json<Product>,
) -> Result<Status, Status> {
    if user.is_admin() {
        create_product(database, new_product).await
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
    if user.is_admin() {
        update_product(database, product_id, update_data).await
    } else {
        Err(Status::Forbidden)
    }
}

#[delete("/inventory/<product_id>")]
pub async fn delete_product_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
    product_id: String,
) -> Result<Status, Status> {
    if user.is_admin() {
        delete_product(database, product_id).await
    } else {
        Err(Status::Forbidden)
    }
}

#[delete("/inventory/categories/<category>")]
pub async fn delete_categories_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
    category: String,
) -> Result<Status, Status> {
    if user.is_admin() {
        delete_category(database, category).await
    } else {
        Err(Status::Forbidden)
    }
}

//Examenes 

#[post("/exams", format = "json", data = "<new_exam>")]
pub async fn create_exam_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
    new_exam: Json<Exam>,
) -> Result<Status, Status> {
    if user.is_admin() {
        create_exam(database, new_exam).await
    } else {
        Err(Status::Forbidden)
    }
}

#[get("/exams")]
pub async fn get_exams_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<ExamAsString>>, Status> {
    if user.has_role("admin") || user.has_role("usuario") {
        get_exams(database).await
    } else {
        Err(Status::Forbidden)
    }
}

#[put("/exams/<exam_id>", format = "json", data = "<update_data>")]
pub async fn update_exam_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
    exam_id: String,
    update_data: Json<UpdateExam>,
) -> Result<Status, Status> {
    if user.is_admin() {
        update_exam(database, exam_id, update_data).await
    } else {
        Err(Status::Forbidden)
    }
}

#[delete("/exams/<exam_id>")]
pub async fn delete_exam_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
    exam_id: String,
) -> Result<Status, Status> {
    if user.is_admin() {
        delete_exam(database, exam_id).await
    } else {
        Err(Status::Forbidden)
    }
}

#[post("/sales/date-range", format = "json", data = "<date_range>")]
pub async fn get_sales_by_date_range_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
    date_range: Json<serde_json::Value>,
) -> Result<Json<Vec<SimplifiedSales>>, Status> {
    if user.has_role("admin") || user.has_role("usuario") {
        let start_date = date_range.get("start_date").and_then(|v| v.as_str()).unwrap_or("");
        let end_date = date_range.get("end_date").and_then(|v| v.as_str()).unwrap_or("");

        if start_date.is_empty() || end_date.is_empty() {
            return Err(Status::BadRequest);
        }

        get_sales_by_date_range(database, start_date.to_string(), end_date.to_string()).await
    } else {
        Err(Status::Forbidden)
    }
}

// Crear un Bundle
#[post("/bundles", format = "json", data = "<new_bundle>")]
pub async fn create_bundle_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
    new_bundle: Json<Bundle>,
) -> Result<Status, Status> {
    if user.is_admin() {
        create_bundle(database, new_bundle).await
    } else {
        Err(Status::Forbidden)
    }
}

// Obtener todos los Bundles
#[get("/bundles")]
pub async fn get_bundles_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
) -> Result<Json<Vec<Bundle>>, Status> {
    if user.is_admin() {
        get_bundles(database).await
    } else {
        Err(Status::Forbidden)
    }
}

// Obtener un Bundle por ID
#[get("/bundles/<bundle_id>")]
pub async fn get_bundle_by_id_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
    bundle_id: String,
) -> Result<Json<Bundle>, Status> {
    if user.is_admin() {
        get_bundle_by_id(database, bundle_id).await
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
    if user.is_admin() {
        update_bundle(database, bundle_id, updated_bundle).await
    } else {
        Err(Status::Forbidden)
    }
}

// Eliminar un Bundle
#[delete("/bundles/<bundle_id>")]
pub async fn delete_bundle_route(
    database: &State<Surreal<Client>>,
    user: AuthenticatedUser,
    bundle_id: String,
) -> Result<Status, Status> {
    if user.is_admin() {
        delete_bundle(database, bundle_id).await
    } else {
        Err(Status::Forbidden)
    }
}

