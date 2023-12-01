/// A simple macro for aesthetics: `ok!()` instead of `Ok(())`
#[macro_export]
macro_rules! ok {
    () => {
        Ok(())
    };
}

#[macro_export]
macro_rules! trace_sql {
    // trace_sql!(target: "my_target", key1 = 42, key2 = true; "a {} event", "log")
    // trace_sql!(target: "my_target", "a {} event", "log")
    (target: $target:expr, $($arg:tt)+) => (if std::env::var_os("sql_trace").is_some() { ::log::log!(target: $target, ::log::Level::Trace, $($arg)+); });

    // trace_sql!("a {} event", "log")
    ($($arg:tt)+) => (if std::env::var_os("sql_trace").is_some() { ::log::log!(::log::Level::Trace, $($arg)+); })
}
