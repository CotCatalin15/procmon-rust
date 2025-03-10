#![allow(internal_features)]
#![feature(core_intrinsics)]

mod fake_communication;

use fake_communication::FakeCommunication;
use kmum_common::KmMessage;
use rusqlite::Connection;
use tokio::{sync::mpsc::Receiver, task::block_in_place};
use tracing::info;

#[tokio::main]
async fn main() {
    std::panic::set_hook(Box::new(|_info| core::intrinsics::breakpoint()));

    let sub = tracing_subscriber::fmt()
        .with_ansi(false) // Disable ANSI color codes
        .finish();
    tracing::subscriber::set_global_default(sub).expect("Failed to sent global tracing subscriber");

    info!("Starting client");

    /*
       handles.spawn(async {
                   let mut file = tokio::fs::File::open("test.txt").await.unwrap();

                   let mut msg = [0u8; 1024];
                   file.write_all("Hello".as_bytes()).await.unwrap();
                   file.read(&mut msg).await.unwrap();

                   drop(file);
               });
    */

    procmon_core::runtime::ProcmonRuntime::new(FakeCommunication::new())
        .run(|receiver| {
            let mut connection = Connection::open("D:\\Procmondb\\events.db").unwrap();
            create_table(&mut connection);

            async move {
                event_receiver(connection, receiver).await;
            }
        })
        .await;
}

async fn event_receiver(mut connection: Connection, mut receiver: Receiver<KmMessage>) {
    let mut events = Vec::new();

    loop {
        let size = receiver.recv_many(&mut events, 256).await;
        if size == 0 {
            break;
        }

        block_in_place(|| commit_events(&mut connection, &events[..size]));
    }
}

fn create_table(conn: &mut Connection) {
    conn.execute("DROP TABLE IF EXISTS events", []).unwrap();

    conn.execute(
        "CREATE TABLE IF NOT EXISTS events (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp INTEGER NOT NULL,
            pid INTEGER NOT NULL,
            uid INTEGER NOT NULL,
            tid TEXT NOT NULL,
            path BLOB,
            operation TEXT NOT NULL,
            additional_data TEXT
        )",
        [],
    )
    .unwrap();

    // Create an index on timestamp for ordered access
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events (timestamp)",
        [],
    )
    .unwrap();
}

fn commit_events(conn: &mut Connection, events: &[KmMessage]) {
    tracing::info!("Comming events to sql table {}", events.len());

    let transcation = conn.transaction().unwrap();

    //
    let mut stmt = transcation
        .prepare("INSERT INTO events(timestamp, pid, uid, tid, path, operation, additional_data) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)")
        .unwrap();

    for event in events {
        let path_blob = if event.event.path.is_empty() {
            None
        } else {
            Some(from_u16(event.event.path.0.as_slice()))
        };

        stmt.execute(rusqlite::params![
            event.event.date as i64,
            event.process.pid as i64,
            event.process.unique_id as i64,
            event.event.thread as i64,
            path_blob,
            "MY OPERATION",
            "Other stuff"
        ])
        .unwrap();
    }

    drop(stmt);
    transcation.commit().unwrap();
}

fn from_u16(from: &[u16]) -> &[u8] {
    let len = from.len().checked_mul(2).unwrap();
    let ptr: *const u8 = from.as_ptr().cast();
    unsafe { std::slice::from_raw_parts(ptr, len) }
}
