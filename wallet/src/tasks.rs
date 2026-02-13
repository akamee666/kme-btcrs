use crate::{Core, Transaction};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::time;
use tokio::time::Duration;
use tracing::*;

pub async fn update_utxos(core: Arc<Core>) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(20));
        loop {
            interval.tick().await;
            if let Err(e) = core.fetch_utxos().await {
                error!("Failed to update UTXOS: {e}");
            }
        }
    })
}

pub async fn handle_transactions(
    rx: kanal::AsyncReceiver<Transaction>,
    core: Arc<Core>,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        while let Ok(transaction) = rx.recv().await {
            if let Err(e) = core.send_transaction(transaction).await {
                error!("Failed to send transaction: {}", e);
            }
        }
    })
}

pub async fn ui_task(core: Arc<Core>) -> JoinHandle<()> {
    tokio::task::spawn_blocking(move || {
        info!("Running UI");
        if let Err(e) = run_ui(core) {
            error!("UI ended with error: {e}");
        }
    })
}
