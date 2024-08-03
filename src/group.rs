#[macro_export]
macro_rules! group {
    ($($member:expr),* $(,)?) => ([$($member as &dyn ::plap::AnyArg,)*]);
}
