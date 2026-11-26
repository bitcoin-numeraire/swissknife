use std::{sync::Arc, time::Duration};

use crate::application::composition::AppServices;

pub fn start_payment_workers(services: Arc<AppServices>) {
    // An attempt notification can precede the provider's terminal aggregate state.
    // Keep recovery independent of the workers that may be occupied sending held payments.
    let recovery = services.clone();
    tokio::spawn(async move {
        loop {
            // Startup already reconciles. Delay from the end of each pass so slow nodes never accumulate ticks.
            tokio::time::sleep(Duration::from_secs(30)).await;
            if let Err(error) = recovery.payment.sync().await {
                tracing::warn!(%error, "Pending Lightning payment reconciliation will retry");
            }
        }
    });

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
