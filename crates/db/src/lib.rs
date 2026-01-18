use std::{env, sync::Arc};

use sqlx::{
    Error, PgPool,
    postgres::{PgConnection, PgPoolOptions},
};

pub mod models;
pub mod serde_helpers;

#[derive(Clone)]
pub struct DBService {
    pub pool: PgPool,
}

impl DBService {
    /// Create a new DBService connecting to PostgreSQL.
    /// Uses DATABASE_URL environment variable.
    pub async fn new() -> Result<DBService, Error> {
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/vibe_kanban".to_string());
        let pool = PgPool::connect(&database_url).await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(DBService { pool })
    }

    pub async fn new_with_after_connect<F>(after_connect: F) -> Result<DBService, Error>
    where
        F: for<'a> Fn(
                &'a mut PgConnection,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<(), Error>> + Send + 'a>,
            > + Send
            + Sync
            + 'static,
    {
        let pool = Self::create_pool(Some(Arc::new(after_connect))).await?;
        Ok(DBService { pool })
    }

    async fn create_pool<F>(after_connect: Option<Arc<F>>) -> Result<PgPool, Error>
    where
        F: for<'a> Fn(
                &'a mut PgConnection,
            ) -> std::pin::Pin<
                Box<dyn std::future::Future<Output = Result<(), Error>> + Send + 'a>,
            > + Send
            + Sync
            + 'static,
    {
        let database_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/vibe_kanban".to_string());

        let pool = if let Some(hook) = after_connect {
            PgPoolOptions::new()
                .after_connect(move |conn, _meta| {
                    let hook = hook.clone();
                    Box::pin(async move {
                        hook(conn).await?;
                        Ok(())
                    })
                })
                .connect(&database_url)
                .await?
        } else {
            PgPool::connect(&database_url).await?
        };

        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(pool)
    }
}
