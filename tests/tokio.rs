//! Test with tokio runtime

use tokio::{sync::OnceCell, time::sleep};
use tracing::{debug, info};
use tracing_ext::sub::PrettyConsoleLayer;
use tracing_subscriber::{prelude::*, util::SubscriberInitExt, EnvFilter};

/// Keep track of tests initialization
static INIT: OnceCell<()> = OnceCell::const_new();

/// Initializes the tests
async fn init() {
    INIT.get_or_init(|| async {
        let layer_filter = EnvFilter::from_default_env();
        let layer_console = PrettyConsoleLayer::default()
            .wrapped(true)
            .oneline(true)
            .events_only(true)
            .show_time(false)
            .show_target(false)
            .show_file_info(false)
            .show_span_info(false)
            .indent(6);

        tracing_subscriber::registry()
            .with(layer_console)
            .with(layer_filter)
            .init();
    })
    .await;
}

#[derive(Debug)]
struct Struct1 {
    _field1: String,
    _field2: u8,
}

#[tracing::instrument]
async fn do_task_1(a: u8, b: u8) {
    let c = a + b;
    info!(c, "inside task 1");
    do_task_nested(c).await;
}

#[tracing::instrument]
async fn do_task_nested(x: u8) {
    let test_struct = Struct1 {
        _field1: "test_field1".to_string(),
        _field2: 2,
    };
    debug!(?test_struct, "inside do_task_nested()");
}

#[tracing::instrument]
async fn do_task_2(x: u8) {
    debug!("inside task 2");
}

#[tokio::test]
async fn test_tokio() {
    init().await;

    info!(field1 = "Field 1", "Test initialized");

    let handle_1 = tokio::spawn(async move {
        sleep(std::time::Duration::from_millis(100)).await;
        do_task_1(1, 2).await;
    });

    let handle_2 = tokio::spawn(async move {
        do_task_2(10).await;
    });

    let (_, _) = tokio::join!(handle_1, handle_2);
    info!("Test OK");
}
