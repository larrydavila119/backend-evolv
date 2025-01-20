use rocket::http::Status;
use rocket::serde::json::Json;
use rocket::State;
use serde::{Deserialize, Serialize};
use surrealdb::engine::remote::ws::Client;
use surrealdb::Surreal;
use surrealdb::sql::Thing;
use log::{info, error};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug)]
pub struct NewCliente {
    pub fullname: String,
    pub is_minor: bool,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub monthly_pay_ref: Option<String>,
    pub is_preferred: bool,
    pub schedule: Option<String>,
    pub is_active: bool,
    pub times: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Cliente {
    pub id: Thing, 
    pub fullname: String,
    pub is_minor: bool,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub monthly_pay_ref: Option<String>,
    pub is_preferred: bool,
    pub schedule: Option<String>,
    pub is_active: bool,
    pub times: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ClienteAsString {
    pub id: String,
    pub fullname: String,
    pub is_minor: bool,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub monthly_pay_ref: Option<String>,
    pub is_preferred: bool,
    pub schedule: Option<String>,
    pub is_active: bool,
    pub times: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateCliente {
    pub fullname: Option<String>,
    pub is_minor: Option<bool>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub monthly_pay_ref: Option<String>,
    pub is_preferred: Option<bool>,
    pub schedule: Option<String>,
    pub is_active: bool,
    pub times: Option<String>,
}

// Implementación de conversión de `Client` a `ClientAsString`
impl From<(Thing, Cliente)> for ClienteAsString {
    fn from((thing, cliente): (Thing, Cliente)) -> Self {
        ClienteAsString {
            id: thing.to_string(),
            fullname: cliente.fullname,
            is_minor: cliente.is_minor,
            phone: cliente.phone,
            email: cliente.email,
            monthly_pay_ref: cliente.monthly_pay_ref,
            is_preferred: cliente.is_preferred,
            schedule: cliente.schedule,
            is_active: cliente.is_active,
            times: cliente.times,
        }
    }
}

pub async fn create_client(
    database: &State<Surreal<Client>>,
    new_client: Json<NewCliente>,
) -> Result<Status, Status> {
    let client = new_client.into_inner();

    if client.is_minor && (client.phone.is_none() || client.email.is_none()) {
        error!("Un cliente menor de edad debe tener un teléfono y correo electrónico.");
        return Err(Status::BadRequest);
    }

    // Crear el cliente
    let query = format!(
        "CREATE clients CONTENT {{
            fullname: '{}',
            is_minor: {},
            phone: '{}',
            email: '{}',
            monthly_pay_ref: '{}',
            is_preferred: {},
            schedule: '{}',
            is_active: {},
            times: '{}'
        }} RETURN *;",
        client.fullname,
        client.is_minor,
        client.phone.unwrap_or_default(),
        client.email.unwrap_or_default(),
        client.monthly_pay_ref.unwrap_or_default(),
        client.is_preferred,
        client.schedule.clone().unwrap_or_default(),
        client.is_active,
        client.times.unwrap_or_default()
    );

    match database.query(&query).await {
        Ok(mut results) => {
            match results.take::<Vec<Cliente>>(0) {
                Ok(vec) if !vec.is_empty() => {
                    let cliente = vec[0].clone();

                    // Consultar el id del schedule basado en el nombre proporcionado
                    let schedule_id: Option<Thing> = if let Some(ref schedule_name) = cliente.schedule {
                        let schedule_query = format!(
                            "SELECT VALUE id FROM schedules WHERE name = '{}';",
                            schedule_name
                        );
                        match database.query(&schedule_query).await {
                            Ok(mut res) => {
                                let result: Option<Thing> = res.take(0).unwrap_or(None);
                                result
                            }
                            Err(err) => {
                                error!(
                                    "Error al obtener el schedule para el cliente: {:?}, error: {:?}",
                                    schedule_name, err
                                );
                                return Err(Status::InternalServerError);
                            }
                        }
                    } else {
                        None
                    };

                    // Crear el registro en payments
                    let payment_query = format!(
                        "CREATE payments CONTENT {{
                            client_id: {},
                            months: [{{
                                Enero: false,
                                Febrero: false,
                                Marzo: false,
                                Abril: false,
                                Mayo: false,
                                Junio: false,
                                Julio: false,
                                Agosto: false,
                                Septiembre: false,
                                Octubre: false,
                                Noviembre: false,
                                Diciembre: false
                            }}],
                            schedule: {},
                            year: 2025
                        }};",
                        cliente.id,
                        schedule_id.map(|id| id.to_string()).unwrap_or_else(|| "null".to_string())
                    );

                    match database.query(&payment_query).await {
                        Ok(_) => {
                            info!("Registro en payments creado para el cliente: {:?}", cliente.id);
                        }
                        Err(err) => {
                            error!(
                                "Error al crear registro en payments para el cliente: {:?}, error: {:?}",
                                cliente.id, err
                            );
                            return Err(Status::InternalServerError);
                        }
                    }

                    info!("Cliente creado correctamente: {:?}", cliente);
                    Ok(Status::Created)
                }
                Ok(_) => {
                    error!("El resultado del query está vacío.");
                    Err(Status::InternalServerError)
                }
                Err(e) => {
                    error!("Error al procesar el resultado del query: {:?}", e);
                    Err(Status::InternalServerError)
                }
            }
        }
        Err(err) => {
            error!("Error al ejecutar el query: {:?}", err);
            Err(Status::InternalServerError)
        }
    }
}


