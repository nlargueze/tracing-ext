//! Extensions for subscribers
//!
//! This module provides utilities for subscribers

use std::{collections::HashMap, time::Instant};

use tracing_subscriber::registry::SpanRef;

pub mod pretty;

#[cfg(test)]
pub mod tests;

/// Trait for a span extension
pub trait SpanExtension {
    /// Initializes the extension
    fn init<S>(span_ref: &SpanRef<S>)
    where
        S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
        Self: Default + Send + Sync + 'static,
    {
        let mut extensions = span_ref.extensions_mut();
        if extensions.get_mut::<Self>().is_none() {
            let ext = Self::default();
            extensions.insert(ext);
        }
    }

    /// Records the span attributes for the extension
    fn record_attrs<S>(span_ref: &SpanRef<S>, attrs: &tracing::span::Attributes<'_>)
    where
        S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
        Self: tracing::field::Visit + Sized + 'static,
    {
        let mut extensions = span_ref.extensions_mut();
        let ext = extensions
            .get_mut::<Self>()
            .expect("Extension not initialized");
        attrs.record(ext);
    }
}

/// A span extension to record the span attributes
#[derive(Debug, Default)]
pub struct SpanExtAttrs {
    /// Attributes values
    attrs: HashMap<&'static str, String>,
}

impl SpanExtension for SpanExtAttrs {}

impl tracing::field::Visit for SpanExtAttrs {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let value = format!("{value:?}");
        self.attrs.insert(field.name(), value);
    }
}

/// A span extensison to record timing info
#[derive(Debug)]
pub struct SpanExtTiming {
    /// Instant when the span was entered
    pub entered: Instant,
}

impl Default for SpanExtTiming {
    fn default() -> Self {
        Self {
            entered: Instant::now(),
        }
    }
}

impl SpanExtension for SpanExtTiming {}

/// A visitor for events
///
/// The visitor saves the event data
#[derive(Debug, Default)]
pub struct EventVisitor {
    /// Fields
    fields: HashMap<&'static str, String>,
}

impl tracing::field::Visit for EventVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let value_str = format!("{value:?}");
        self.fields.insert(field.name(), value_str);
    }
}

impl EventVisitor {
    /// Records an event fields
    ///
    /// Returns the event message and the event fields
    pub fn record_event(event: &tracing::Event) -> Self {
        let mut f_visitor = EventVisitor::default();
        event.record(&mut f_visitor);
        f_visitor
    }

    /// Returns the event message
    pub fn message(&self) -> &str {
        match self.fields.get("message") {
            Some(s) => s,
            None => {
                panic!("Event message not found")
            }
        }
    }

    /// Returns the event fields (exc. message)
    pub fn meta_fields(&self) -> HashMap<&'static str, &str> {
        self.fields
            .iter()
            .filter_map(|(k, v)| {
                if *k == "message" {
                    return None;
                }
                Some((*k, v.as_str()))
            })
            .collect()
    }
}
