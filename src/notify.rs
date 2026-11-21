//! Desktop notifications for order status changes.

use crate::api::{self, ApiContext};
use crate::ui;
use colored::*;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const POLL_INTERVAL: Duration = Duration::from_secs(30);
const MAX_WATCH: Duration = Duration::from_secs(4 * 60 * 60);
/// Hidden subcommand used to re-exec the binary as a detached watcher.
pub const WATCH_SUBCOMMAND: &str = "__watch";

/// Best-effort desktop notification; silently does nothing if unsupported.
pub fn send(title: &str, body: &str) {
    #[cfg(target_os = "linux")]
    let command = Command::new("notify-send")
        .args(["--app-name=drpizza", title, body])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    #[cfg(target_os = "macos")]
    let command = Command::new("osascript")
        .arg("-e")
        .arg(format!(
            "display notification {} with title {}",
            applescript_string(body),
            applescript_string(title)
        ))
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    let _ = command;
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    let _ = (title, body);
}

#[cfg(target_os = "macos")]
fn applescript_string(text: &str) -> String {
    format!("\"{}\"", text.replace('\\', "\\\\").replace('"', "\\\""))
}

pub fn status_message(status: &str) -> String {
    match status {
        "confirmed" => "✅ Confirmado pela loja".to_string(),
        "released" => "🛵 Saiu para entrega!".to_string(),
        "concluded" | "closed" => "🍕 Pedido concluído. Bom apetite!".to_string(),
        "cancelled" => "❌ Pedido cancelado".to_string(),
        "waiting_confirmation" => "💳 Pagamento confirmado, aguardando a loja".to_string(),
        other => format!("📌 {}", ui::translate_status(other)),
    }
}

fn is_final(status: &str) -> bool {
    matches!(status, "concluded" | "closed" | "cancelled")
}

/// Re-execs this binary as a detached background watcher, so the terminal is
/// released. It only reads order status and exits on a final status or after
/// `MAX_WATCH`.
pub fn spawn_background_watcher(
    ctx: &ApiContext,
    order_uid: &str,
    order_number: u64,
    status: &str,
) -> std::io::Result<()> {
    let exe = std::env::current_exe()?;
    let mut command = Command::new(exe);
    command
        .args([
            WATCH_SUBCOMMAND,
            order_uid,
            &order_number.to_string(),
            &ctx.company_id.to_string(),
            &ctx.company_slug,
            status,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(unix)]
    {
        // Own process group: Ctrl+C in the terminal won't kill the watcher.
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    command.spawn().map(|_| ())
}

/// Polls the order and notifies on every status change. With `print`, also
/// logs changes to the terminal (foreground `status --watch`).
pub async fn watch_order(
    ctx: &ApiContext,
    order_uid: &str,
    order_number: u64,
    initial_status: &str,
    notify: bool,
    print: bool,
) {
    let title = format!("Dr. Pizza – Pedido #{}", order_number);
    let started = Instant::now();
    let mut last_status = initial_status.to_string();

    while !is_final(&last_status) && started.elapsed() < MAX_WATCH {
        tokio::time::sleep(POLL_INTERVAL).await;
        let Ok(detail) = api::fetch_order_detail(ctx, order_uid).await else {
            continue;
        };
        if detail.status == last_status {
            continue;
        }
        let message = status_message(&detail.status);
        if notify {
            send(&title, &message);
        }
        if print {
            println!(
                "{} {}",
                chrono::Local::now()
                    .format("%H:%M")
                    .to_string()
                    .bright_black(),
                message.bold()
            );
        }
        last_status = detail.status;
    }
}

#[cfg(test)]
mod tests {
    use super::{is_final, status_message};

    #[test]
    fn status_messages_cover_delivery_and_final_states() {
        assert_eq!(status_message("released"), "🛵 Saiu para entrega!");
        assert!(status_message("pending_online_payment").contains("Aguardando pagamento"));
        assert!(is_final("concluded") && is_final("cancelled") && !is_final("released"));
    }
}
