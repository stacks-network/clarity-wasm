/// A simple macro for aesthetics: `ok!()` instead of `Ok(())`
#[macro_export]
macro_rules! ok {
    () => {
        Ok(())
    };
}
