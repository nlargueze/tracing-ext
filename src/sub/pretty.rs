//! A pretty tracing layer for console printing

use std::fmt::Write;

use colored::Colorize;
use time::macros::format_description;
use tracing_subscriber::registry::SpanRef;

use super::{EventVisitor, SpanExtAttrs, SpanExtTiming, SpanExtension};

/// Default time format
const TIME_FORMAT_DEFAULT: &[time::format_description::FormatItem<'static>] =
    format_description!("[hour]:[minute]:[second].[subsecond digits:6]");

/// A tracing layer with pretty print to the console
#[derive(Debug)]
pub struct PrettyConsolelayer {
    /// Time format
    time_format: &'static [time::format_description::FormatItem<'static>],
    /// Prints the enter and exit events
    print_enter_exit: bool,
    /// Shows the time
    show_time: bool,
    /// Shows the target
    show_target: bool,
}

impl Default for PrettyConsolelayer {
    fn default() -> Self {
        Self {
            time_format: TIME_FORMAT_DEFAULT,
            print_enter_exit: true,
            show_time: true,
            show_target: true,
        }
    }
}

impl PrettyConsolelayer {
    /// Sets the time format
    pub fn time_format(
        mut self,
        format: &'static [time::format_description::FormatItem<'static>],
    ) -> Self {
        self.time_format = format;
        self
    }

    /// Sets if enter and exit events should be printed
    pub fn print_enter_exit(mut self, print: bool) -> Self {
        self.print_enter_exit = print;
        self
    }

    /// Sets if the time is shown
    pub fn show_time(mut self, show: bool) -> Self {
        self.show_time = show;
        self
    }

    /// Sets if the target is shown
    pub fn show_target(mut self, show: bool) -> Self {
        self.show_target = show;
        self
    }
}

impl<S> tracing_subscriber::Layer<S> for PrettyConsolelayer
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let span_ref = ctx.span(id).expect("span not found");
        SpanExtAttrs::init(&span_ref);
        SpanExtTiming::init(&span_ref);
        SpanExtAttrs::record_attrs(&span_ref, attrs);
    }

    fn on_enter(&self, id: &tracing::span::Id, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let span_ref = ctx.span(id).expect("span not found");
        if self.print_enter_exit {
            let s = self.fmt_span_enter(&span_ref);
            eprint!("{s}");
        }
    }

    fn on_exit(&self, id: &tracing::span::Id, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let span_ref = ctx.span(id).expect("span not found");
        if self.print_enter_exit {
            let s = self.fmt_span_exit(&span_ref);
            eprint!("{s}");
        }

        if let Some(_parent) = span_ref.parent() {
            // => the span has a parent and hence it is recorded on the parent
            eprintln!("SPAN_HAS_PARENT");
        } else {
            // the exiting span is at the root and hence all spans can be printed
            eprintln!("SPAN_IS_ROOT");
        }
    }

    fn on_close(&self, _id: tracing::span::Id, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        // eprintln!("ON_CLOSE");
    }

    fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let span_ref = ctx.current_span().id().and_then(|id| ctx.span(id));
        let visitor = EventVisitor::record_event(event);
        let s = self.fmt_event::<S>(event, span_ref, &visitor);
        eprint!("{s}");
    }
}

impl PrettyConsolelayer {
    /// Returns the indentation for fields and attributes
    fn indent(&self) -> String {
        " ".repeat(6)
    }

