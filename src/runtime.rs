// Flux runtime - values, environment, and execution state.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt;
use std::io::{self, BufRead, Write};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::time::{FluxDate, FluxDateTime, FluxDuration, FluxInstant, FluxTask, FluxTime};

/// A Flux type descriptor — represents the type of a Flux value.
#[derive(Debug, Clone, PartialEq)]
pub enum FluxType {
    Nil,
    Bool,
    Int,
    Float,
    String,
    Array(Option<Box<FluxType>>),
    Map(Option<Box<FluxType>>, Option<Box<FluxType>>),
    Function {
        params: Vec<FluxType>,
        ret: Box<FluxType>,
    },
    Range,
    Duration,
    Instant,
    Date,
    Time,
    DateTime,
    Task(Option<Box<FluxType>>),
    Channel(Option<Box<FluxType>>),
    Event(Option<Box<FluxType>>),
    Error,
    Any,
    Union(Vec<FluxType>),
    Optional(Box<FluxType>),
}

impl fmt::Display for FluxType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FluxType::Nil => write!(f, "Nil"),
            FluxType::Bool => write!(f, "Bool"),
            FluxType::Int => write!(f, "Int"),
            FluxType::Float => write!(f, "Float"),
            FluxType::String => write!(f, "String"),
            FluxType::Array(None) => write!(f, "Array"),
            FluxType::Array(Some(inner)) => write!(f, "Array<{}>", inner),
            FluxType::Map(None, None) => write!(f, "Map"),
            FluxType::Map(Some(k), Some(v)) => write!(f, "Map<{}, {}>", k, v),
            FluxType::Map(_, _) => write!(f, "Map"),
            FluxType::Function { params, ret } => {
                write!(f, "(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", p)?;
                }
                write!(f, ") -> {}", ret)
            }
            FluxType::Range => write!(f, "Range"),
            FluxType::Duration => write!(f, "Duration"),
            FluxType::Instant => write!(f, "Instant"),
            FluxType::Date => write!(f, "Date"),
            FluxType::Time => write!(f, "Time"),
            FluxType::DateTime => write!(f, "DateTime"),
            FluxType::Task(None) => write!(f, "Task"),
            FluxType::Task(Some(inner)) => write!(f, "Task<{}>", inner),
            FluxType::Channel(None) => write!(f, "Channel"),
            FluxType::Channel(Some(inner)) => write!(f, "Channel<{}>", inner),
            FluxType::Event(None) => write!(f, "Event"),
            FluxType::Event(Some(inner)) => write!(f, "Event<{}>", inner),
            FluxType::Error => write!(f, "Error"),
            FluxType::Any => write!(f, "Any"),
            FluxType::Union(types) => {
                for (i, t) in types.iter().enumerate() {
                    if i > 0 {
                        write!(f, " | ")?;
                    }
                    write!(f, "{}", t)?;
                }
                Ok(())
            }
            FluxType::Optional(inner) => write!(f, "{}?", inner),
        }
    }
}

impl FluxType {
    /// Check if a value's runtime type is compatible with this type descriptor.
    pub fn is_compatible(&self, value_type: &FluxType) -> bool {
        if *self == FluxType::Any || *value_type == FluxType::Any {
            return true;
        }
        if *self == *value_type {
            return true;
        }
        // Int and Float are compatible (numeric coercion)
        if (*self == FluxType::Int && *value_type == FluxType::Float)
            || (*self == FluxType::Float && *value_type == FluxType::Int)
        {
            return true;
        }
        // Optional<T> is compatible with T and Nil
        if let FluxType::Optional(inner) = self {
            if *value_type == FluxType::Nil || inner.is_compatible(value_type) {
                return true;
            }
        }
        // Union compatibility
        if let FluxType::Union(types) = self {
            return types.iter().any(|t| t.is_compatible(value_type));
        }
        // Array compatibility (unparameterized Array matches any Array)
        if let (FluxType::Array(None), FluxType::Array(_)) = (self, value_type) {
            return true;
        }
        if let (FluxType::Array(_), FluxType::Array(None)) = (self, value_type) {
            return true;
        }
        // Map compatibility
        if let (FluxType::Map(None, None), FluxType::Map(_, _)) = (self, value_type) {
            return true;
        }
        if let (FluxType::Map(_, _), FluxType::Map(None, None)) = (self, value_type) {
            return true;
        }
        false
    }

