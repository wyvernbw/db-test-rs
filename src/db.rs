use std::ops::Deref;

use serde::{Deserialize, Serialize};
use sqlx::{query, SqliteConnection};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nanoid(String);

impl Deref for Nanoid {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Nanoid {
    pub fn new() -> Self {
        Self(nanoid::nanoid!())
    }
}

impl Default for Nanoid {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Option<String>> for Nanoid {
    fn from(opt: Option<String>) -> Self {
        opt.map_or_else(Nanoid::new, |s| Nanoid(s))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coffee {
    pub id: Nanoid,
    pub roastery: String,
    pub icon: String,
    pub farmer: String,
    pub price: i64,
    pub origin: String,
}

pub async fn initialize_db(db: &mut SqliteConnection) -> anyhow::Result<()> {
    query!(
        "CREATE TABLE IF NOT EXISTS coffees (
            id TEXT PRIMARY KEY UNIQUE,
            roastery TEXT NOT NULL,
            icon TEXT NOT NULL,
            farmer TEXT NOT NULL,
            price INTEGER NOT NULL,
            origin TEXT NOT NULL
        )"
    )
    .execute(db)
    .await?;
    Ok(())
}
