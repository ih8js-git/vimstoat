use log::{error, warn};
use notify_rust::{Notification, ResponseHandler, Timeout};
use std::{fs, path::PathBuf, thread};

use crate::Result;

pub const ICON_FILE: &str = "icon"; // since idk which file extention we'll be using, I'm leaving it without
pub const NOTIFICATION_TIMEOUT: u32 = 10_000;

fn get_icon_path() -> Option<PathBuf> {
    let mut icon_path = dirs::data_dir()?;
    icon_path.push(env!("CARGO_PKG_NAME"));

    if let Err(e) = fs::create_dir_all(&icon_path) {
        error!("Error creating data directory: {e:?}");
        return None;
    };

    icon_path.push(ICON_FILE);
    Some(icon_path)
}

pub struct NotifyHandler {
    icon_path: Option<PathBuf>,
}

impl NotifyHandler {
    pub async fn new() -> Result<Self> {
        let icon_path = get_icon_path();
        if icon_path.is_none() {
            warn!("Failed to find icon path!");
        }

        Ok(Self { icon_path })
    }

    pub fn clone_icon_path(&self) -> String {
        self.icon_path
            .clone()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    }

    pub async fn send_notification(
        &self,
        title: String,
        body: String,
        icon: String,
        actions: Vec<(String, String)>,
        response: impl ResponseHandler + Send + 'static,
    ) {
        thread::spawn(move || {
            let mut notification = Notification::new()
                .appname(env!("CARGO_PKG_NAME"))
                .summary(&title)
                .body(&body)
                .timeout(Timeout::Milliseconds(NOTIFICATION_TIMEOUT))
                .icon(&icon)
                .clone();

            for action in actions {
                notification.action(&action.0, &action.1);
            }

            let res = notification.show();
            match res {
                Ok(handle) => {
                    if let Err(e) = handle.wait_for_response(response) {
                        error!("Error fetching notification response: {e:?}");
                    };
                }
                Err(e) => {
                    error!("Error sending notification: {e:?}");
                }
            }
        });
    }
}
