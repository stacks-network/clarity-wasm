use std::{collections::BTreeMap, time::SystemTime, fmt::Write as _, io::Write};

use time::{macros::format_description, Duration, Instant, OffsetDateTime};
use console::style;
use parking_lot::Mutex;
use tracing::{field::{Field, Visit}, span, Event, Level, Subscriber};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

#[derive(Debug, Default)]
pub struct ClarityTracingLayer {
    data: Mutex<LayerData>
}

#[derive(Debug, Default)]
struct LayerData {
    current_span_id: Option<span::Id>,
    total_spans: u32,
    active_spans: u32,
}

#[derive(Debug, Default)]
struct Data {
    first_entered_at: Option<Instant>,
    last_entered_at: Option<Instant>,
    accumulated_time: Duration,
    fields: BTreeMap<&'static str, FieldValue>,
    parent_span: Option<span::Id>,
    level: usize
}

impl Data {
    pub fn parent_span_id(&self) -> &Option<span::Id> {
        &self.parent_span
    }
}

impl Visit for Data {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.fields.insert(field.name(), FieldValue::Debug(format!("{:?}", value)));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields.insert(field.name(), FieldValue::Bool(value));
    }

    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        tracing::error!(field = field.name(), error = value);
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.fields.insert(field.name(), FieldValue::F64(value));
    }

    fn record_i128(&mut self, field: &Field, value: i128) {
        self.fields.insert(field.name(), FieldValue::I64(value as i64));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields.insert(field.name(), FieldValue::I64(value));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.fields.insert(field.name(), FieldValue::String(value.to_owned()));
    }

    fn record_u128(&mut self, field: &Field, value: u128) {
        self.fields.insert(field.name(), FieldValue::U64(value as u64));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields.insert(field.name(), FieldValue::U64(value));
    }
}

#[derive(Debug, Clone)]
enum FieldValue {
    F64(f64),
    I64(i64),
    U64(u64),
    Bool(bool),
    String(String),
    Debug(String),
}

impl std::fmt::Display for FieldValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FieldValue::F64(value) => write!(f, "{}", value),
            FieldValue::I64(value) => write!(f, "{}", value),
            FieldValue::U64(value) => write!(f, "{}", value),
            FieldValue::Bool(value) => write!(f, "{}", value),
            FieldValue::String(value) => write!(f, "{}", value),
            FieldValue::Debug(value) => write!(f, "{}", value),
        }
    }
}

impl<S> Layer<S> for ClarityTracingLayer
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    /// Called when a new span has been created, but not yet entered.
    fn on_new_span(
        &self, 
        attrs: &span::Attributes<'_>, 
        id: &span::Id, 
        ctx: Context<'_, S>
    ) {
        let span = ctx.span(id)
            .expect("span should exist");

        let mut span_data = Data::default();

        let mut current_span = span.parent();
        if current_span.is_none() {
            if let Some(current_span_id) = ctx.current_span().id() {
                current_span = ctx.span(current_span_id);
            }
        }

        // If there's already a current span, then this span is a child of that span.
        if let Some(parent_span) = current_span {
            span_data.parent_span = Some(parent_span.id());

            let parent_extensions = parent_span.extensions();
            let parent_data = parent_extensions
                .get::<Data>()
                .expect("parent data should exist");

            span_data.level = parent_data.level + 1;

            for field in &parent_data.fields {
                span_data.fields.insert(field.0, field.1.clone());
            }
        }

        // We do this _after_ inheriting parent fields so that a child span can
        // overwrite its parent's fields.
        attrs.record(&mut span_data);

        let mut span_extensions = span.extensions_mut();
        span_extensions.insert::<Data>(span_data);

        let mut layer_data = self.data.lock();
        layer_data.total_spans += 1;
        layer_data.current_span_id = Some(id.clone());
    }

    /// Called when a span has been modified after creation. Note that new fields
    /// cannot be added here, only updated, so all fields must be initialized in
    /// [Self::on_new_span] i.e. on [tracing::Span] creation. To define a field 
    /// with an empty value, use [tracing::field::Empty] as the value.
    fn on_record(
        &self, 
        id: &span::Id, 
        values: &span::Record<'_>, 
        ctx: Context<'_, S>
    ) {
        let span = ctx.span(id)
            .expect("span should exist");

        let mut span_extensions = span.extensions_mut();

        let span_data = span_extensions
            .get_mut::<Data>()
            .expect("data should exist");

        values.record(span_data);
    }

    /// Called when a [tracing::Span] is entered. This may occur multiple times
    /// during the lifetime of the span.
    fn on_enter(
        &self, 
        id: &span::Id, 
        ctx: Context<'_, S>
    ) {
        let span = ctx.span(id)
            .expect("span should exist");

        let mut span_extensions = span.extensions_mut();
        
        let span_data = span_extensions
            .get_mut::<Data>()
            .expect("data should exist");

        self.data.lock().active_spans += 1;

        let now = Instant::now();
        span_data.first_entered_at = Some(now);
        span_data.last_entered_at = Some(now);
    }

    /// Called when a [tracing::Span] is exited. This may occur multiple times
    /// during the lifetime of the [tracing::Span].
    fn on_exit(
        &self, 
        id: &span::Id, 
        ctx: Context<'_, S>
    ) {
        let span = ctx.span(id)
            .expect("span should exist");

        let mut span_extensions = span.extensions_mut();

        let span_data = span_extensions
            .get_mut::<Data>()
            .expect("data should exist");

        let now = Instant::now();
        let elapsed = now - span_data.last_entered_at
            .expect("span should have last_entered_at set");
        span_data.last_entered_at = None;
        span_data.accumulated_time += elapsed;

        let mut layer_data = self.data.lock();
        layer_data.active_spans -= 1;
        layer_data.current_span_id = span_data.parent_span.clone();
    }

    /// Called when the [tracing::Span] has been dropped. This is guaranteed to
    /// only be called once per [tracing::Span].
    fn on_close(
        &self, 
        id: span::Id, 
        ctx: Context<'_, S>
    ) {
        let span = ctx.span(&id)
            .expect("span should exist");

        let mut span_extensions = span.extensions_mut();
        let span_data = span_extensions
            .get_mut::<Data>()
            .expect("data should exist");

        let mut level_data = self.data.lock();
        level_data.total_spans -= 1;
    }
}

