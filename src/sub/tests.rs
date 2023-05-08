//! Subscriber tests

use std::sync::Once;

use tracing::{debug, info, warn};
use tracing_subscriber::{prelude::*, EnvFilter};

use super::pretty::PrettyConsoleLayer;

/// Keep track of tests initialization
static INIT: Once = Once::new();

/// Initializes the tests
fn init() {
    INIT.call_once(|| {
        // let layer_fmt = fmt::layer().with_target(false);
        let layer_filter = EnvFilter::from_default_env();
        let layer_console = PrettyConsoleLayer::default()
            .wrapped(true)
            .oneline(false)
            .events_only(true)
            .show_time(true)
            .show_target(true)
            .show_span_info(true)
            .indent(6);

        tracing_subscriber::registry()
            // .with(layer_fmt)
            .with(layer_console)
            .with(layer_filter)
            .init();
    });
}

#[derive(Debug)]
struct Struct1 {
    _field1: String,
    _field2: u8,
    _field3: Struct2,
}

#[derive(Debug)]
struct Struct2 {
    _field4: String,
    _field5: String,
}

#[tracing::instrument]
fn do_something(a: u8, b: u8) {
    let c = a + b;
    info!(c, "inside do_something()");
    do_something_nested_1(c);
}

#[tracing::instrument]
fn do_something_nested_1(c: u8) {
    let test_struct = Struct1 {
        _field1: "test_field1".to_string(),
        _field2: 2,
        _field3: Struct2 {
            _field4: "test_field4".to_string(),
            _field5: "test_field5".to_string(),
        },
    };
    debug!(?test_struct, "inside do_something_nested_1()");
    do_something_nested_2(c);
}

#[tracing::instrument]
fn do_something_nested_2(c: u8) {
    debug!("inside do_something_nested_2()");
}

#[test]
fn test_simple() {
    init();

    info!(field1 = "Field 1", "Test initialized");
    do_something(1, 2);
    info!("Test OK");
}
