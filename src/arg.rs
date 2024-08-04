use proc_macro2::Ident;

#[derive(Debug, Default)]
pub struct ArgAttrs {
    kind: ArgKind,
    optional: bool,
}

impl ArgAttrs {
    pub fn kind(&mut self, kind: ArgKind) -> &mut Self {
        self.kind = kind;
        self
    }

    pub fn is_expr(&mut self) -> &mut Self {
        self.kind(ArgKind::Expr)
    }

    pub fn is_flag(&mut self) -> &mut Self {
        self.kind(ArgKind::Flag)
    }

    pub fn is_token_tree(&mut self) -> &mut Self {
        self.kind(ArgKind::TokenTree)
    }

    pub fn is_help(&mut self) -> &mut Self {
        self.kind(ArgKind::Help)
    }

    pub fn optional(&mut self) -> &mut Self {
        self.optional = true;
        self
    }

    pub fn get_kind(&self) -> ArgKind {
        self.kind
    }

    pub fn get_optional(&self) -> bool {
        self.optional
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ArgKind {
    Expr,
    Flag,
    TokenTree,
    Help,
}

impl Default for ArgKind {
    fn default() -> Self {
        ArgKind::TokenTree
    }
}

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

    pub fn take_last(mut self) -> Option<T> {
        self.values.pop()
    }

    pub fn take_one(mut self) -> T {
        let val = self
            .values
            .pop()
            .unwrap_or_else(|| panic!("too few values provided"));
        if !self.values.is_empty() {
            panic!("too many values provided");
        }
        val
    }

    pub fn take_many(self) -> Vec<T> {
        if self.values.is_empty() {
            panic!("too few values provided");
        }
        self.values
    }

    pub fn take_any(self) -> Vec<T> {
        self.values
    }
}

impl Arg<syn::LitBool> {
    pub fn take_flag(self) -> bool {
        self.take_flag_or(false)
    }

    pub fn take_flag_or(self, default: bool) -> bool {
        self.take_last().map(|b| b.value()).unwrap_or(default)
    }
}
