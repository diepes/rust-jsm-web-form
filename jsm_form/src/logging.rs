use std::fmt;

use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;

/// Custom formatter that prints level, module path, function name, and message text.
pub struct FunctionFormatter;

impl Default for FunctionFormatter {
    fn default() -> Self {
        Self
    }
}

struct EventVisitor {
    message: Option<String>,
    function: Option<String>,
    other_fields: Vec<(String, String)>,
}

impl EventVisitor {
    fn new() -> Self {
        Self {
            message: None,
            function: None,
            other_fields: Vec::new(),
        }
    }

    fn clean(value: String) -> String {
        if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
            value[1..value.len() - 1].to_string()
        } else {
            value
        }
    }
}

impl Visit for EventVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let raw = format!("{value:?}");
        let cleaned = Self::clean(raw);
        match field.name() {
            "message" => self.message = Some(cleaned),
            "function" => self.function = Some(cleaned),
            name => self.other_fields.push((name.to_string(), cleaned)),
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        let cleaned = value.to_string();
        match field.name() {
            "message" => self.message = Some(cleaned),
            "function" => self.function = Some(cleaned),
            name => self.other_fields.push((name.to_string(), cleaned)),
        }
    }
}

impl<S, N> FormatEvent<S, N> for FunctionFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let metadata = event.metadata();
        let module_path = metadata.module_path().unwrap_or_else(|| metadata.target());
        let level = metadata.level();

        let mut visitor = EventVisitor::new();
        event.record(&mut visitor);

        write!(writer, "{level} {module_path}")?;

        if let Some(function) = visitor.function {
            write!(writer, "::{function}")?;
        }

        if let Some(span) = ctx.lookup_current() {
            let mut span_stack = Vec::new();
            let mut current = Some(span);
            while let Some(span) = current {
                span_stack.push(span.name());
                current = span.parent();
            }
            if !span_stack.is_empty() {
                span_stack.reverse();
                write!(writer, " [{}]", span_stack.join("::"))?;
            }
        }

        write!(writer, ":")?;

        if let Some(message) = visitor.message {
            write!(writer, " {message}")?;
        }

        for (name, value) in visitor.other_fields {
            write!(writer, " {name}={value}")?;
        }

        writeln!(writer)
    }
}

/// Initialize tracing subscriber with the custom formatter and environment filter support.
pub fn init_logging() {
    use tracing_subscriber::EnvFilter;

    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .event_format(FunctionFormatter::default())
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::{Arc, Mutex};
    use tracing::subscriber::DefaultGuard;
    use tracing_subscriber::fmt::MakeWriter;

    #[derive(Clone, Default)]
    struct BufferWriter {
        buffer: Arc<Mutex<String>>,
    }

    struct BufferGuard {
        buffer: Arc<Mutex<String>>,
    }

    impl<'a> MakeWriter<'a> for BufferWriter {
        type Writer = BufferGuard;

        fn make_writer(&'a self) -> Self::Writer {
            BufferGuard {
                buffer: Arc::clone(&self.buffer),
            }
        }
    }

    impl Write for BufferGuard {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            let mut lock = self.buffer.lock().unwrap();
            lock.push_str(&String::from_utf8_lossy(buf));
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl BufferWriter {
        fn contents(&self) -> String {
            self.buffer.lock().unwrap().clone()
        }
    }

    fn install_test_subscriber() -> (BufferWriter, DefaultGuard) {
        let writer = BufferWriter::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(writer.clone())
            .event_format(FunctionFormatter::default())
            .finish();
        let guard = tracing::subscriber::set_default(subscriber);
        (writer, guard)
    }

    #[test]
    fn function_name_includes_module_and_level() {
        let (writer, guard) = install_test_subscriber();
        // Emit a log using the exported macro to ensure function detection works.
        crate::log_info!("sample message");
        drop(guard);

        let output = writer.contents();
        assert!(output.contains("INFO"), "output was: {output:?}");
        assert!(
            output.contains("jsm_form::logging::tests::function_name_includes_module_and_level"),
            "output missing module/function: {output:?}"
        );
        assert!(output.contains("sample message"), "output missing message: {output:?}");
    }
}

#[macro_export]
#[doc(hidden)]
macro_rules! __log_function_path {
    () => {{
        fn __type_name_of<T>(_value: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = __type_name_of(|| {});
        match name.find("::{{closure") {
            Some(index) => &name[..index],
            None => name,
        }
    }};
}

#[macro_export]
macro_rules! log_trace {
    ($($arg:tt)*) => {{
        tracing::trace!(function = %$crate::__log_function_path!(), $($arg)*);
    }};
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => {{
        tracing::debug!(function = %$crate::__log_function_path!(), $($arg)*);
    }};
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {{
        tracing::info!(function = %$crate::__log_function_path!(), $($arg)*);
    }};
}

#[macro_export]
macro_rules! log_warn {
    ($($arg:tt)*) => {{
        tracing::warn!(function = %$crate::__log_function_path!(), $($arg)*);
    }};
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {{
        tracing::error!(function = %$crate::__log_function_path!(), $($arg)*);
    }};
}
