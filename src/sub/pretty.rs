//! A pretty tracing layer for console printing

use std::{collections::HashMap, io::Write, time::Instant};

use colored::Colorize;
use time::macros::format_description;
use tracing::Level;
use tracing_subscriber::registry::SpanRef;

use super::{EventVisitor, SpanExtension};

/// Default time format
const TIME_FORMAT_DEFAULT: &[time::format_description::FormatItem<'static>] =
    format_description!("[hour]:[minute]:[second].[subsecond digits:6]");

/// A tracing layer with pretty print to the console
///
/// ```
///  use tracing_ext::sub::PrettyConsoleLayer;
///
///  let pretty_layer = PrettyConsoleLayer::default()
///     .wrapped(true)
///     .oneline(false)
///     .events_only(true)
///     .show_time(true)
///     .show_target(true)
///     .show_file_info(true)
///     .show_span_info(true)
///     .indent(6);
/// ```
#[derive(Debug, Default)]
pub struct PrettyConsoleLayer {
    /// Format
    format: PrettyFormatOptions,
}

/// Formatting options (for spans and events)
#[derive(Debug)]
struct PrettyFormatOptions {
    /// Defines if the display is wrapped
    pub wrapped: bool,
    /// If true, spans and events are printed in 1 line
    pub oneline: bool,
    /// Time format
    pub time_format: &'static [time::format_description::FormatItem<'static>],
    /// The span is shown (enter and exit info)
    pub events_only: bool,
    /// The timestanp is shown
    pub show_time: bool,
    /// The target is shown
    pub show_target: bool,
    /// The file info is shown
    pub show_file_info: bool,
    /// Shows the span info
    pub show_span_info: bool,
    /// Indentation (x spaces) - invalid if the `oneline` option is set
    pub indent: usize,
}

impl Default for PrettyFormatOptions {
    fn default() -> Self {
        Self {
            wrapped: false,
            oneline: false,
            time_format: TIME_FORMAT_DEFAULT,
            events_only: false,
            show_time: true,
            show_target: true,
            show_file_info: true,
            show_span_info: true,
            indent: 6,
        }
    }
}

impl PrettyConsoleLayer {
    /// Sets the kind is wrapped
    pub fn wrapped(mut self, wrapped: bool) -> Self {
        self.format.wrapped = wrapped;
        self
    }

    /// Shows each span and event as 1 line
    pub fn oneline(mut self, oneline: bool) -> Self {
        self.format.oneline = oneline;
        self
    }

    /// Sets the time format
    pub fn time_format(
        mut self,
        format: &'static [time::format_description::FormatItem<'static>],
    ) -> Self {
        self.format.time_format = format;
        self
    }

    /// Sets if only the events are shown
    pub fn events_only(mut self, show: bool) -> Self {
        self.format.events_only = show;
        self
    }

    /// Sets if the time is shown
    pub fn show_time(mut self, show: bool) -> Self {
        self.format.show_time = show;
        self
    }

    /// Sets if the target is shown
    pub fn show_target(mut self, show: bool) -> Self {
        self.format.show_target = show;
        self
    }

    /// Sets if the file info is shown
    pub fn show_file_info(mut self, show: bool) -> Self {
        self.format.show_file_info = show;
        self
    }

    /// Sets if the span inline info is shown
    pub fn show_span_info(mut self, show: bool) -> Self {
        self.format.show_span_info = show;
        self
    }

    /// Sets the indentation (in x spaces)
    pub fn indent(mut self, indent: usize) -> Self {
        self.format.indent = indent;
        self
    }
}

/// A span extension for the span record
#[derive(Debug)]
struct SpanExtRecord {
    /// Level within the tree
    tree_level: usize,
    /// Span ID
    id: u64,
    /// Span name
    name: &'static str,
    /// Span target
    target: String,
    /// File
    file: String,
    /// Line
    line: u32,
    /// Span attributes
    attrs: HashMap<&'static str, String>,
    /// Entered time
    entered: Instant,
    /// Events within the span
    events: Vec<EventRecord>,
    // children
    children: Vec<SpanExtRecord>,
}

