use std::{error::Error, future::Future, ops::DerefMut, pin::Pin, process::Output};

use anyhow::Context;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sqlx::{Pool, Sqlite, SqliteConnection, SqlitePool};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use crate::db;

pub struct Response {
    pub status: &'static str,
    pub body: String,
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}

pub mod status {
    use super::Response;

    pub const OK: Response = Response {
        status: "200 OK",
        body: String::new(),
    };

    pub const BAD_REQUEST: Response = Response {
        status: "400 Bad Request",
        body: String::new(),
    };

    pub const INTERNAL_SERVER_ERROR: Response = Response {
        status: "500 Internal Server Error",
        body: String::new(),
    };
}

pub trait IntoResponse: Send + 'static {
    fn into_response(self) -> Response;
}

impl IntoResponse for () {
    fn into_response(self) -> Response {
        Response {
            status: "200 OK".into(),
            body: "{}".into(),
        }
    }
}

impl<T: Serialize + Send + 'static, E: ToString + 'static + Send> IntoResponse for Result<T, E> {
    fn into_response(self) -> Response {
        match self {
            Ok(v) => Response {
                status: "200 OK".into(),
                body: serde_json::to_string(&v).unwrap(),
            },
            Err(err) => Response {
                status: "500 Internal Server Error".into(),
                body: err.to_string(),
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Request {
    pub method: String,
    #[serde(default)]
    pub data: serde_json::Value,
}

fn parse_request(data: &str) -> anyhow::Result<Request> {
    serde_json::from_str(data).context("Invalid Request")
}

pub async fn run_request<Ft, R>(
    handler: impl Fn(Request, &'static Pool<Sqlite>) -> Ft,
    request: Request,
    db: &'static Pool<Sqlite>,
) -> R
where
    Ft: Future<Output = R> + Send + 'static,
    R: IntoResponse,
{
    handler(request, db).await
}

pub async fn server_loop<H, Ft, R>(handler_fn: H) -> anyhow::Result<()>
where
    H: Send + Clone + 'static + Fn(Request, &'static Pool<Sqlite>) -> Ft,
    Ft: Future<Output = R> + Send + 'static,
    R: IntoResponse,
{
    let pool = SqlitePool::connect("sqlite://./coffee.db").await?;
    {
        let mut conn = pool.acquire().await?;
        db::initialize_db(conn.deref_mut()).await?;
    }
    let db: &Pool<_> = Box::leak(Box::new(pool.clone()));
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    tracing::info!("🔥 Listening on {}", listener.local_addr()?);
    loop {
        let (socket, _) = listener.accept().await?;
        let handler = handler_fn.clone();
        tokio::spawn(async move {
            let result = handle_connection(handler, socket, db).await;
            if let Err(err) = result {
                tracing::error!("Error: {err}");
            }
        });
    }
}

async fn handle_connection<H, Ft, R>(
    handler_fn: H,
    mut socket: TcpStream,
    pool: &'static SqlitePool,
) -> anyhow::Result<()>
where
    H: Send + Clone + 'static + Fn(Request, &'static Pool<Sqlite>) -> Ft,
    Ft: Future<Output = R> + Send + 'static,
    R: IntoResponse,
{
    let mut data = [0u8; 1024];
    async fn send_response(socket: &mut TcpStream, res: Response) -> anyhow::Result<()> {
        let res = response(res)?;
        socket.write_all(res.as_bytes()).await?;
        socket.flush().await?;
        tracing::info!("🌸 Sent response");
        Ok(())
    }

    loop {
        let len = socket.read(&mut data).await?;
        if len == 0 {
            break;
        }
        let req = data[..len].to_vec();
        let Ok(req) = String::from_utf8(req) else {
            tracing::error!("Invalid UTF-8 request");
            send_response(&mut socket, status::BAD_REQUEST).await?;
            continue;
        };
        let Some(body) = req.split("\r\n\r\n").nth(1) else {
            send_response(&mut socket, status::BAD_REQUEST).await?;
            tracing::error!("Invalid request");
            continue;
        };
        let res = run_request(handler_fn.clone(), parse_request(body)?, pool).await;
        send_response(&mut socket, res.into_response()).await?;
    }
    Ok(())
}

fn response(res: Response) -> anyhow::Result<String> {
    let content_length = res.body.len();
    let status = res.status;

    let http_response = [
        &format!("HTTP/1.1 {status}\r\n"),
        "Content-Type: application/json\r\n",
        &format!("Content-Length: {content_length}\r\n\r\n"),
        &res.body,
    ];
    let http_response = http_response.join("");
    Ok(http_response)
}
