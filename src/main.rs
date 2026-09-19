mod db;
mod model;
mod routes;
mod ui;

use db::Database;
use topcoat::router::{Router, RouterBuilderDiscoverExt};

#[tokio::main]
async fn main() -> topcoat::Result<()> {
    let path = std::env::var("TODO_DB_PATH").unwrap_or_else(|_| "data/todos.sqlite3".into());
    let database = Database::open(path).await?;
    let router = Router::builder().app_context(database).discover().build();
    println!("Topcoat Todo is ready. Data is stored in SQLite.");
    topcoat::start(router).await?;
    Ok(())
}
