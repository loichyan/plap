use proc_macro2::Ident;

#[derive(Debug)]
pub struct Arg<T> {
    #[cfg(not(feature = "string"))]
    name: &'static str,
    #[cfg(feature = "string")]
    name: crate::str::Str,
    keys: Vec<Ident>,
    values: Vec<T>,
}

impl<T> Arg<T> {
    pub fn new(name: &'static str) -> Self {
        #[allow(clippy::useless_conversion)]
        Self {
            #[cfg(not(feature = "string"))]
            name,
            #[cfg(feature = "string")]
            name: name.into(),
            keys: <_>::default(),
            values: <_>::default(),
        }
    }

    #[cfg(feature = "string")]
    pub fn from_string(name: impl Into<String>) -> Self {
        Self {
            name: crate::str::Str::from(name.into()),
            keys: <_>::default(),
            values: <_>::default(),
        }
    }

    pub fn name(&self) -> &str {
        #[cfg(not(feature = "string"))]
        return self.name;
        #[cfg(feature = "string")]
        return self.name.as_str();
    }

    pub fn keys(&self) -> &[Ident] {
        &self.keys
    }

    pub fn values(&self) -> &[T] {
        &self.values
    }

    pub fn add(&mut self, key: Ident, value: T) {
        self.keys.push(key);
        self.values.push(value);
    }

    pub fn clear(&mut self) {
        self.keys.clear();
        self.values.clear();
    }
}
