mod action;
mod api;
mod app;
mod cache;
mod command;
mod input;
mod logger;
mod models;
mod notification;
mod views;

use app::App;

pub type Result<T> = anyhow::Result<T>;

#[tokio::main]
async fn main() -> anyhow::Result<(), Box<dyn std::error::Error>> {
    logger::init()?;

    let mut terminal = views::init_terminal()?;
    let mut app = App::new().await?;

    let run_res = app.run(&mut terminal).await;

    views::restore_terminal()?;
    run_res?;

    Ok(())
}
