use std::env;
use crate::server::database::pool::GenericRow;
use crate::server::database::pool::DbClient;
use log::{error, info};
use tokio::{pin, select, time};
use tokio_util::sync::CancellationToken;
use tokio_util::task::task_tracker;
use crate::DEFAULT_DB_WRITE_POOL_CONN_STR;
use crate::server::DB_TIMEOUT_SECONDS;
use crate::server::database::pool::{Init, Pool};

async fn worker(cancel_token: CancellationToken) {
    let mut write_pool = Pool::new().await.unwrap();
    let conn_str = env::var("DB_WRITE_POOL_CONN_STR").unwrap_or(DEFAULT_DB_WRITE_POOL_CONN_STR.to_string());
    write_pool.init(conn_str).await.ok();
    let interval = time::interval(time::Duration::new(60, 0)); // run once every minute
    pin!(interval);
    loop {
        select! {
            _ = interval.tick() => {},
            _ = cancel_token.cancelled() => {
                info!("received cancel signal, returning");
                return;
            }
        }

        let local_conn = write_pool.acquire(DB_TIMEOUT_SECONDS).await.unwrap();
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
            info!("nothing to update, continue to sleep");
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
    loop {
        let tracker = task_tracker::TaskTracker::new();
        tracker.spawn(worker(cancel_token.clone()));
        if tracker.close() {
            tracker.wait().await;
            if cancel_token.is_cancelled() { // stop sweeper only when app is being shutting down 
                break;   
            }
        }
    }
}


#[cfg(test)]
mod test {
    use std::time::Duration;
    use super::*;

    #[tokio::test]
    async fn test_worker() {
        let cancel_token = CancellationToken::new();
        let worker_cancel_token = cancel_token.clone();
        let sleeper_cancel_token = cancel_token.clone();
        let actual = select! {
            _ = worker(worker_cancel_token) => {
                "unexpected"    
            },
            _ = time::sleep(Duration::new(2, 0)) => {
                sleeper_cancel_token.cancel();
                "expected"
            }
        };
        assert_eq!(actual, "expected");
    }
    
    #[tokio::test]
    async fn test_bill_item_sweeper() {
        let cancel_token = CancellationToken::new();
        let sweeper_cancel_token = cancel_token.clone();
        let sleeper_cancel_token = cancel_token.clone();
        let actual = select! {
            _ = bill_item_sweeper(sweeper_cancel_token) => {
                "unexpected"
            },
            _ = time::sleep(Duration::new(2, 0)) => {
                sleeper_cancel_token.cancel();
                "expected"
            }
        };
        assert_eq!(actual, "expected");
    }
}