use super::*;
use tauri::async_runtime::block_on;

/// Settings live in a `thread_local`, so a choice only reaches the bidders if
/// every command lands on the one engine thread: set one, deal, solve.
#[test]
fn commands_share_one_engine_thread() {
    block_on(async {
        let options = describe_options().await;
        assert!(options.contains("notrump_defense"));
        set_choice("ns".into(), "notrump_defense".into(), "woolsey".into()).await;
        assert!(book("ns".into()).await.len() > 2);

        let snapshot = deal_demo("N".into(), "none".into()).await;
        assert!(snapshot.contains("\"auction\""));
        assert!(dd_table().await.contains("\"rows\""));

        assert_eq!(binky_run(1).await, "null");
        assert!(!binky_create("AKQJ".into(), "".into(), true).await);
    });
}