pub async fn get_clients(
    database: &State<Surreal<Client>>,
) -> Result<Json<Vec<ClienteAsString>>, Status> {
    let result: Result<Vec<Cliente>, surrealdb::Error> = database.select("clients").await;

    match result {
        Ok(raw_clients) => {
            let clients: Vec<ClienteAsString> = raw_clients
                .into_iter()
                .map(|client| ClienteAsString {
                    id: client.id.to_string(),
                    fullname: client.fullname,
                    is_minor: client.is_minor,
                    phone: client.phone,
                    email: client.email,
                    monthly_pay_ref: client.monthly_pay_ref,
                    is_preferred: client.is_preferred,
                    schedule: client.schedule,
                    is_active: client.is_active,
                    times: client.times,
                })
                .collect();

            info!("Clientes obtenidos exitosamente.");
            Ok(Json(clients))
        }
        Err(err) => {
            error!("Error al consultar la base de datos: {:?}", err);
            Err(Status::InternalServerError)
        }
    }
}

pub async fn update_client(
    database: &State<Surreal<Client>>, 
    client_id: String,
    updated_data: Json<UpdateCliente>,
) -> Result<Status, Status> {
    let client = updated_data.into_inner();

    let query = format!(
        "UPDATE clients:{} CONTENT {{
            fullname: '{}',
            is_minor: {},
            phone: '{}',
            email: '{}',
            monthly_pay_ref: '{}',
            is_preferred: {},
            schedule: '{}',
            is_active: {},
            times: '{}'
        }};",
        client_id,
        client.fullname.unwrap_or_default(),
        client.is_minor.unwrap_or_default(),
        client.phone.unwrap_or_default(),
        client.email.unwrap_or_default(),
        client.monthly_pay_ref.unwrap_or_default(),
        client.is_preferred.unwrap_or_default(),
        client.schedule.unwrap_or_default(),
        client.is_active,
        client.times.unwrap_or_default(),
    );

    match database.query(&query).await {
        Ok(_) => {
            info!("Cliente actualizado correctamente.");
            Ok(Status::Ok)
        }
        Err(err) => {
            error!("Error al actualizar cliente: {:?}", err);
            Err(Status::InternalServerError)
        }
    }
}

pub async fn delete_client(
    database: &State<Surreal<Client>>, 
    client_id: String,
) -> Result<Status, Status> {
    let query = format!("DELETE {};", client_id);

    match database.query(&query).await {
        Ok(_) => {
            info!("Cliente eliminado correctamente.");
            Ok(Status::Ok)
        }
        Err(err) => {
            error!("Error al eliminar cliente: {:?}", err);
            Err(Status::InternalServerError)
        }
    }
}

