mod config;
mod error;
mod handlers;
mod middlewares;
mod models;

use anyhow::Context;
use chat_core::{set_layer, verriy_token, DecodingKey, EncodingKey, TokenVeirfy, User};
use handlers::*;
use middlewares::verify_chat;
use sqlx::PgPool;
use std::{fmt, ops::Deref, sync::Arc};
use tokio::fs;

use axum::{
    middleware::from_fn_with_state,
    routing::{get, post},
    Router,
};
pub use config::AppConfig;
pub use error::*;
pub use models::*;

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

impl TokenVeirfy for AppState {
    type Err = AppError;

    fn verify(&self, token: &str) -> Result<User, Self::Err> {
        Ok(self.dk.verify(token)?)
    }
}

pub async fn get_router(config: AppConfig) -> Result<Router, AppError> {
    let state = AppState::try_new(config).await?;
    let chat = Router::new()
        .route(
            "/:id",
            get(get_chat_handler)
                .patch(update_chat_handler)
                .delete(delete_chat_handler)
                .post(send_message_handler),
        )
        .route("/:id/messages", get(list_message_handler))
        .layer(from_fn_with_state(state.clone(), verify_chat))
        .route("/", get(list_chat_handler).post(create_chat_handler));

    let api = Router::new()
        .route("/users", get(list_chat_users_handler))
        .nest("/chats", chat)
        .route("/upload", post(upload_handler))
        .route("/files/:ws_id/*path", get(download_file_handler))
        .layer(from_fn_with_state(state.clone(), verriy_token::<AppState>))
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
        fs::create_dir_all(&config.server.base_dir)
            .await
            .context("create base_dir failed")?;
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
        pub async fn new_for_test() -> Result<(TestPg, Self), AppError> {
            let config = AppConfig::load()?;
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

            ts.execute(s).await.expect("execute sql failed");
        }
        ts.commit().await.expect("commit transaction failed");
        (tdb, pool)
    }
}