    /// Parse a type name string into a FluxType.
    pub fn from_name(name: &str) -> Option<FluxType> {
        match name {
            "Nil" => Some(FluxType::Nil),
            "Bool" => Some(FluxType::Bool),
            "Int" => Some(FluxType::Int),
            "Float" => Some(FluxType::Float),
            "String" => Some(FluxType::String),
            "Array" => Some(FluxType::Array(None)),
            "Map" => Some(FluxType::Map(None, None)),
            "Function" => Some(FluxType::Function {
                params: vec![],
                ret: Box::new(FluxType::Any),
            }),
            "Range" => Some(FluxType::Range),
            "Duration" => Some(FluxType::Duration),
            "Instant" => Some(FluxType::Instant),
            "Date" => Some(FluxType::Date),
            "Time" => Some(FluxType::Time),
            "DateTime" => Some(FluxType::DateTime),
            "Task" => Some(FluxType::Task(None)),
            "Channel" => Some(FluxType::Channel(None)),
            "Event" => Some(FluxType::Event(None)),
            "Error" => Some(FluxType::Error),
            "Any" => Some(FluxType::Any),
            _ => None,
        }
    }
}

/// Get the FluxType of a runtime Value.
pub fn type_of(value: &Value) -> FluxType {
    match value {
        Value::String(_) => FluxType::String,
        Value::Integer(_) => FluxType::Int,
        Value::Float(_) => FluxType::Float,
        Value::Boolean(_) => FluxType::Bool,
        Value::Nil => FluxType::Nil,
        Value::Array(_) => FluxType::Array(None),
        Value::Map(_) => FluxType::Map(None, None),
        Value::Function(_) => FluxType::Function {
            params: vec![],
            ret: Box::new(FluxType::Any),
        },
        Value::Range(_) => FluxType::Range,
        Value::Instant(_) => FluxType::Instant,
        Value::Duration(_) => FluxType::Duration,
        Value::Date(_) => FluxType::Date,
        Value::Time(_) => FluxType::Time,
        Value::DateTime(_) => FluxType::DateTime,
        Value::Task(_) => FluxType::Task(None),
        Value::Error(_) => FluxType::Error,
        Value::Event(_) => FluxType::Event(None),
        Value::Channel(_) => FluxType::Channel(None),
        Value::Struct(_) => FluxType::Any, // User-defined types use Any for now
    }
}

/// A runtime value in the Flux language.
#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Nil,
    Array(Rc<RefCell<Vec<Value>>>),
    Map(Rc<RefCell<Vec<(Value, Value)>>>),
    Function(FluxFunction),
    Range(FluxRange),
    Instant(FluxInstant),
    Duration(FluxDuration),
    Date(FluxDate),
    Time(FluxTime),
    DateTime(FluxDateTime),
    Task(FluxTask),
    Error(FluxError),
    Event(FluxEvent),
    Channel(FluxChannel),
    Struct(FluxStruct),
}

/// A Flux struct value — an instance of a user-defined structured type.
#[derive(Debug, Clone)]
pub struct FluxStruct {
    pub type_name: String,
    pub fields: Rc<RefCell<Vec<(String, Value)>>>,
}

/// A Flux channel — a FIFO message-passing queue between computations.
/// Thread-safe: uses Arc<Mutex<...>> for cross-thread communication.
#[derive(Debug, Clone)]
pub struct FluxChannel {
    pub id: u64,
    pub buffer: Arc<Mutex<Vec<Value>>>,
    pub closed: Arc<Mutex<bool>>,
}