#[derive(Debug, Default)]
struct PrintData {
    has_had_child: bool,
}

#[derive(Clone, Default)]
pub struct PrintTreeLayer;

impl<S> Layer<S> for PrintTreeLayer
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    fn on_new_span(&self, _attrs: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, S>) {
        let span = ctx.span(id)
            .expect("span should exist");

        let mut span_extensions = span.extensions_mut();

        let print_data = PrintData::default();
        span_extensions.insert::<PrintData>(print_data);
    }

    fn on_enter(&self, id: &span::Id, ctx: Context<'_, S>) {
        let span = ctx.span(id)
            .expect("span should exist");

        let span_extensions = span.extensions();
        
        let span_data = span_extensions
            .get::<Data>()
            .expect("data should exist");

        //println!("{}on_enter for span: {:?}", str::repeat(".", span_data.level * 2), id);
        let _ = self.print_enter(span.metadata().level(), id, span.name(), span_data);
    }

    fn on_exit(&self, id: &span::Id, ctx: Context<'_, S>) {
        let span = ctx.span(id)
            .expect("span should exist");

        let span_extensions = span.extensions();
        
        let span_data = span_extensions
            .get::<Data>()
            .expect("data should exist");      

        //println!("{}on_exit for span: {:?}", str::repeat(".", span_data.level * 2), id);
        let _ = self.print_exit(span.metadata().level(), id, span.name(), span_data);
    }

    fn on_close(&self, id: span::Id, ctx: Context<'_, S>) {
        let span = ctx.span(&id)
            .expect("span should exist");

        let span_extensions = span.extensions();
        
        let span_data = span_extensions
            .get::<Data>()
            .expect("data should exist");

        let print_data = span_extensions
            .get::<PrintData>()
            .expect("print data should exist");

        let mut has_parent_had_children = false;
        if let Some(parent_span_id) = span_data.parent_span_id() {
            let parent_span = ctx.span(parent_span_id)
                .expect("parent span should exist");

            let mut parent_span_extensions = parent_span.extensions_mut();
            let print_data = parent_span_extensions
                .get_mut::<PrintData>()
                .expect("print data should exist");

            if print_data.has_had_child {
                has_parent_had_children = true;
            } else {
                print_data.has_had_child = true;
            }
        }

        let _ = self.print_close(span.metadata().level(), &id, has_parent_had_children,  span_data);

        
    }

    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let current = ctx.current_span();
        let current_span_id = event.parent().or(current.id());

        if let Some(span_id) = current_span_id {
            let span = ctx.span(span_id)
                .expect("span should exist");

            let span_extensions = span.extensions();
            
            let span_data = span_extensions
                .get::<Data>()
                .expect("data should exist");

            //println!("{}on_event for span: {:?}", str::repeat(".", span_data.level * 2), span_id);
            let _ = self.print_event_with_span(span_id, span_data, event);
        } else {
            let _ = self.print_event_without_span(event);
        }
    }
}

impl PrintTreeLayer {
    fn format_time() -> color_eyre::Result<String> {
        let format = format_description!("[year]-[month]-[day] [hour]:[minute]:[second]Z");
        Ok(OffsetDateTime::now_utc().format(&format)?)
    }

