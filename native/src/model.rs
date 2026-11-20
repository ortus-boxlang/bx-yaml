//! The values YAML can hold, independent of the MatchBox VM, so reading and writing can be tested on their own.

/// A point in time and the zone it is shown in, as the VM's `datetime.v1` capability carries it.
#[derive(Debug, Clone, PartialEq)]
pub struct DateTimeValue {
    /// Whole seconds since 1970-01-01T00:00:00Z.
    pub seconds: i64,
    pub nanos: u32,
    /// `UTC`, `+HH:MM`, an IANA name, or empty for "the system zone".
    pub zone: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    DateTime(DateTimeValue),
    Array(Vec<Value>),
    /// Keys in document order, with the spelling they were written in.
    Table(Vec<(String, Value)>),
}

/// Which BoxLang exception a failure becomes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The text is not YAML the module can read.
    Parse,
    /// A value that YAML cannot hold.
    Serialization,
    /// A file could not be read or written.
    Io,
}

impl Kind {
    /// The exception type scripts see, when it is not the generic one. SnakeYAML's parse errors are plain Java exceptions
    /// with no BoxLang type, so they stay generic here too; file errors are `BoxIOException`s, whose `type` is `Application`.
    pub fn exception_type(self) -> Option<&'static str> {
        match self {
            Kind::Io => Some("Application"),
            Kind::Parse | Kind::Serialization => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Failure {
    pub kind: Kind,
    pub message: String,
}

impl Failure {
    pub fn new(kind: Kind, message: impl Into<String>) -> Self {
        Self { kind, message: message.into() }
    }
}

pub type Result<T> = std::result::Result<T, Failure>;
