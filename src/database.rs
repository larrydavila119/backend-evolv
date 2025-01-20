use surrealdb::engine::remote::ws::Client;
use surrealdb::engine::remote::ws::Ws;
use surrealdb::opt::auth::Root;
use surrealdb::Surreal;
use crate::Paint;

pub async fn connect_db() -> Result<Surreal<Client>, surrealdb::Error> {
    let db = Surreal::new::<Ws>("127.0.0.1:8080").await?;
    db.signin(Root {
        username: "root",
        password: "root",
    })
    .await?;
    db.use_ns("central-choi").use_db("central-choi").await?;

    println!("{}", Paint::green("Conexión a SurrealDB establecida correctamente."));

    Ok(db)
}
