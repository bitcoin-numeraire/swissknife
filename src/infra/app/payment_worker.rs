use std::{sync::Arc, time::Duration};

use crate::application::composition::AppServices;

pub fn start_payment_workers(services: Arc<AppServices>) {
    // Bound simultaneous provider calls so one held invoice cannot block every other sender.
    for _ in 0..4 {
        let services = services.clone();
        tokio::spawn(async move {
            loop {
                match services.payment.execute_next().await {
                    Ok(true) => continue,
                    Ok(false) => {}
                    Err(error) => tracing::warn!(%error, "Payment command execution did not complete"),
                }
                // A local submission wakes a worker immediately. The periodic check recovers
                // work committed by another replica even if it stopped before signaling.
                let _ =
                    tokio::time::timeout(Duration::from_secs(15), services.payment_command_available.notified()).await;
            }
        });
    }
}
