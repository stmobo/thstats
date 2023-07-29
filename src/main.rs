use std::env;
use std::time::Duration;

use sqlx::sqlite::SqlitePool;
use th07::Touhou7;
use tokio::sync::oneshot;
use tokio::time::interval;
use types::Game;

pub mod db;
pub mod decompress;
pub mod types;

pub mod th07;

use db::{CardAttemptInfo, CardSnapshot, SnapshotStream, UpdateStream};

pub async fn display_card_stats<G: Game>(
    pool: &SqlitePool,
    snapshot: &CardSnapshot<G>,
    attempt_info: Option<&CardAttemptInfo>,
) -> anyhow::Result<()> {
    let title = format!("#{:03} {}", snapshot.card_id(), snapshot.card_name());
    let update_status = (
        attempt_info.map(|a| a.is_capture()).unwrap_or(false),
        attempt_info.is_some(),
    );

    let capture_status = match update_status {
        (true, true) => " - CAPTURE",
        (false, true) => " - MISS",
        _ => "",
    };

    print!(
        "{:^85} [{:<8}]: {:>4} / {:<4} ({:^5.1}%",
        title,
        snapshot.shot_type.to_string(),
        snapshot.captures,
        snapshot.attempts,
        ((snapshot.captures as f64) / (snapshot.attempts as f64)) * 100.0
    );

    let recent_cutoff = snapshot.timestamp.saturating_sub(time::Duration::hours(12));
    let prev_snap: Option<CardSnapshot<G>> = CardSnapshot::get_first_snapshot_after(
        pool,
        snapshot.card,
        snapshot.shot_type,
        recent_cutoff,
    )
    .await?;

    if let Some(prev_snap) = prev_snap {
        let d_attempts = snapshot.attempts.saturating_sub(prev_snap.attempts);
        let d_captures = snapshot
            .captures
            .saturating_sub(prev_snap.captures)
            .min(d_attempts);

        if d_attempts > 0 {
            print!(
                ", recent {} / {} = {:^5.1}%",
                d_captures,
                d_attempts,
                ((d_captures as f64) / (d_attempts as f64)) * 100.0
            );
        }
    }

    println!("){}", capture_status);

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let pool =
        SqlitePool::connect(&env::var("DATABASE_URL").unwrap_or(String::from("sqlite:touhou.db")))
            .await?;

    let (exit_tx, mut exit_rx) = oneshot::channel();
    let mut ctrl_c_handle = tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        exit_tx.send(()).unwrap();
    });

    let mut snap_stream: SnapshotStream<Touhou7> = tokio::select! {
        s = SnapshotStream::new() => s?,
        _ = &mut exit_rx => {
            return Ok(())
        }
    };

    let prev_snapshot = snap_stream.read_snapshot_data().await?;
    for card_snapshot in prev_snapshot.iter_cards() {
        display_card_stats(&pool, card_snapshot, None).await?;
    }
    prev_snapshot.insert(&pool).await?;

    let mut interval = interval(Duration::from_millis(1000));
    let mut update_stream = UpdateStream::new(prev_snapshot);

    while !ctrl_c_handle.is_finished() {
        let f = async {
            interval.tick().await;
            snap_stream.refresh_snapshots().await
        };

        let new_snapshot = tokio::select! {
            s = f => s?,
            _ = &mut ctrl_c_handle => {
                println!("Ctrl-C received, exiting...");
                break;
            }
        };

        if let Some(update) =
            new_snapshot.and_then(|new_snapshot| update_stream.update(new_snapshot))
        {
            for event in update.events() {
                print!(
                    "\n[{}] {} {} {}",
                    event.timestamp(),
                    event.shot_type(),
                    event.stage(),
                    event.difficulty()
                );
                if let Some(practice_no) = event.practice_no() {
                    print!(" Practice #{}", practice_no);
                }
                println!(":");

                for (card_id, attempt_info) in event.attempted_cards() {
                    let new_card_snapshot = update
                        .cur_snapshot()
                        .get_card(event.shot_type(), card_id)
                        .unwrap();
                    display_card_stats(&pool, new_card_snapshot, Some(attempt_info)).await?;
                }
            }

            update_stream.cur_snapshot().insert(&pool).await?;
        }
    }

    pool.close().await;

    Ok(())
}
