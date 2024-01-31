use std::{collections::BTreeMap, time::{Duration, Instant}};

use parking_lot::Mutex;
use tracing::{field::{Field, Visit}, span, Event, Subscriber};
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

        //println!("{}on_enter for span: {:?}", str::repeat(".", span_data.level * 2), id);

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

        self.data.lock().active_spans -= 1;
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
        println!("event [current span {:?}]: {:?}", ctx.current_span().id(), _event);
    }
}

#[derive(Clone, Default)]
pub struct PrintTreeLayer {}

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

        println!("{}on_enter for span: {:?}", str::repeat(".", span_data.level * 2), id);
    }

    fn on_exit(&self, id: &span::Id, ctx: Context<'_, S>) {
        let span = ctx.span(id)
            .expect("span should exist");

        let span_extensions = span.extensions();
        
        let span_data = span_extensions
            .get::<Data>()
            .expect("data should exist");

        println!("{}on_exit for span: {:?}", str::repeat(".", span_data.level * 2), id);
    }

    fn on_close(&self, id: span::Id, ctx: Context<'_, S>) {
        let span = ctx.span(&id)
            .expect("span should exist");

        let span_extensions = span.extensions();
        
        let span_data = span_extensions
            .get::<Data>()
            .expect("data should exist");

        println!("{}on_close for span: {:?}", str::repeat(".", span_data.level * 2), id);
    }
}