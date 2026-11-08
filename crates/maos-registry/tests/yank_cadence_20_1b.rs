//! Required real-process 30-second floor, isolated from the default 30-second binaries.
#[path = "../../../tests/harness/registry_20_1.rs"]
mod support;
#[path = "../../../tests/harness/registry_trust_20_1b.rs"]
mod trust;
use maos_domain::ports::registry::{RegistryError, YankEntry, YankList, YankOrigin};
use maos_registry::yank::{YankObserver, YankPoller, YankSource};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};
struct Source {
    client: maos_registry::McpSpiritRegistryClient,
    polls: AtomicUsize,
}
impl YankSource for Source {
    fn fetch_yanks(&self, cursor: u64) -> Result<YankList, RegistryError> {
        let result = self.client.yanks_since(cursor);
        self.polls.fetch_add(1, Ordering::SeqCst);
        result
    }
}
struct Observer(AtomicBool, String);
impl YankObserver for Observer {
    fn on_yank(
        &self,
        entry: &YankEntry,
    ) -> Result<maos_registry::yank::YankDisposition, RegistryError> {
        maos_registry::yank_auth::verify_entry(
            entry,
            &trust::public(&trust::PUBLISHER),
            None,
            &self.1,
            maos_registry::yank_auth::now_unix_ms().map_err(RegistryError::Transport)?,
        )
        .map_err(RegistryError::Transport)?;
        self.0.store(true, Ordering::SeqCst);
        Ok(maos_registry::yank::YankDisposition::Verified)
    }
}
#[tokio::test]
async fn signed_yank_crosses_real_server_at_30_second_floor_ac7() {
    let scratch = support::Scratch::new();
    std::env::set_var(
        "MAOS_REGISTRY_YANK_CURSOR_PATH",
        scratch.0.join("cursor.json"),
    );
    std::env::set_var("MAOS_REGISTRY_YANK_POLL_INTERVAL_S", "1");
    let server = support::Server::start(&scratch, &support::sibling("maos-registry-server"));
    assert!(support::rpc(
        &server.addr,
        "registry.publish",
        serde_json::to_value(trust::package("cadence", "1.0.0", &trust::PUBLISHER)).unwrap()
    )
    .get("result")
    .is_some());
    let source = Arc::new(Source {
        client: maos_registry::McpSpiritRegistryClient::http(
            format!("http://{}/mcp", server.addr),
            maos_registry::client::RegistryClientConfig {
                security: maos_registry::security::RegistrySecurityConfig::development_loopback(),
                ..Default::default()
            },
        )
        .unwrap(),
        polls: AtomicUsize::new(0),
    });
    let observer = Arc::new(Observer(
        AtomicBool::new(false),
        format!("http://{}/mcp", server.addr),
    ));
    let (stop, stopped) = tokio::sync::watch::channel(false);
    let interval = maos_registry::yank::resolve_poll_interval().unwrap();
    assert_eq!(interval.as_secs(), 30);
    let task = tokio::spawn(maos_registry::yank::yank_poller_production_loop(
        Arc::new(YankPoller::new(
            source.clone(),
            observer.clone(),
            format!("http://{}/mcp", server.addr),
        )),
        stopped,
        interval,
    ));
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while source.polls.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("initial empty poll must complete");
    let start = std::time::Instant::now();
    assert!(support::rpc(
        &server.addr,
        "registry.deprecate",
        trust::yank(
            &server.addr,
            "cadence",
            "1.0.0",
            &trust::PUBLISHER,
            YankOrigin::Publisher
        )
    )
    .get("result")
    .is_some());
    let propagation = tokio::time::timeout(std::time::Duration::from_secs(35), async {
        while !observer.0.load(Ordering::SeqCst) {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await;
    stop.send(true).unwrap();
    task.await.unwrap();
    propagation.expect("signed yank must arrive on the next clamped production poll");
    assert!(start.elapsed().as_secs() <= 35);
    assert_eq!(source.polls.load(Ordering::SeqCst), 2);
    std::env::remove_var("MAOS_REGISTRY_YANK_CURSOR_PATH");
    std::env::remove_var("MAOS_REGISTRY_YANK_POLL_INTERVAL_S");
}
