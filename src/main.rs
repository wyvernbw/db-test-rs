#![feature(const_trait_impl)]
#![feature(associated_type_defaults)]
#![feature(impl_trait_in_assoc_type)]

use std::ops::Deref;

use db::{Coffee, Nanoid};
use serde::{Deserialize, Serialize};
use service::{server_loop, IntoResponse, Request};
use sqlx::{query, query_as, Pool, Sqlite};

mod db;
mod service;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    server_loop(handle_with_timeout).await?;
    Ok(())
}

async fn handle_with_timeout(req: Request, db: &Pool<Sqlite>) -> impl IntoResponse {
    let result =
        tokio::time::timeout(tokio::time::Duration::from_secs(30), rpc_router(req, db)).await?;
    anyhow::Ok(result.into_response().body)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", content = "data")]
pub enum Methods {
    GetRandomCoffee,
    AddCoffee(AddCoffeeParams),
    GrabId { roastery: String, origin: String },
    GrabCoffee(Nanoid),
    EditCoffee(EditCoffee),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddCoffeeParams {
    pub roastery: String,
    pub icon: String,
    pub farmer: String,
    pub price: i64,
    pub origin: String,
}

async fn rpc_router(req: Request, db: &Pool<Sqlite>) -> impl IntoResponse {
    let value = serde_json::to_value(req)?;
    let method: Methods = serde_json::from_value(value)?;
    tracing::info!("🌸 Received request: {:?}", method);
    let response = match method {
        Methods::AddCoffee(params) => add_coffee(params, db).await.into_response(),
        Methods::GetRandomCoffee => get_random_coffee(db).await.into_response(),
        Methods::GrabId { roastery, origin } => {
            grab_id(&roastery, &origin, db).await.into_response()
        }
        Methods::GrabCoffee(id) => grab_coffee(id, db).await.into_response(),
        Methods::EditCoffee(params) => edit_coffee(params, db).await.into_response(),
    };
    anyhow::Ok(response.body)
}

async fn get_random_coffee(db: &Pool<Sqlite>) -> impl IntoResponse {
    let coffee = query_as!(
        Coffee,
        "SELECT * FROM coffees
        ORDER BY RANDOM()
        LIMIT 1 
        "
    )
    .fetch_one(db)
    .await?;

    anyhow::Ok(coffee)
}

async fn grab_id(roastery: &str, origin: &str, db: &Pool<Sqlite>) -> impl IntoResponse {
    let id = query!(
        "SELECT id FROM coffees WHERE roastery = ? AND origin = ?",
        roastery,
        origin
    )
    .fetch_one(db)
    .await?;
    let id = id.id.ok_or(anyhow::Error::msg("No coffee found"))?;
    anyhow::Ok(id)
}

async fn grab_coffee(id: Nanoid, db: &Pool<Sqlite>) -> impl IntoResponse {
    let id = id.deref();
    let coffee = query_as!(Coffee, "SELECT * FROM coffees WHERE id = ?", id)
        .fetch_one(db)
        .await?;
    anyhow::Ok(coffee)
}

async fn add_coffee(coffee: AddCoffeeParams, db: &Pool<Sqlite>) -> impl IntoResponse {
    let id = Nanoid::new().to_string();
    query!(
        "INSERT INTO coffees (id, roastery, icon, farmer, price, origin) 
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        id,
        coffee.roastery,
        coffee.icon,
        coffee.farmer,
        coffee.price,
        coffee.origin
    )
    .execute(db)
    .await?;
    anyhow::Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditCoffee {
    pub id: Nanoid,
    pub roastery: Option<String>,
    pub icon: Option<String>,
    pub farmer: Option<String>,
    pub price: Option<i64>,
    pub origin: Option<String>,
}

async fn edit_coffee(coffee: EditCoffee, db: &Pool<Sqlite>) -> impl IntoResponse {
    let id = coffee.id.deref();
    let res = query_as!(
        Coffee,
        "UPDATE coffees 
        SET 
            roastery = COALESCE(?, roastery),
            icon = COALESCE(?, icon),
            farmer = COALESCE(?, farmer),
            price = COALESCE(?, price),
            origin = COALESCE(?, origin)
        WHERE id = ?
        RETURNING *",
        coffee.roastery,
        coffee.icon,
        coffee.farmer,
        coffee.price,
        coffee.origin,
        id
    )
    .fetch_one(db)
    .await?;
    anyhow::Ok(res)
}
