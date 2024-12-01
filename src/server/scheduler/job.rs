use crate::server::database::pool::GenericRow;
use crate::server::database::pool::DbClient;
use std::time::Duration;
use log::{error, info};
use tokio::{pin, select, time};
use tokio_util::sync::CancellationToken;
use tokio_util::task::task_tracker;
use crate::server::database::pool::{Init, Pool};

const DEFAULT_DB_WRITE_POOL_CONN_STR: &str = "postgresql://postgres:pass@localhost";

async fn worker(cancel_token: CancellationToken) {
    let mut write_pool = Pool::new().await.unwrap();
    write_pool.init(DEFAULT_DB_WRITE_POOL_CONN_STR.to_string()).await.ok();
    let interval = time::interval(Duration::from_mins(1_u64)); // run once every minute
    pin!(interval);
    loop {
        select! {
            _ = interval.tick() => {},
            _ = cancel_token.cancelled() => {
                info!("received cancel signal, returning gracefully");
                return;
            }
        }
        
        let local_conn = write_pool.acquire().await.unwrap();
        let client = local_conn.client.as_ref().unwrap();
        let ids = match client.query(r#"
                SELECT id
                FROM bill_item
                WHERE state = 'created'
                AND date_add(created_at, make_interval(mins := time_to_deliver)) <= CURRENT_TIMESTAMP
                LIMIT 10
            "#, &[]).await {
            Ok(rows) => rows.iter().map(|row| row.get("id")).collect::<Vec<i64>>(),
            Err(e) => {
                error!("failed to query delivered items, {:?}", e.code().unwrap());
                vec![]
            }
        };
        
        if ids.is_empty() {
            continue;
        }
        
        let stmt = format!(r#"
                UPDATE "bill_item"
                SET state = 'delivered'
                WHERE state = 'created' AND id IN ({})
                RETURNING id
            "#, ids.iter().map(|id| id.to_string()).collect::<Vec<String>>().join(","));
        
        match client.query(&stmt, &[]).await {
            Ok(rows) => {
                info!("marked bill items {:?} as delivered", rows.iter().map(|row| row.get::<&str, i64>("id")).collect::<Vec<_>>());
            },
            Err(e) => {
                error!("failed to mark some bill items as delivered, {:?}", e.code().unwrap());
            }
        };
    }
}

pub async fn bill_item_sweeper(cancel_token: CancellationToken) {
    let tracker = task_tracker::TaskTracker::new();
    tracker.spawn(worker(cancel_token));
    if tracker.close() {
        tracker.wait().await;
    }
}