    /// Prints info on span enter
    fn fmt_span_enter<S>(&self, span_ref: &SpanRef<S>) -> String
    where
        S: for<'a> tracing_subscriber::registry::LookupSpan<'a>,
    {
        let mut s = String::new();

        let now_str = if self.show_time {
            time::OffsetDateTime::now_utc()
                .format(&self.time_format)
                .expect("invalid datetime")
        } else {
            "".into()
        }
        .dimmed();

        let span_id = span_ref.id().into_u64();
        let span_name = span_ref.metadata().name().magenta();

        writeln!(s, "[==>] ({span_id}) {span_name} {now_str}").unwrap();

        if self.show_target {
            let target = span_ref.metadata().target().dimmed();
            let file = span_ref.metadata().file().unwrap_or("");
            let line = span_ref.metadata().line().unwrap_or_default().to_string();
            writeln!(
                s,
                "{}{}",
                self.indent(),
                format!("{target} ({file}:{line})").dimmed()
            )
            .unwrap();
        }

        // get span attributes
        let mut extensions = span_ref.extensions_mut();
        let ext_attrs = extensions
            .get_mut::<SpanExtAttrs>()
            .expect("extension not initialized");
        for (k, v) in &ext_attrs.attrs {
            writeln!(s, "{}{}: {}", self.indent(), k.italic(), v).unwrap();
        }

        s
    }

    /// Prints info on span exit
    fn fmt_span_exit<S>(&self, span_ref: &SpanRef<S>) -> String
    where
        S: for<'a> tracing_subscriber::registry::LookupSpan<'a>,
    {
        // Extensions
        let mut extensions = span_ref.extensions_mut();
        let ext_attrs = extensions
            .get_mut::<SpanExtTiming>()
            .expect("extension not initialized");

        let mut s = String::new();

        let now_str = if self.show_time {
            time::OffsetDateTime::now_utc()
                .format(&self.time_format)
                .expect("invalid datetime")
        } else {
            "".into()
        }
        .dimmed();

        let span_id = span_ref.id().into_u64();
        let duration_us = ext_attrs.entered.elapsed().as_micros();

        writeln!(s, "[<==] ({span_id}) {duration_us}us {now_str}").unwrap();

        if self.show_target {
            let target = span_ref.metadata().target().dimmed();
            let file = span_ref.metadata().file().unwrap_or("");
            let line = span_ref.metadata().line().unwrap_or_default().to_string();
            writeln!(
                s,
                "{}{}",
                self.indent(),
                format!("{target} ({file}:{line})").dimmed()
            )
            .unwrap();
        }

        s
    }

    /// Prints event
    fn fmt_event<S>(
        &self,
        event: &tracing::Event<'_>,
        span_ref: Option<SpanRef<S>>,
        visitor: &EventVisitor,
    ) -> String
    where
        S: for<'a> tracing_subscriber::registry::LookupSpan<'a>,
    {
        let mut s = String::new();

        let now_str = if self.show_time {
            time::OffsetDateTime::now_utc()
                .format(&self.time_format)
                .expect("invalid datetime")
        } else {
            "".into()
        }
        .dimmed();
        let level_str = {
            match *event.metadata().level() {
                tracing::Level::TRACE => "TRACE".magenta(),
                tracing::Level::DEBUG => "DEBUG".blue(),
                tracing::Level::INFO => "INFO ".green(),
                tracing::Level::WARN => "WARN ".yellow(),
                tracing::Level::ERROR => "ERROR".red(),
            }
        };
        let message = visitor.message();
        writeln!(s, "{level_str} {message} {now_str}").unwrap();

        if self.show_target {
            let target = event.metadata().target();
            let file = event.metadata().file().unwrap_or("");
            let line = event.metadata().line().unwrap_or_default().to_string();
            writeln!(
                s,
                "{}{}",
                self.indent(),
                format!("{target} ({file}:{line})").dimmed()
            )
            .unwrap();
        }

        // span context
        if let Some(r) = span_ref {
            let name = r.name();
            writeln!(
                s,
                "{}{}: {}",
                self.indent(),
                "span.name".dimmed(),
                name.truecolor(191, 160, 217)
            )
            .unwrap();
        }

        // event fields
        for (k, v) in visitor.meta_fields() {
            writeln!(s, "{}{}: {}", self.indent(), k.italic(), v).unwrap();
        }

        s
    }
}
