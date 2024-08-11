mod config;
mod error;
mod handlers;
mod middlewares;
mod models;
mod utils;

use anyhow::Context;
use handlers::*;
use middlewares::{set_layer, verriy_token};
use sqlx::PgPool;
use std::{fmt, ops::Deref, sync::Arc};
use utils::{DecodingKey, EncodingKey};

use axum::{
    middleware::from_fn_with_state,
    routing::{get, patch, post},
    Router,
};
pub use config::AppConfig;
pub use error::*;
pub use models::User;

#[derive(Debug, Clone)]
pub(crate) struct AppState {
    pub(crate) inner: Arc<AppStateInner>,
}

#[allow(unused)]
pub(crate) struct AppStateInner {
    pub(crate) config: AppConfig,
    pub(crate) dk: DecodingKey,
    pub(crate) ek: EncodingKey,
    pub(crate) pool: PgPool,
}

pub async fn get_router(config: AppConfig) -> Result<Router, AppError> {
    let state = AppState::try_new(config).await?;

    let api = Router::new()
        .route("/users", get(list_chat_users_handler))
        .route("/chat", get(list_chat_handler).post(create_chat_handler))
        .route(
            "/chat/:id",
            patch(update_chat_handler)
                .delete(delete_chat_handler)
                .post(send_message_handler),
        )
        .route("/chat/:id/messages", get(list_message_handler))
        .layer(from_fn_with_state(state.clone(), verriy_token))
        // routes doesn't need token verification
        .route("/signin", post(signin_handler))
        .route("/signup", post(signup_handler));

    let app = Router::new()
        .route("/", get(index_handler))
        .nest("/api", api)
        .with_state(state);

    Ok(set_layer(app))
}

// 当调用 state.config => state.inner.config
impl Deref for AppState {
    type Target = AppStateInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl AppState {
    pub async fn try_new(config: AppConfig) -> Result<Self, AppError> {
        let dk = DecodingKey::load(&config.auth.pk).context("load pk failed")?;
        let ek = EncodingKey::load(&config.auth.sk).context("load sk failed")?;
        let pool = PgPool::connect(&config.server.db_url)
            .await
            .context("connect to db failed")?;
        Ok(Self {
            inner: Arc::new(AppStateInner {
                config,
                ek,
                dk,
                pool,
            }),
        })
    }
}

impl fmt::Debug for AppStateInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppStateInner")
            .field("config", &self.config)
            .finish()
    }
}

#[cfg(test)]
mod test_util {
    use super::*;
    use sqlx_db_tester::TestPg;

    impl AppState {
        pub async fn new_for_test(config: AppConfig) -> Result<(TestPg, Self), AppError> {
            let dk = DecodingKey::load(&config.auth.pk).context("load pk failed")?;
            let ek = EncodingKey::load(&config.auth.sk).context("load sk failed")?;
            let post = config.server.db_url.rfind('/').expect("invalid db_url");
            let server_url = &config.server.db_url[..post];
            println!("db_url: {}", config.server.db_url);
            println!("server_url: {}", server_url);
            let (tdb, pool) = get_test_pool(Some(server_url)).await;
            Ok((
                tdb,
                Self {
                    inner: Arc::new(AppStateInner {
                        config,
                        ek,
                        dk,
                        pool,
                    }),
                },
            ))
        }
    }

    #[cfg(test)]
    pub async fn get_test_pool(url: Option<&str>) -> (TestPg, PgPool) {
        use sqlx::Executor;

        let url = match url {
            Some(url) => url.to_string(),
            None => "postgres://postgres:root@localhost:5432".to_string(),
        };

        let tdb = TestPg::new(url.to_string(), std::path::Path::new("../migrations"));

        let pool = tdb.get_pool().await;

        // run prepare sql to insert test data
        let sql = include_str!("../fixtures/test.sql").split(";");
        let mut ts = pool.begin().await.expect("begin transaction failed");
        for s in sql {
            if s.trim().is_empty() {
                continue;
            }
            println!("{}", s);
            ts.execute(s).await.expect("execute sql failed");
        }
        ts.commit().await.expect("commit transaction failed");
        (tdb, pool)
    }
}
