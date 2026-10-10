mod action;
mod api;
mod app;
mod cache;
mod command;
mod config;
mod input;
mod logger;
mod models;
mod notification;
#[cfg(test)]
mod test_helpers;
mod views;

use app::App;

pub type Result<T> = anyhow::Result<T>;

#[tokio::main]
async fn main() -> anyhow::Result<(), Box<dyn std::error::Error>> {
    logger::init()?;
    let (config, config_warning) = config::load();

    let mut terminal = views::init_terminal()?;
    let mut app = App::new(config, config_warning).await?;

    let run_res = app.run(&mut terminal).await;

    views::restore_terminal()?;
    run_res?;

    Ok(())
}
