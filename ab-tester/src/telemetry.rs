use std::{collections::BTreeMap, fmt::Write as _, io::Write};

use cpu_time::ProcessTime;
use time::{macros::format_description, Duration, Instant, OffsetDateTime};
use console::style;
//use parking_lot::Mutex;
use tracing::{field::{Field, Visit}, span, Event, Level, Subscriber};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

#[derive(Debug, Default)]
pub struct ClarityTracingLayer;

/*#[derive(Debug, Default)]
struct LayerData {
    current_span_id: Option<span::Id>,
    total_spans: u32,
    active_spans: u32,
}*/

#[derive(Debug, Default)]
struct Data {
    //real_first_entered_at: Option<Instant>,
    real_last_entered_at: Option<Instant>,
    real_accumulated_time: Duration,
    real_wait_time: Duration,
    cpu_last_entered_at: Option<ProcessTime>,
    cpu_accumulated_time: Duration,
    cpu_wait_time: Duration,
    cpu_system_time: Duration,
    fields: BTreeMap<&'static str, FieldValue>,
    parent_span: Option<span::Id>,
    level: usize,
    has_had_children: bool
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

            let mut parent_extensions = parent_span.extensions_mut();
            let parent_data = parent_extensions
                .get_mut::<Data>()
                .expect("parent data should exist");

            parent_data.has_had_children = true;

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

        //let mut layer_data = self.data.lock();
        //layer_data.total_spans += 1;
        //layer_data.current_span_id = Some(id.clone());
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

        //self.data.lock().active_spans += 1;

        let now = Instant::now();
        //span_data.real_first_entered_at = Some(now);
        span_data.real_last_entered_at = Some(now);
        span_data.cpu_last_entered_at = Some(ProcessTime::now());
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

        let real_now = Instant::now();
        let real_elapsed = real_now - span_data.real_last_entered_at
            .expect("span should have last_entered_at set");
        span_data.real_last_entered_at = None;
        span_data.real_accumulated_time += real_elapsed;

        span_data.cpu_accumulated_time += span_data.cpu_last_entered_at
            .expect("span should have cpu_last_entered_at set")
            .elapsed();
        span_data.cpu_last_entered_at = None;
        span_data.cpu_system_time = span_data.cpu_accumulated_time - span_data.cpu_wait_time;

        if let Some(parent_span_id) = span_data.parent_span_id() {
            let parent_span = ctx.span(parent_span_id)
                .expect("parent span should exist");

            let mut parent_span_extensions = parent_span.extensions_mut();
            let parent_span_data = parent_span_extensions
                .get_mut::<Data>()
                .expect("parent data should exist");

            parent_span_data.real_wait_time += span_data.real_accumulated_time;
            parent_span_data.cpu_wait_time += span_data.cpu_accumulated_time;
            parent_span_data.cpu_system_time += span_data.cpu_system_time;
        }
        // /Moved from close

        //let mut layer_data = self.data.lock();
        //layer_data.active_spans -= 1;
        //layer_data.current_span_id = span_data.parent_span.clone();
    }

    /// Called when the [tracing::Span] has been dropped. This is guaranteed to
    /// only be called once per [tracing::Span].
    fn on_close(
        &self, 
        id: span::Id, 
        ctx: Context<'_, S>
    ) {
        //let span = ctx.span(&id)
        //    .expect("span should exist");

        /*let mut span_extensions = span.extensions_mut();
        let span_data = span_extensions
            .get_mut::<Data>()
            .expect("data should exist");*/

        

        //let mut level_data = self.data.lock();
        //level_data.total_spans -= 1;
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

        let _ = self.print_enter(span.metadata().level(), span.name(), span_data);
    }

    fn on_exit(&self, id: &span::Id, ctx: Context<'_, S>) {
        let span = ctx.span(id)
            .expect("span should exist");

        let span_extensions = span.extensions();
        
        let span_data = span_extensions
            .get::<Data>()
            .expect("data should exist");      

        let _ = self.print_exit(span.metadata().level(), span.name(), span_data, span.metadata().file(), span.metadata().line());
    }

    fn on_close(&self, id: span::Id, ctx: Context<'_, S>) {
        let span = ctx.span(&id)
            .expect("span should exist");

        let span_extensions = span.extensions();
        
        let span_data = span_extensions
            .get::<Data>()
            .expect("data should exist");

        let _ = self.print_close(span.metadata().level(), span_data);

        
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
            let _ = self.print_event_with_span(span_data, event);
        } else {
            let _ = self.print_event_without_span(event);
        }
    }
}

enum PrintColor {
    Green,
    Red,
    Orange,
    Yellow,
    Default
}