impl FluxChannel {
    pub fn new(id: u64) -> Self {
        FluxChannel {
            id,
            buffer: Arc::new(Mutex::new(Vec::new())),
            closed: Arc::new(Mutex::new(false)),
        }
    }

    pub fn send(&self, value: Value) -> Result<(), String> {
        if *self.closed.lock().unwrap() {
            return Err("cannot send on closed channel".to_string());
        }
        self.buffer.lock().unwrap().push(value);
        Ok(())
    }

    pub fn receive(&self) -> Option<Value> {
        let mut buf = self.buffer.lock().unwrap();
        if buf.is_empty() {
            None
        } else {
            Some(buf.remove(0))
        }
    }

    pub fn close(&self) {
        *self.closed.lock().unwrap() = true;
    }

    pub fn is_closed(&self) -> bool {
        *self.closed.lock().unwrap()
    }

    pub fn len(&self) -> usize {
        self.buffer.lock().unwrap().len()
    }
}

/// A Flux event value — represents an occurrence with a type, payload, and timestamp.
#[derive(Debug, Clone)]
pub struct FluxEvent {
    /// The event type/name (e.g. "message", "tick", "user.created").
    pub event_type: String,
    /// The event payload — any Flux value.
    pub payload: Box<Value>,
    /// Timestamp when the event was created.
    pub timestamp: FluxInstant,
}

/// A Flux error value.
#[derive(Debug, Clone)]
pub struct FluxError {
    pub message: String,
}

impl fmt::Display for FluxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// A Flux range value.
#[derive(Debug, Clone)]
pub struct FluxRange {
    pub start: i64,
    pub end: i64,
    pub inclusive: bool,
}

