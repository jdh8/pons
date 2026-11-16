//! A desktop client for pons: the web UI in a native window.
//!
//! The page is `web/`'s, unchanged, embedded at build time.  What differs is
//! where the engine runs: `web/app.js` finds `window.__TAURI__` and calls the
//! commands below instead of loading the wasm, so the bidders and the solver
//! run as native code on their own thread and double dummy fans out over
//! `pons-dds`'s pool.  Every command returns the same JSON string its
//! `pons-web` namesake hands the browser.

// No console window beside a release build on Windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use pons_web::{Binky, WebTable};
use std::sync::{LazyLock, mpsc};

/// What the engine thread owns: the table, and the Evaluate tab's shuffler
struct Engine {
    table: WebTable,
    binky: Option<Binky>,
}

type Job = Box<dyn FnOnce(&mut Engine) + Send>;

/// A seed from the clock, in the decimal spelling `pons-web` takes
fn seed() -> String {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos() as u64)
        .to_string()
}

/// The engine thread's inbox.  One thread owns the table *and* the settings
/// (`pons-web` keeps them in a `thread_local`), so every engine call is a job
/// sent here, and the window never waits on a deal, a book build or a solve.
static ENGINE: LazyLock<mpsc::Sender<Job>> = LazyLock::new(|| {
    let (engine, inbox) = mpsc::channel::<Job>();
    std::thread::spawn(move || {
        let mut engine = Engine {
            table: WebTable::new(&seed()),
            binky: None,
        };
        for job in inbox {
            job(&mut engine);
        }
    });
    engine
});

/// Run `job` on the engine thread and wait for its answer
async fn ask<R: Send + 'static>(job: impl FnOnce(&mut Engine) -> R + Send + 'static) -> R {
    let (reply, mut answer) = tauri::async_runtime::channel(1);
    ENGINE
        .send(Box::new(move |engine| {
            // The asker may have gone away (a closed window); nothing to do
            let _ = reply.try_send(job(engine));
        }))
        .expect("the engine thread outlives the window");
    answer.recv().await.expect("the engine thread answers")
}

#[tauri::command]
async fn deal_practice(seat: String, dealer: String, vul: String, min_hcp: u8) -> String {
    ask(move |engine| engine.table.deal_practice(&seat, &dealer, &vul, min_hcp)).await
}

#[tauri::command]
async fn deal_demo(dealer: String, vul: String) -> String {
    ask(move |engine| engine.table.deal_demo(&dealer, &vul)).await
}

#[tauri::command]
async fn deal_pbn(pbn: String, dealer: String, vul: String) -> String {
    ask(move |engine| engine.table.deal_pbn(&pbn, &dealer, &vul)).await
}

#[tauri::command]
async fn bid(call: String) -> String {
    ask(move |engine| engine.table.bid(&call)).await
}

#[tauri::command]
async fn hint() -> String {
    ask(|engine| engine.table.hint()).await
}

#[tauri::command]
async fn dd_table() -> String {
    ask(|engine| engine.table.dd_table()).await
}

#[tauri::command]
async fn oracle(samples: u32) -> String {
    ask(move |engine| engine.table.oracle(samples)).await
}

#[tauri::command]
async fn book(pair: String) -> String {
    ask(move |_| pons_web::book(&pair)).await
}

#[tauri::command]
async fn point_census() -> String {
    ask(|_| pons_web::point_census()).await
}

#[tauri::command]
async fn describe_options() -> String {
    ask(|_| pons_web::describe_options()).await
}

#[tauri::command]
async fn set_option(pair: String, key: String, on: bool) {
    ask(move |_| pons_web::set_option(&pair, &key, on)).await;
}

#[tauri::command]
async fn set_choice(pair: String, key: String, value: String) {
    ask(move |_| pons_web::set_choice(&pair, &key, &value)).await;
}

/// Seat the Evaluate tab's two hands; `false` when they are not two full,
/// disjoint hands
#[tauri::command]
async fn binky_create(north: String, south: String, notrump: bool) -> bool {
    ask(move |engine| {
        engine.binky = Binky::create(&north, &south, notrump, &seed());
        engine.binky.is_some()
    })
    .await
}

/// Solve `samples` more layouts of the seated hands; `"null"` with none seated
#[tauri::command]
async fn binky_run(samples: u32) -> String {
    ask(move |engine| {
        engine
            .binky
            .as_mut()
            .map_or_else(|| "null".to_string(), |binky| binky.run(samples))
    })
    .await
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            deal_practice,
            deal_demo,
            deal_pbn,
            bid,
            hint,
            dd_table,
            oracle,
            book,
            point_census,
            describe_options,
            set_option,
            set_choice,
            binky_create,
            binky_run,
        ])
        .run(tauri::generate_context!())
        .expect("the window opens");
}

#[cfg(test)]
mod tests;