    fn print_prefix(level: &Level) -> color_eyre::Result<()> {
        let mut f = std::io::stdout();

        write!(f, " {}", style("[").dim())?;
        let mut styled_level = style(
            format!("{: <5}", level)
        ).dim();
        match *level {
            Level::INFO => styled_level = styled_level.green(),
            Level::DEBUG => styled_level = styled_level.blue(),
            Level::TRACE => {},
            Level::WARN => styled_level = styled_level.yellow(),
            Level::ERROR => styled_level = styled_level.red(),
        }
        write!(f, "{} ", styled_level)?;
        write!(f, "{}", style(Self::format_time()?).dim())?;
        write!(f, "{} ", style("]").dim())?;

        Ok(())
    }

    fn print_enter(&self, level: &Level, id: &span::Id, name: &str, data: &Data) -> color_eyre::Result<()> {
        Self::print_prefix(level)?;

        let mut f = std::io::stdout();
        for i in 1..=data.level {
            if i == data.level {
                if i > 1 { write!(f, " ")?; }
                write!(f, "├── ")?;
            } else {
                if i > 1 { write!(f, " ")?; }
                write!(f, "│  ")?;
            }
        }
        write!(f, "{}", style("⥂ ").green())?;
        write!(f, "{}", style(name).bold())?;
        write!(f, "\r\n")?;

        Ok(())
    }

    fn print_exit(&self, level: &Level, id: &span::Id, name: &str, data: &Data) -> color_eyre::Result<()> {
        Self::print_prefix(level)?;

        let mut f = std::io::stdout();
        for i in 1..=data.level {
            if i == data.level {
                if i > 1 { write!(f, " ")?; }
                write!(f, "└── ")?;
            } else {
                if i > 1 { write!(f, " ")?; }
                write!(f, "│  ")?;
            }
        }
        write!(f, "{}", style("⥄ ").red())?;
        write!(f, "{}", style(name).bold())?;
        write!(f, "\r\n")?;

        Ok(())
    }

    fn print_close(&self, level: &Level, id: &span::Id, has_had_child: bool, data: &Data) -> color_eyre::Result<()> {
        Self::print_prefix(level)?;

        let mut f = std::io::stdout();

        for i in 1..=data.level {
            if i == data.level {
                if i > 1 { write!(f, " ")?; }
                write!(f, "    ")?;
            } else {
                if i > 1 { write!(f, " ")?; }
                write!(f, "│  ")?;
            }
        }

        //if data.level == 0 { write!(f, " ")?; }
        write!(f, "{}", style("⟳").dim())?;
        write!(f, " level: {}, has_parent_had_child: {} ", data.level, has_had_child)?;
        write!(f, "{}", style(" runtime ").dim().bold())?;
        write!(f, "{}", style("⇝ total: ").dim())?;
        write!(f, "{}", style("5ns, ").cyan().dim())?;
        write!(f, "{}", style("busy: ").dim())?;
        write!(f, "{}", style("4ns, ").cyan().dim())?;
        write!(f, "{}", style("idle: ").dim())?;
        write!(f, "{}", style("4ns").cyan().dim())?;

        write!(f, "\r\n")?;

        Ok(())
    }

    fn print_event_with_span(&self, span_id: &span::Id, span_data: &Data, event: &Event) -> color_eyre::Result<()> {
        Self::print_prefix(event.metadata().level())?;

        let mut f = std::io::stdout();

        for i in 1..=span_data.level {
            if i > 100 {
                panic!("exceeded max span depth");
            }
            if i == span_data.level {
                if i > 1 { write!(f, " ")?; }
                write!(f, "├── ")?;
            } else {
                if i > 1 { write!(f, " ")?; }
                write!(f, "│  ")?;
            }
        }

        let mut event_data = Data::default();
        event.record(&mut event_data);
    
        write!(f, "{}", style("⚡").yellow().dim())?;
        write!(f, "{}", event_data.fields["message"])?;
        write!(f, "\r\n")?;

        Ok(())
    }

    fn print_event_without_span(&self, event: &Event) -> color_eyre::Result<()> {
        Self::print_prefix(event.metadata().level())?;

        let mut f = std::io::stdout();

        let mut event_data = Data::default();
        event.record(&mut event_data);

        write!(f, "{}", style("⚡").yellow().dim())?;
        write!(f, "{}", event_data.fields["message"])?;
        write!(f, "\r\n")?;
        Ok(())
    }
}

pub struct CleanupLayer;

impl<S> Layer<S> for CleanupLayer
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    fn on_close(&self, id: span::Id, ctx: Context<'_, S>) {
        let span = ctx.span(&id)
            .expect("span should exist");

        let mut span_extensions = span.extensions_mut();
        if let Some(data) = span_extensions.remove::<Data>() {
            drop(data);
        }
    }
}