/// A Flux function value (named or anonymous, with closure).
#[derive(Debug, Clone)]
pub struct FluxFunction {
    /// Optional name (None for anonymous functions).
    pub name: Option<String>,
    /// Parameter patterns.
    pub params: Vec<crate::ast::Pattern>,
    /// The function body.
    pub body: crate::ast::Block,
    /// The environment captured at the point of function creation (closure).
    pub closure_env: Environment,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Integer(a), Value::Integer(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::Boolean(a), Value::Boolean(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            (Value::Array(a), Value::Array(b)) => Rc::ptr_eq(a, b),
            (Value::Map(a), Value::Map(b)) => Rc::ptr_eq(a, b),
            (Value::Function(a), Value::Function(b)) => std::ptr::eq(a as *const _, b as *const _),
            (Value::Range(a), Value::Range(b)) => {
                a.start == b.start && a.end == b.end && a.inclusive == b.inclusive
            }
            (Value::Instant(a), Value::Instant(b)) => a == b,
            (Value::Duration(a), Value::Duration(b)) => a == b,
            (Value::Date(a), Value::Date(b)) => a == b,
            (Value::Time(a), Value::Time(b)) => a == b,
            (Value::DateTime(a), Value::DateTime(b)) => a == b,
            (Value::Task(a), Value::Task(b)) => a.id == b.id,
            (Value::Error(a), Value::Error(b)) => a.message == b.message,
            (Value::Event(a), Value::Event(b)) => {
                a.event_type == b.event_type && a.payload == b.payload
            }
            (Value::Channel(a), Value::Channel(b)) => a.id == b.id,
            (Value::Struct(a), Value::Struct(b)) => {
                a.type_name == b.type_name && Rc::ptr_eq(&a.fields, &b.fields)
            }
            _ => false,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::String(s) => write!(f, "{}", s),
            Value::Integer(n) => write!(f, "{}", n),
            Value::Float(n) => {
                if *n == n.trunc() {
                    write!(f, "{:.1}", n)
                } else {
                    write!(f, "{}", n)
                }
            }
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Nil => write!(f, "nil"),
            Value::Array(elements) => {
                let elems = elements.borrow();
                write!(f, "[")?;
                for (i, elem) in elems.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    match elem {
                        Value::String(s) => write!(f, "\"{}\"", s)?,
                        _ => write!(f, "{}", elem)?,
                    }
                }
                write!(f, "]")
            }
            Value::Map(entries) => {
                let entries = entries.borrow();
                write!(f, "{{")?;
                for (i, (key, val)) in entries.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    match key {
                        Value::String(s) => write!(f, "\"{}\": ", s)?,
                        _ => write!(f, "{}: ", key)?,
                    }
                    match val {
                        Value::String(s) => write!(f, "\"{}\"", s)?,
                        _ => write!(f, "{}", val)?,
                    }
                }
                write!(f, "}}")
            }
            Value::Function(func) => match &func.name {
                Some(name) => write!(f, "<function {}>", name),
                None => write!(f, "<function>"),
            },
            Value::Range(r) => {
                if r.inclusive {
                    write!(f, "{}..{}", r.start, r.end)
                } else {
                    write!(f, "{}..<{}", r.start, r.end)
                }
            }
            Value::Instant(i) => write!(f, "{}", i),
            Value::Duration(d) => write!(f, "{}", d),
            Value::Date(d) => write!(f, "{}", d),
            Value::Time(t) => write!(f, "{}", t),
            Value::DateTime(dt) => write!(f, "{}", dt),
            Value::Task(t) => write!(f, "{}", t),
            Value::Error(e) => write!(f, "{}", e),
            Value::Event(ev) => {
                write!(f, "Event(\"{}\", ", ev.event_type)?;
                match ev.payload.as_ref() {
                    Value::String(s) => write!(f, "\"{}\"", s)?,
                    other => write!(f, "{}", other)?,
                }
                write!(f, ")")
            }
            Value::Channel(ch) => {
                if ch.is_closed() {
                    write!(f, "<channel {} closed>", ch.id)
                } else {
                    write!(f, "<channel {}>", ch.id)
                }
            }
            Value::Struct(s) => {
                write!(f, "{} {{", s.type_name)?;
                let fields = s.fields.borrow();
                for (i, (name, val)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ",")?;
                    }
                    write!(f, " {}: ", name)?;
                    match val {
                        Value::String(sv) => write!(f, "\"{}\"", sv)?,
                        other => write!(f, "{}", other)?,
                    }
                }
                if !fields.is_empty() {
                    write!(f, " ")?;
                }
                write!(f, "}}")
            }
        }
    }
}

impl Value {
    pub fn type_name(&self) -> &str {
        match self {
            Value::String(_) => "String",
            Value::Integer(_) => "Integer",
            Value::Float(_) => "Float",
            Value::Boolean(_) => "Boolean",
            Value::Nil => "Nil",
            Value::Array(_) => "Array",
            Value::Map(_) => "Map",
            Value::Function(_) => "Function",
            Value::Range(_) => "Range",
            Value::Instant(_) => "Instant",
            Value::Duration(_) => "Duration",
            Value::Date(_) => "Date",
            Value::Time(_) => "Time",
            Value::DateTime(_) => "DateTime",
            Value::Task(_) => "Task",
            Value::Error(_) => "Error",
            Value::Event(_) => "Event",
            Value::Channel(_) => "Channel",
            Value::Struct(s) => &s.type_name,
        }
    }

    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Boolean(b) => *b,
            Value::Integer(n) => *n != 0,
            Value::Float(n) => *n != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::Nil => false,
            Value::Array(a) => !a.borrow().is_empty(),
            Value::Map(m) => !m.borrow().is_empty(),
            Value::Function(_) => true,
            Value::Range(r) => {
                if r.inclusive {
                    true
                } else {
                    r.start != r.end
                }
            }
            Value::Instant(_) => true,
            Value::Duration(d) => d.nanos != 0,
            Value::Date(_) => true,
            Value::Time(_) => true,
            Value::DateTime(_) => true,
            Value::Task(_) => true,
            Value::Error(_) => true,
            Value::Event(_) => true,
            Value::Channel(_) => true,
            Value::Struct(_) => true,
        }
    }

    pub fn to_number(&self) -> Option<NumericValue> {
        match self {
            Value::Integer(n) => Some(NumericValue::Int(*n)),
            Value::Float(n) => Some(NumericValue::Flt(*n)),
            Value::Boolean(b) => Some(NumericValue::Int(if *b { 1 } else { 0 })),
            _ => None,
        }
    }

    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            Value::Integer(_) | Value::Float(_) | Value::Boolean(_)
        )
    }
}