impl PrintTreeLayer {
    fn format_duration(duration: Duration, ansi: bool) -> color_eyre::Result<String> {
        let mut buff = String::new();
        let total_seconds = duration.whole_seconds();
        let nanos = duration.subsec_nanoseconds();

        let print_color: PrintColor;

        if total_seconds > 0 {
            write!(buff, "{}", total_seconds)?;
            if nanos > 0 {
                let fractional_seconds = nanos as f64 / 1_000_000_000.0;
                write!(buff, ".{}", format!("{:.3}", fractional_seconds).trim_start_matches("0.").trim_end_matches('0'))?;
            }
            write!(buff, "s")?;
            print_color = PrintColor::Red;
        } else if nanos >= 1_000_000 {
            let fractional_milliseconds = nanos as f64 / 1_000_000.0;
            write!(buff, "{}", format!("{:.3}", fractional_milliseconds).trim_end_matches('0').trim_end_matches('.'))?;
            write!(buff, "ms")?;
            print_color = PrintColor::Orange;
        } else if nanos >= 1_000 {
            let fractional_microseconds = nanos as f64 / 1_000.0;
            write!(buff, "{}", format!("{:.3}", fractional_microseconds).trim_end_matches('0').trim_end_matches('.'))?;
            write!(buff, "µs")?;
            print_color = PrintColor::Yellow;
        } else {
            write!(buff, "{}ns", nanos)?;
            print_color = PrintColor::Green;
        }

        if ansi {
            let str = match print_color {
                PrintColor::Green => format!("{}", style(buff).green().dim()),
                PrintColor::Red => format!("{}", style(buff).red().dim()),
                PrintColor::Orange => format!("{}", style(buff).color256(166).dim()),
                PrintColor::Yellow => format!("{}", style(buff).yellow().dim()),
                PrintColor::Default => format!("{}", style(buff).dim()),
            };
            Ok(str)
        } else {
            Ok(buff)
        }
    }

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

    fn print_enter(&self, level: &Level, name: &str, data: &Data) -> color_eyre::Result<()> {
        Self::print_prefix(level)?;

        let mut f = std::io::stdout();
        for i in 1..=data.level {
            if i == data.level {
                if i > 1 { write!(f, " ")?; }
                write!(f, "├ ")?;
            } else {
                if i > 1 { write!(f, " ")?; }
                write!(f, "│")?;
            }
        }
        write!(f, "{}", style("⥂ ").green())?;
        write!(f, "{} ", style(name).bold())?;

        for field in data.fields.iter() {
            write!(f, "{}=", style(field.0).italic().cyan().dim())?;
            write!(f, "{} ", style(field.1).dim().italic())?;
        }

        write!(f, "\r\n")?;

        Ok(())
    }

    fn print_exit(&self, level: &Level, name: &str, data: &Data, file: Option<&str>, line_no: Option<u32>) -> color_eyre::Result<()> {
        Self::print_prefix(level)?;

        let mut f = std::io::stdout();
        for i in 1..=data.level {
            if i == data.level {
                if i > 1 { write!(f, " ")?; }
                write!(f, "├ ")?;
            } else {
                if i > 1 { write!(f, " ")?; }
                write!(f, "│")?;
            }
        }
        write!(f, "{}", style("⥄ ").red())?;
        write!(f, "{} ", style(name).bold())?;

        if data.has_had_children {
            for field in data.fields.iter() {
                write!(f, "{}=", style(field.0).italic().cyan().dim())?;
                write!(f, "{} ", style(field.1).dim().italic())?;
            }
        }
        
        /*if let Some(file) = file {
            write!(f, "{}", style(file).magenta().italic().dim())?;
            if let Some(line_no) = line_no {
                write!(f, "{}", style(":").italic().dim())?;
                write!(f, "{}", style(line_no).magenta().italic().dim())?;
            }
        }*/

        write!(f, "\r\n")?;

        Ok(())
    }

    fn print_close(&self, level: &Level, data: &Data) -> color_eyre::Result<()> {
        Self::print_prefix(level)?;

        let mut f = std::io::stdout();

        for i in 1..=data.level {
            if i == data.level {
                if i > 1 { write!(f, " ")?; }
                write!(f, "┊ ")?;
            } else {
                if i > 1 { write!(f, " ")?; }
                write!(f, "│")?;
            }
        }

        write!(f, "{}", style("⟳").dim())?;
        write!(f, "{}", style(" runtime [").dim().bold())?;
        write!(f, "{}", style("clock ").cyan().bold().dim())?;
        write!(f, "{}", style("⇝ total: ").dim())?;
        write!(f, "{}", Self::format_duration(data.real_accumulated_time, true)?)?;
        write!(f, "{}", style(" busy: ").dim())?;
        write!(f, "{}", Self::format_duration(data.real_accumulated_time - data.real_wait_time, true)?)?;
        write!(f, "{}", style(" wait: ").dim())?;
        write!(f, "{}", Self::format_duration(data.real_wait_time, true)?)?;

        write!(f, "{}", style("] [").dim().bold())?;
        write!(f, "{}", style("cpu ").cyan().bold().dim())?;
        write!(f, "{}", style("⇝ busy: ").dim())?;
        write!(f, "{}", Self::format_duration(data.cpu_accumulated_time - data.cpu_wait_time, true)?)?;
        write!(f, "{}", style(" wait: ").dim())?;
        write!(f, "{}", Self::format_duration(data.cpu_wait_time, true)?)?;
        write!(f, "{}", style("]").dim().bold())?;

        write!(f, "\r\n")?;

        Ok(())
    }

