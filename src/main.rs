mod errors;
mod routes;
mod youtube;

use std::env;

use chrono::Local;
use dotenvy::dotenv;
use fern::colors::{ColoredLevelConfig, Color};
use log::{error, info};
use poem::{Route, listener::TcpListener, Server, EndpointExt, middleware::Cors};
use redis::aio::ConnectionManager;

use crate::routes::session;

fn setup_logger() -> Result<(), fern::InitError> {
    let colors = ColoredLevelConfig::new()
        .debug(Color::BrightBlue)
        .warn(Color::Yellow)
        .error(Color::Red);

    fern::Dispatch::new()
        .format(move |out, message, record| {
            let date = Local::now();

            out.finish(format_args!(
                "{}[{} {} {}] {}\x1B[0m",
                format_args!(
                    "\x1B[{}m",
                    colors.get_color(&record.level()).to_fg_str()
                ),
                date.format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.target(),
                message,
            ))
        })
        .level(log::LevelFilter::Debug)
        .chain(std::io::stdout())
        .apply()?;
    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(why) = setup_logger() {
        eprintln!("Failed to setup logger: {why}");
    }

    dotenv().ok();
    let redis_url = env::var("REDIS").unwrap();

    let client = redis::Client::open(redis_url).unwrap();
    let con = ConnectionManager::new(client).await;

    match con {
        Ok(con) => {
            info!("Connected to Redis");

            let app = Route::new()
                .nest("/session", session::register_routes())
                .data(con)
                .with(Cors::new());

            let server = Server::new(TcpListener::bind("127.0.0.1:3000"))
                .run(app)
                .await;

            if let Err(why) = server {
                error!("Failed to start API: {why}");
            }
        }
        Err(why) => error!("Failed to connect to Redis: {why}"),
    }
}
