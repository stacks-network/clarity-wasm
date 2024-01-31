use std::{collections::BTreeMap, fmt::Write, time::{Duration, Instant}};

use console::style;
use parking_lot::Mutex;
use tracing::{field::{Field, Visit}, span, Event, Subscriber};
use tracing_subscriber::{layer::Context, registry::{LookupSpan, SpanRef}, Layer};

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
        let mut span_data = Data::default();

        // If there's already a current span, then this span is a child of that span.
        if let Some(current_span_id) = &self.data.lock().current_span_id {
            span_data.parent_span = Some(current_span_id.clone());

            let parent_span = ctx.span(current_span_id)
                .expect("current span should exist");

            let parent_extensions = parent_span.extensions();
            let parent_data = parent_extensions
                .get::<Data>()
                .expect("parent data should exist");

            span_data.level = parent_data.level + 1;

            for field in &parent_data.fields {
                span_data.fields.insert(field.0, field.1.clone());
            }
        }

        attrs.record(&mut span_data);

        let span = ctx.span(id)
            .expect("span should exist");

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

        if let Some(parent_span_id) = span_data.parent_span_id() {
            let parent_span = ctx.span(parent_span_id)
                .expect("parent span should exist");

            let mut parent_extensions = parent_span.extensions_mut();
            let parent_data = parent_extensions.get_mut::<Data>()
                .expect("parent data should exist");

            span_data.fields.append(&mut parent_data.fields);
        }

        self.data.lock().total_spans -= 1;
    }

    fn on_event(&self, _event: &Event<'_>, ctx: Context<'_, S>) {
        //println!("event [current span {:?}]: {:?}", ctx.current_span().id(), _event);
    }
}

#[derive(Clone, Default)]
pub struct PrintTreeLayer;

impl<S> Layer<S> for PrintTreeLayer
where
    S: Subscriber + for<'span> LookupSpan<'span>,
{
    fn on_enter(&self, id: &span::Id, ctx: Context<'_, S>) {
        let span = ctx.span(id)
            .expect("span should exist");

        let span_extensions = span.extensions();
        
        let span_data = span_extensions
            .get::<Data>()
            .expect("data should exist");

        //println!("{}on_enter for span: {:?}", str::repeat(".", span_data.level * 2), id);
        self.print_enter(id, span_data);
    }

    fn on_exit(&self, id: &span::Id, ctx: Context<'_, S>) {
        let span = ctx.span(id)
            .expect("span should exist");

        let span_extensions = span.extensions();
        
        let span_data = span_extensions
            .get::<Data>()
            .expect("data should exist");

        //println!("{}on_exit for span: {:?}", str::repeat(".", span_data.level * 2), id);
        self.print_exit(id, span_data);
    }

    fn on_close(&self, id: span::Id, ctx: Context<'_, S>) {
        let span = ctx.span(&id)
            .expect("span should exist");

        let span_extensions = span.extensions();
        
        let span_data = span_extensions
            .get::<Data>()
            .expect("data should exist");

        //println!("{}on_close for span: {:?}, level = {}", str::repeat(".", span_data.level * 2), id, span_data.level);
        let _ = self.print_close(&id, span_data);
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
    fn print_enter(&self, id: &span::Id, data: &Data) {
        let mut buffer = String::new();
        for i in 1..=data.level {
            if i == data.level {
                if i > 1 { buffer.push(' ') }
                buffer.push_str("├── ");
            } else {
                if i > 1 { buffer.push(' ') }
                buffer.push_str("│  ");
            }
        }
        buffer.push_str(&style("⥂").green().dim().to_string());
        buffer.push_str(&format!(" enter for span: {:?}", id));
        println!("{}", buffer);
    }

    fn print_exit(&self, id: &span::Id, data: &Data) {
        let mut buffer = String::new();
        for i in 1..=data.level {
            if i == data.level {
                if i > 1 { buffer.push(' ') }
                buffer.push_str("└── ");
            } else {
                if i > 1 { buffer.push(' ') }
                buffer.push_str("│  ");
            }
        }
        buffer.push_str(&style("⥄").red().dim().to_string());
        buffer.push_str(&format!(" exit for span: {:?}", id));
        println!("{}", buffer);
    }

    fn print_close(&self, id: &span::Id, data: &Data) -> color_eyre::Result<()> {
        let mut f = String::new();
        for i in 0..=data.level - 1 {
            if i < data.level - 1 {
                if i > 1 { f.push(' ') }
                f.push_str("│  ");
            } else {
                if i > 0 { f.push(' ') }
                f.push_str("    ");
            }
        }
        //write!(f, "{}", style("[ ").dim().bold())?;
        
        write!(f, "{}", style("⟳").dim())?;
        write!(f, "{}", style(" runtime ").dim().bold())?;
        write!(f, "{}", style("total: ").dim())?;
        write!(f, "{}", style("5ns, ").cyan().dim())?;
        write!(f, "{}", style("busy: ").dim())?;
        write!(f, "{}", style("4ns, ").cyan().dim())?;
        write!(f, "{}", style("idle: ").dim())?;
        write!(f, "{}", style("4ns").cyan().dim())?;
        //write!(f, "{}", style(" ]").dim().bold())?;

        println!("{}",  f);

        Ok(())
    }

    fn print_event_with_span(&self, span_id: &span::Id, span_data: &Data, event: &Event) -> color_eyre::Result<()> {
        let mut buffer = String::new();

        for i in 1..=span_data.level {
            if i == span_data.level {
                if i > 1 { buffer.push(' ') }
                buffer.push_str("├── ");
            } else {
                if i > 1 { buffer.push(' ') }
                buffer.push_str("│  ");
            }
        }

        let mut event_data = Data::default();
        event.record(&mut event_data);

        buffer.push_str(&style("⚡").yellow().dim().to_string());
        buffer.push_str(&format!("event: {:?}", event_data.fields["message"]));
        println!("{}", buffer);

        Ok(())
    }

    fn print_event_without_span(&self, event: &Event) -> color_eyre::Result<()> {
        println!("without span: {:?}", event);
        Ok(())
    }
}