    fn print_event_with_span(&self, span_data: &Data, event: &Event) -> color_eyre::Result<()> {
        Self::print_prefix(event.metadata().level())?;

        let mut f = std::io::stdout();

        for i in 1..=span_data.level {
            if i > 100 {
                panic!("exceeded max span depth");
            }
            if i == span_data.level {
                if i > 1 { write!(f, " ")?; }
                write!(f, "├ ")?;
            } else {
                if i > 1 { write!(f, " ")?; }
                write!(f, "│")?;
            }
        }

        let mut event_data = Data::default();
        event.record(&mut event_data);
    
        write!(f, "{}", style("⚡ ").yellow().dim())?;
        write!(f, "{}", event_data.fields["message"])?;
        write!(f, "\r\n")?;

        Ok(())
    }

    fn print_event_without_span(&self, event: &Event) -> color_eyre::Result<()> {
        Self::print_prefix(event.metadata().level())?;

        let mut f = std::io::stdout();

        let mut event_data = Data::default();
        event.record(&mut event_data);

        write!(f, "{}", style("⚡ ").yellow().dim())?;
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

#[cfg(test)]
mod tests {
    use time::Duration;

    use crate::telemetry::PrintTreeLayer;

    #[test]
    fn test_format_duration_seconds() {
        let duration = Duration::seconds(1) + Duration::nanoseconds(500_000_000);
        let formatted = PrintTreeLayer::format_duration(duration, false).unwrap();
        assert_eq!(formatted, "1.5s");

        let duration = Duration::seconds(1) + Duration::nanoseconds(250_000_000);
        let formatted = PrintTreeLayer::format_duration(duration, false).unwrap();
        assert_eq!(formatted, "1.25s");

        // Test truncating to three decimal places with rounding.
        let duration = Duration::seconds(1) + Duration::nanoseconds(678_900_000);
        let formatted = PrintTreeLayer::format_duration(duration, false).unwrap();
        assert_eq!(formatted, "1.679s");

        let duration = Duration::seconds(1);
        let formatted = PrintTreeLayer::format_duration(duration, false).unwrap();
        assert_eq!(formatted, "1s");

        let duration = Duration::milliseconds(1);
        let formatted = PrintTreeLayer::format_duration(duration, false).unwrap();
        // We can't just check for 's' here because it could be 'ms', 'μs' or 'ns'.
        assert!(formatted.ends_with("ms"));
    }

    #[test]
    fn test_format_duration_milliseconds() {
        let duration = Duration::milliseconds(1);
        let formatted = PrintTreeLayer::format_duration(duration, false).unwrap();
        assert_eq!(formatted, "1ms");

        let duration = Duration::milliseconds(1) + Duration::microseconds(500);
        let formatted = PrintTreeLayer::format_duration(duration, false).unwrap();
        assert_eq!(formatted, "1.5ms");

        let duration = Duration::milliseconds(1) + Duration::microseconds(123);
        let formatted = PrintTreeLayer::format_duration(duration, false).unwrap();
        assert_eq!(formatted, "1.123ms");

        // Test truncating to three decimal places with rounding.
        let duration = Duration::milliseconds(1) + Duration::nanoseconds(678_900);
        let formatted = PrintTreeLayer::format_duration(duration, false).unwrap();
        assert_eq!(formatted, "1.679ms");

        let duration = Duration::seconds(1);
        let formatted = PrintTreeLayer::format_duration(duration, false).unwrap();
        assert!(!formatted.ends_with("ms"));
    }

    #[test]
    fn test_format_duration_microseconds() {
        let duration = Duration::microseconds(1);
        let formatted = PrintTreeLayer::format_duration(duration, false).unwrap();
        assert_eq!(formatted, "1µs");

        let duration = Duration::microseconds(1) + Duration::nanoseconds(500);
        let formatted = PrintTreeLayer::format_duration(duration, false).unwrap();
        assert_eq!(formatted, "1.5µs");

        let duration = Duration::microseconds(1) + Duration::nanoseconds(123);
        let formatted = PrintTreeLayer::format_duration(duration, false).unwrap();
        assert_eq!(formatted, "1.123µs");

        // Test truncating to three decimal places with rounding.
        let duration = Duration::microseconds(1) + Duration::nanoseconds(123);
        let formatted = PrintTreeLayer::format_duration(duration, false).unwrap();
        assert_eq!(formatted, "1.123µs");
    }
}