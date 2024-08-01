use proc_macro2::Ident;

#[derive(Debug)]
pub struct Arg<T> {
    #[cfg(feature = "string")]
    name: crate::str::Str,
    #[cfg(not(feature = "string"))]
    name: &'static str,
    keys: Vec<Ident>,
    values: Vec<T>,
}

impl<T> Arg<T> {
    pub fn new(name: &'static str) -> Self {
        #[allow(clippy::useless_conversion)]
        Self {
            #[cfg(feature = "string")]
            name: name.into(),
            #[cfg(not(feature = "string"))]
            name,
            keys: <_>::default(),
            values: <_>::default(),
        }
    }

    #[cfg(feature = "string")]
    #[cfg_attr(docsrs, doc(cfg(feature = "string")))]
    pub fn from_string(name: impl Into<String>) -> Self {
        Self {
            name: crate::str::Str::from(name.into()),
            keys: <_>::default(),
            values: <_>::default(),
        }
    }

    pub fn name(&self) -> &str {
        #[cfg(feature = "string")]
        return self.name.as_str();
        #[cfg(not(feature = "string"))]
        return self.name;
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