impl tracing::field::Visit for SpanExtRecord {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        let value = format!("{value:?}");
        self.attrs.insert(field.name(), value);
    }
}

impl SpanExtension for SpanExtRecord {}

impl SpanExtRecord {
    /// Instantiates from a [SpanRef]
    ///
    /// NB: attributes are not collected yet
    fn new_from_span_ref<S>(span_ref: &SpanRef<S>) -> Self
    where
        S: for<'b> tracing_subscriber::registry::LookupSpan<'b>,
    {
        let tree_level = if let Some(parent) = span_ref.parent() {
            let extensions = parent.extensions();
            let tree_level = extensions.get::<Self>().unwrap().tree_level;
            tree_level + 1
        } else {
            0
        };

        Self {
            tree_level,
            id: span_ref.id().into_u64(),
            name: span_ref.name(),
            target: span_ref.metadata().target().to_string(),
            file: span_ref.metadata().file().unwrap_or("").to_string(),
            line: span_ref.metadata().line().unwrap_or(0),
            attrs: HashMap::new(),
            entered: Instant::now(),
            events: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Serializes the span entry
    fn serialize_span_entry(&self, opts: &PrettyFormatOptions) -> Vec<u8> {
        if opts.events_only {
            return vec![];
        }

        let mut buf: Vec<u8> = vec![];

        let tree_indent = if opts.wrapped {
            self.tree_level * opts.indent
        } else {
            0
        };
        let tree_indent_str = " ".repeat(tree_indent);
        write!(buf, "{}", tree_indent_str).unwrap();

        if !opts.wrapped {
            write!(buf, "{:w$}", format!("-->"), w = opts.indent).unwrap();
        }
        write!(buf, "{}", format!("{{{}}}", self.name).magenta()).unwrap();

        let field_indent = tree_indent + opts.indent;
        let field_indent_str = " ".repeat(field_indent);
        let field_new_line = if opts.oneline {
            " ".to_string()
        } else {
            format!("\n{field_indent_str}")
        };

        if opts.show_time {
            let time_str = time::OffsetDateTime::now_utc()
                .format(opts.time_format)
                .expect("invalid datetime");
            let line = format!("{}: {}", "time".italic(), time_str);
            write!(buf, "{field_new_line}{}", line.dimmed()).unwrap();
        };

        // span info
        if opts.show_span_info {
            let span_id = format!("{}: {}", "span.id".italic(), self.id);
            write!(buf, "{field_new_line}{}", span_id.dimmed()).unwrap();
        }

        if opts.show_target {
            let target = format!("{}: {}", "target".italic(), self.target);
            write!(buf, "{field_new_line}{}", target.dimmed()).unwrap();
        }

        if opts.show_file_info {
            let target = format!("{}: {}:{}", "file".italic(), self.file, self.line);
            write!(buf, "{field_new_line}{}", target.dimmed()).unwrap();
        }

        // span attributes
        for (k, v) in &self.attrs {
            write!(buf, "{field_new_line}{}={}", k.to_string().italic(), v).unwrap();
        }

        buf
    }

    /// Serializes the span exit
    fn serialize_span_exit(&self, opts: &PrettyFormatOptions) -> Vec<u8> {
        if opts.events_only {
            return vec![];
        }

        let mut buf: Vec<u8> = vec![];

        let tree_indent = if opts.wrapped {
            self.tree_level * opts.indent
        } else {
            0
        };
        let tree_indent_str = " ".repeat(tree_indent);
        write!(buf, "{}", tree_indent_str).unwrap();

        if !opts.wrapped {
            write!(buf, "{:w$}", format!("<--"), w = opts.indent).unwrap();
        }
        write!(buf, "{}", format!("!{{{}}}", self.name).magenta()).unwrap();

        // span info
        if opts.show_span_info {
            let span_id = format!("({}={})", "id".italic(), self.id);
            write!(buf, " {}", span_id.dimmed()).unwrap();
        }

        let duration_us = self.entered.elapsed().as_micros();
        write!(buf, " {}", format!("{duration_us}us").dimmed()).unwrap();

        buf
    }
}

/// An event record
#[derive(Debug)]
struct EventRecord {
    level: Level,
    target: String,
    file: String,
    line: u32,
    message: String,
    meta_fields: HashMap<&'static str, String>,
    /// Span info (tree level, id, name)
    span: Option<(usize, u64, String)>,
}

impl EventRecord {
    /// Serializes an event
    fn serialize(&self, opts: &PrettyFormatOptions) -> Vec<u8> {
        let mut buf: Vec<u8> = vec![];

        let tree_indent = if opts.wrapped {
            let tree_level = self.span.as_ref().map(|(l, _, _)| *l).unwrap_or(0);
            tree_level * opts.indent
        } else {
            0
        };
        let tree_indent_str = " ".repeat(tree_indent);
        write!(buf, "{}", tree_indent_str).unwrap();

        let level_str = match self.level {
            tracing::Level::TRACE => format!("{:w$}", "TRACE", w = opts.indent).magenta(),
            tracing::Level::DEBUG => format!("{:w$}", "DEBUG", w = opts.indent).blue(),
            tracing::Level::INFO => format!("{:w$}", "INFO", w = opts.indent).green(),
            tracing::Level::WARN => format!("{:w$}", "WARN", w = opts.indent).yellow(),
            tracing::Level::ERROR => format!("{:w$}", "ERROR", w = opts.indent).red(),
        };
        write!(buf, "{}", level_str).unwrap();
        write!(buf, "{}", self.message).unwrap();

        let field_indent = tree_indent + opts.indent;
        let field_indent_str = " ".repeat(field_indent);
        let field_new_line = if opts.oneline {
            " ".to_string()
        } else {
            format!("\n{field_indent_str}")
        };

        if opts.show_time {
            let time_str = time::OffsetDateTime::now_utc()
                .format(opts.time_format)
                .expect("invalid datetime");
            let line = format!("{}: {}", "time".italic(), time_str);
            write!(buf, "{field_new_line}{}", line.dimmed()).unwrap();
        };

        // event context
        if opts.show_span_info {
            if let Some((_, id, name)) = &self.span {
                let span_id = format!("{}: {}", "span.id".italic(), id);
                write!(buf, "{field_new_line}{}", span_id.dimmed()).unwrap();

                let span_name = format!(
                    "{field_new_line}{}{} {}",
                    "span.name".italic().dimmed(),
                    ":".dimmed(),
                    name.truecolor(191, 160, 217)
                );
                write!(buf, "{}", span_name.dimmed()).unwrap();
            }
        }

        if opts.show_target {
            let target = format!("{}: {}", "target".italic(), self.target);
            write!(buf, "{field_new_line}{}", target.dimmed()).unwrap();
        }

        if opts.show_file_info {
            let target = format!("{}: {}:{}", "file".italic(), self.file, self.line);
            write!(buf, "{field_new_line}{}", target.dimmed()).unwrap();
        }

        // event fields
        for (k, v) in &self.meta_fields {
            write!(buf, "{field_new_line}{}={}", k.to_string().italic(), v).unwrap();
        }

        buf
    }
}

impl<S> tracing_subscriber::Layer<S> for PrettyConsoleLayer
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
        let record = SpanExtRecord::new_from_span_ref(&span_ref);
        SpanExtRecord::register_value(record, &span_ref);
        SpanExtRecord::record_attrs(&span_ref, attrs);
    }

