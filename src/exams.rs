use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use serde::Deserialize;
use serde::Serialize;
use surrealdb::engine::remote::ws::Client;
use surrealdb::Surreal;
use log::{info, error};
use surrealdb::sql::Thing;

#[derive(Serialize, Deserialize)]
pub struct Exam {
    id: Option<Thing>,
    name: String,
    price: f64,
}

#[derive(Serialize, Deserialize)]
pub struct UpdateExam {
    name: Option<String>,
    price: Option<f64>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ExamAsString {
    pub id: String,
    pub name: String,
    pub price: f64,
}

impl From<Exam> for ExamAsString {
    fn from(exam: Exam) -> Self {
        ExamAsString {
            id: exam.id.map(|thing| thing.to_string()).unwrap_or_default(),
            name: exam.name,
            price: exam.price,
        }
    }
}

pub async fn create_exam(
    database: &State<Surreal<Client>>,
    new_exam: Json<Exam>,
) -> Result<Status, Status> {
    let exam = new_exam.into_inner();
    let query_check = format!(
        "SELECT * FROM exams WHERE name = '{}';",
        exam.name
    );
    if let Ok(mut results) = database.query(&query_check).await {
        if let Ok(Some(_)) = results.take::<Option<Exam>>(0) {
            error!("El examen '{}' ya existe", exam.name);
            return Err(Status::Conflict);
        }
    }

    let query = format!(
        "CREATE exams CONTENT {{
            name: '{}',
            price: {}
        }} RETURN *;",
        exam.name, exam.price
    );

    info!("Ejecutando el query para crear el examen: {}", exam.name);

    match database.query(&query).await {
        Ok(mut results) => {
            if let Ok(Some(_)) = results.take::<Option<Exam>>(0) {
                info!("Examen '{}' creado correctamente", exam.name);
                return Ok(Status::Created);
            }
        }
        Err(err) => {
            error!("Error al crear el examen: {:?}", err);
        }
    }

    error!("Creación del examen fallida");
    Err(Status::InternalServerError)
}

pub async fn get_exams(
    database: &State<Surreal<Client>>,
) -> Result<Json<Vec<ExamAsString>>, Status> {
    let query = "SELECT * FROM exams;";

    match database.query(query).await {
        Ok(mut results) => {
            let raw_exams: Vec<Exam> = results.take(0).unwrap_or_default();
            let exams: Vec<ExamAsString> = raw_exams
                .into_iter()
                .map(ExamAsString::from)
                .collect();
            Ok(Json(exams))
        }
        Err(err) => {
            error!("Error al obtener los exámenes: {:?}", err);
            Err(Status::InternalServerError)
        }
    }
}

pub async fn update_exam(
    database: &State<Surreal<Client>>,
    exam_id: String,
    update_data: Json<UpdateExam>,
) -> Result<Status, Status> {
    let mut updates = Vec::new();

    if let Some(name) = &update_data.name {
        updates.push(format!("name = '{}'", name));
    }
    if let Some(price) = &update_data.price {
        updates.push(format!("price = {}", price));
    }

    if updates.is_empty() {
        return Err(Status::BadRequest);
    }

    let query = format!(
        "UPDATE {} SET {};",
        exam_id,
        updates.join(", ")
    );

    info!("Ejecutando query: {}", query);

    match database.query(&query).await {
        Ok(_) => Ok(Status::Ok),
        Err(err) => {
            error!("Error al actualizar el examen: {:?}", err);
            Err(Status::InternalServerError)
        }
    }
}

pub async fn delete_exam(
    database: &State<Surreal<Client>>,
    exam_id: String,
) -> Result<Status, Status> {
    let query = format!("DELETE {};", exam_id);

    match database.query(&query).await {
        Ok(_) => Ok(Status::Ok),
        Err(err) => {
            error!("Error al eliminar el examen: {:?}", err);
            Err(Status::InternalServerError)
        }
    }
}