/// A numeric value used during arithmetic coercion.
#[derive(Debug, Clone, Copy)]
pub enum NumericValue {
    Int(i64),
    Flt(f64),
}

/// Promote two numeric values to a common type for arithmetic.
/// If either is float, both become float.
pub fn promote(left: NumericValue, right: NumericValue) -> (f64, f64, bool) {
    match (left, right) {
        (NumericValue::Int(l), NumericValue::Int(r)) => (l as f64, r as f64, false),
        _ => {
            let l = match left {
                NumericValue::Int(n) => n as f64,
                NumericValue::Flt(n) => n,
            };
            let r = match right {
                NumericValue::Int(n) => n as f64,
                NumericValue::Flt(n) => n,
            };
            (l, r, true)
        }
    }
}

/// Trait for runtime output. Allows different backends (stdout, test capture, etc.)
pub trait Output {
    /// Write a value as a line of output (like `print`).
    fn write_line(&mut self, value: &Value);
    /// Write a prompt string without a trailing newline and flush.
    fn write_prompt(&mut self, s: &str);
}

/// Trait for runtime input. Allows different backends (stdin, test injection, etc.)
pub trait Input {
    /// Read one line of input (without trailing newline).
    /// Returns Err with a message on EOF or I/O error.
    fn read_line(&mut self) -> Result<String, String>;
}

/// Standard input runtime — reads from stdin.
pub struct StdInput;

impl Input for StdInput {
    fn read_line(&mut self) -> Result<String, String> {
        let mut line = String::new();
        match io::stdin().lock().read_line(&mut line) {
            Ok(0) => Err("input: unexpected end of input".to_string()),
            Ok(_) => {
                if line.ends_with('\n') {
                    line.pop();
                    if line.ends_with('\r') {
                        line.pop();
                    }
                }
                Ok(line)
            }
            Err(e) => Err(format!("input: I/O error: {}", e)),
        }
    }
}

/// A shared reference to an environment scope.
pub type EnvRef = Rc<RefCell<EnvInner>>;

/// The inner data of an environment scope.
#[derive(Debug)]
pub struct EnvInner {
    bindings: HashMap<String, Value>,
    parent: Option<EnvRef>,
}

/// The variable environment. A chain of scopes with shared references.
#[derive(Debug, Clone)]
pub struct Environment {
    inner: EnvRef,
}

impl Environment {
    /// Create a new empty top-level environment.
    pub fn new() -> Self {
        Environment {
            inner: Rc::new(RefCell::new(EnvInner {
                bindings: HashMap::new(),
                parent: None,
            })),
        }
    }

    /// Create a child scope that can look up variables in this one.
    /// The parent is shared via Rc, not moved.
    pub fn push_scope(&self) -> Self {
        Environment {
            inner: Rc::new(RefCell::new(EnvInner {
                bindings: HashMap::new(),
                parent: Some(Rc::clone(&self.inner)),
            })),
        }
    }

    /// Get the parent scope (for restoring after a function call).
    pub fn parent(&self) -> Option<Self> {
        let inner = self.inner.borrow();
        inner.parent.as_ref().map(|p| Environment {
            inner: Rc::clone(p),
        })
    }