    fn on_enter(&self, id: &tracing::span::Id, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let span_ref = ctx.span(id).expect("span not found");

        let mut extensions = span_ref.extensions_mut();
        let record = extensions
            .get_mut::<SpanExtRecord>()
            .expect("Extension not initialized");

        if !self.format.wrapped {
            let buf = record.serialize_span_entry(&self.format);
            if !buf.is_empty() {
                eprintln!("{}", std::str::from_utf8(&buf).unwrap());
            }
        }
    }

    fn on_exit(&self, id: &tracing::span::Id, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let span_ref = ctx.span(id).expect("span not found");

        let mut extensions = span_ref.extensions_mut();
        let record = extensions
            .get_mut::<SpanExtRecord>()
            .expect("Extension not initialized");

        if !self.format.wrapped {
            let buf = record.serialize_span_exit(&self.format);
            if !buf.is_empty() {
                eprintln!("{}", std::str::from_utf8(&buf).unwrap());
            }
        }
    }

    fn on_close(&self, id: tracing::span::Id, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let span_ref = ctx.span(&id).expect("span not found");

        // When wrapping, if the span has a parent, we record it as a child of the parent.
        // If it is the root, the span tree is outputted
        if self.format.wrapped {
            if let Some(parent_ref) = span_ref.parent() {
                // => the span has a parent and hence it is recorded on the parent
                let mut parent_extensions = parent_ref.extensions_mut();
                let parent_record = parent_extensions
                    .get_mut::<SpanExtRecord>()
                    .expect("Extension not initialized");

                let mut extensions = span_ref.extensions_mut();
                let record = extensions
                    .remove::<SpanExtRecord>()
                    .expect("Extension not initialized");

                parent_record.children.push(record);
            } else {
                // => root of span tree => print
                let mut extensions = span_ref.extensions_mut();
                let record = extensions
                    .remove::<SpanExtRecord>()
                    .expect("Extension not initialized");
                self.output_root_tree(&record);
            }
        }
    }

    fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let visitor = EventVisitor::record_event(event);

        let evt_record = EventRecord {
            level: *event.metadata().level(),
            target: event.metadata().target().to_string(),
            file: event.metadata().file().unwrap_or("").to_string(),
            line: event.metadata().line().unwrap_or(0),
            message: visitor.message().to_string(),
            meta_fields: visitor
                .meta_fields()
                .iter()
                .map(|(k, v)| (*k, v.to_string()))
                .collect(),
            span: ctx.current_span().id().map(|id| {
                let parent_ref = ctx.span(id).expect("span not found");
                let mut extensions = parent_ref.extensions_mut();
                let span_record = extensions
                    .get_mut::<SpanExtRecord>()
                    .expect("Extension not initialized");
                (
                    span_record.tree_level + 1,
                    id.into_u64(),
                    ctx.current_span().metadata().unwrap().name().to_string(),
                )
            }),
        };

        // we print the event is we print by chronological order, or if the event is at the root
        match (self.format.wrapped, ctx.current_span().id().is_some()) {
            (false, _) | (true, false) => {
                let buf = evt_record.serialize(&self.format);
                eprintln!("{}", std::str::from_utf8(&buf).unwrap());
            }
            _ => {
                // NB: push the events to the span record if everything is printed at the end
                let curr_span = ctx.current_span();
                let parent_span_id = curr_span.id().unwrap();
                let span_ref = ctx.span(parent_span_id).expect("span not found");
                let mut extensions = span_ref.extensions_mut();
                let span_record = extensions
                    .get_mut::<SpanExtRecord>()
                    .expect("Extension not initialized");
                span_record.events.push(evt_record);
            }
        }
    }
}

impl PrettyConsoleLayer {
    /// Outputs a tree of spans from the root
    fn output_root_tree(&self, record: &SpanExtRecord) {
        // eprintln!("ENTER SPAN {}", record.id);
        let buf = record.serialize_span_entry(&self.format);
        if !buf.is_empty() {
            eprintln!("{}", std::str::from_utf8(&buf).unwrap());
        }

        for event in &record.events {
            let buf = event.serialize(&self.format);
            if !buf.is_empty() {
                println!("{}", std::str::from_utf8(&buf).unwrap());
            }
        }

        for child in &record.children {
            self.output_root_tree(child);
        }

        let buf = record.serialize_span_exit(&self.format);
        if !buf.is_empty() {
            eprintln!("{}", std::str::from_utf8(&buf).unwrap());
        }
    }
}