    /// Define a new variable in the current (local) scope.
    /// Define a new variable in the current (local) scope.
    pub fn define(&self, name: &str, value: Value) -> Result<(), String> {
        let mut inner = self.inner.borrow_mut();
        if inner.bindings.contains_key(name) {
            Err(format!("variable '{}' is already defined", name))
        } else {
            inner.bindings.insert(name.to_string(), value);
            Ok(())
        }
    }

    /// Define or replace a variable in the current (local) scope.
    /// Used in REPL mode where redefinition is allowed.
    pub fn define_or_assign(&self, name: &str, value: Value) {
        self.inner
            .borrow_mut()
            .bindings
            .insert(name.to_string(), value);
    }

    /// Look up a variable by name, searching local then parent scopes.
    pub fn get(&self, name: &str) -> Option<Value> {
        let inner = self.inner.borrow();
        if let Some(val) = inner.bindings.get(name) {
            Some(val.clone())
        } else if let Some(parent) = &inner.parent {
            let parent_env = Environment {
                inner: Rc::clone(parent),
            };
            parent_env.get(name)
        } else {
            None
        }
    }

    /// Assign a new value to an existing variable, searching local then parent scopes.
    pub fn assign(&self, name: &str, value: Value) -> Result<(), String> {
        let mut inner = self.inner.borrow_mut();
        if inner.bindings.contains_key(name) {
            inner.bindings.insert(name.to_string(), value);
            Ok(())
        } else if let Some(parent) = &inner.parent {
            let parent_env = Environment {
                inner: Rc::clone(parent),
            };
            drop(inner); // release borrow before recursing
            parent_env.assign(name, value)
        } else {
            Err(format!("undefined variable '{}'", name))
        }
    }
}

/// Deep-clone a Value so it has no shared Rc references to the original.
pub fn deep_clone_value(val: &Value) -> Value {
    match val {
        Value::String(s) => Value::String(s.clone()),
        Value::Integer(n) => Value::Integer(*n),
        Value::Float(f) => Value::Float(*f),
        Value::Boolean(b) => Value::Boolean(*b),
        Value::Nil => Value::Nil,
        Value::Array(arr) => {
            let cloned: Vec<Value> = arr.borrow().iter().map(|v| deep_clone_value(v)).collect();
            Value::Array(Rc::new(RefCell::new(cloned)))
        }
        Value::Map(map) => {
            let cloned: Vec<(Value, Value)> = map
                .borrow()
                .iter()
                .map(|(k, v)| (deep_clone_value(k), deep_clone_value(v)))
                .collect();
            Value::Map(Rc::new(RefCell::new(cloned)))
        }
        Value::Function(f) => Value::Function(f.clone()),
        Value::Range(r) => Value::Range(r.clone()),
        Value::Instant(i) => Value::Instant(*i),
        Value::Duration(d) => Value::Duration(*d),
        Value::Date(d) => Value::Date(d.clone()),
        Value::Time(t) => Value::Time(t.clone()),
        Value::DateTime(dt) => Value::DateTime(dt.clone()),
        Value::Task(t) => Value::Task(t.clone()),
        Value::Error(e) => Value::Error(e.clone()),
        Value::Event(ev) => Value::Event(ev.clone()),
        Value::Channel(ch) => Value::Channel(ch.clone()), // Channels ARE shared (Arc-based)
        Value::Struct(s) => {
            let cloned_fields: Vec<(String, Value)> = s
                .fields
                .borrow()
                .iter()
                .map(|(k, v)| (k.clone(), deep_clone_value(v)))
                .collect();
            Value::Struct(FluxStruct {
                type_name: s.type_name.clone(),
                fields: Rc::new(RefCell::new(cloned_fields)),
            })
        }
    }
}

impl Environment {
    /// Deep-clone the entire environment chain into a completely independent copy.
    /// The result has no Rc references to the original.
    pub fn deep_clone(&self) -> Self {
        let inner = self.inner.borrow();
        let cloned_bindings: HashMap<String, Value> = inner
            .bindings
            .iter()
            .map(|(k, v)| (k.clone(), deep_clone_value(v)))
            .collect();
        let cloned_parent = inner.parent.as_ref().map(|p| {
            let parent_env = Environment {
                inner: Rc::clone(p),
            };
            parent_env.deep_clone()
        });
        Environment {
            inner: Rc::new(RefCell::new(EnvInner {
                bindings: cloned_bindings,
                parent: cloned_parent.map(|e| e.inner),
            })),
        }
    }
}

/// A wrapper that allows sending a deep-cloned Environment + Block across threads.
/// SAFETY: The contained Environment has been deep-cloned so it shares no Rc references
/// with any other Environment. The Block is pure AST data (no Rc).
pub struct SendableTaskPayload {
    pub env: Environment,
    pub body: crate::ast::Block,
    pub task: crate::time::FluxTask,
}

// SAFETY: deep_clone guarantees no shared Rc references exist.
// The Environment's Rc<RefCell<...>> instances are unique to this payload.
// Block is composed of String/i64/Vec/enum — all Send.
// FluxTask uses Arc<Mutex<...>> — Send + Sync.
unsafe impl Send for SendableTaskPayload {}

/// A thread-safe output sink for worker threads.
pub struct ThreadOutput {
    lines: Arc<Mutex<Vec<String>>>,
}

impl ThreadOutput {
    pub fn new() -> Self {
        ThreadOutput {
            lines: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Output for ThreadOutput {
    fn write_line(&mut self, value: &Value) {
        self.lines.lock().unwrap().push(format!("{}", value));
    }

    fn write_prompt(&mut self, _s: &str) {
        // ThreadOutput does not handle prompts (spawned tasks)
    }
}

/// Standard output runtime — writes to stdout.
pub struct StdOutput;

impl Output for StdOutput {
    fn write_line(&mut self, value: &Value) {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        writeln!(handle, "{}", value).expect("failed to write to stdout");
    }

    fn write_prompt(&mut self, s: &str) {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        write!(handle, "{}", s).expect("failed to write to stdout");
        handle.flush().expect("failed to flush stdout");
    }
}

/// Test output runtime — captures output into a Vec for assertions.
#[cfg(test)]
pub struct TestOutput {
    pub lines: Vec<String>,
    pub prompts: Vec<String>,
}

#[cfg(test)]
impl TestOutput {
    pub fn new() -> Self {
        TestOutput {
            lines: Vec::new(),
            prompts: Vec::new(),
        }
    }
}

#[cfg(test)]
impl Output for TestOutput {
    fn write_line(&mut self, value: &Value) {
        self.lines.push(format!("{}", value));
    }

    fn write_prompt(&mut self, s: &str) {
        self.prompts.push(s.to_string());
    }
}

/// Test input runtime — provides lines from a pre-loaded queue.
#[cfg(test)]
pub struct TestInput {
    lines: std::collections::VecDeque<String>,
}

#[cfg(test)]
impl TestInput {
    pub fn new(lines: Vec<String>) -> Self {
        TestInput {
            lines: lines.into(),
        }
    }
}

#[cfg(test)]
impl Input for TestInput {
    fn read_line(&mut self) -> Result<String, String> {
        self.lines
            .pop_front()
            .ok_or_else(|| "input: unexpected end of input".to_string())
    }
}

/// Shared test output — uses Rc<RefCell> so output can be read while interpreter is alive.
#[cfg(test)]
pub struct SharedTestOutput {
    pub lines: Rc<RefCell<Vec<String>>>,
}

#[cfg(test)]
impl SharedTestOutput {
    pub fn new() -> Self {
        SharedTestOutput {
            lines: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn get_lines(&self) -> Vec<String> {
        self.lines.borrow().clone()
    }
}

#[cfg(test)]
impl Output for SharedTestOutput {
    fn write_line(&mut self, value: &Value) {
        self.lines.borrow_mut().push(format!("{}", value));
    }

    fn write_prompt(&mut self, _s: &str) {
        // SharedTestOutput does not capture prompts
    }
}
