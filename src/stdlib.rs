// Flux standard library — built-in functions.

use std::cell::RefCell;
use std::rc::Rc;

use crate::interpreter::RuntimeError;
use crate::lexer::Span;
use crate::runtime::Value;

/// Check if a name is a built-in function.
pub fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "print"
            | "length"
            | "keys"
            | "type"
            | "int"
            | "float"
            | "string"
            | "bool"
            | "is_nil"
            | "is_number"
            | "is_string"
            | "is_boolean"
            | "is_array"
            | "is_map"
            | "is_function"
            | "is_range"
            | "entries"
            | "upper"
            | "lower"
            | "trim"
            | "push"
            | "pop"
            | "contains"
            | "contains_key"
            | "remove_key"
            | "values"
            | "abs"
            | "min"
            | "max"
            | "floor"
            | "ceil"
            | "round"
    )
}

/// Execute a built-in function. `print` is handled separately by the interpreter
/// because it needs access to the output backend.
pub fn call_builtin(name: &str, args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    match name {
        "length" => builtin_length(args, span),
        "keys" => builtin_keys(args, span),
        "type" => builtin_type(args, span),
        "int" => builtin_int(args, span),
        "float" => builtin_float(args, span),
        "string" => builtin_string(args, span),
        "bool" => builtin_bool(args, span),
        "is_nil" => builtin_is_nil(args, span),
        "is_number" => builtin_is_number(args, span),
        "is_string" => builtin_is_string(args, span),
        "is_boolean" => builtin_is_boolean(args, span),
        "is_array" => builtin_is_array(args, span),
        "is_map" => builtin_is_map(args, span),
        "is_function" => builtin_is_function(args, span),
        "is_range" => builtin_is_range(args, span),
        "entries" => builtin_entries(args, span),
        "upper" => builtin_upper(args, span),
        "lower" => builtin_lower(args, span),
        "trim" => builtin_trim(args, span),
        "push" => builtin_push(args, span),
        "pop" => builtin_pop(args, span),
        "contains" => builtin_contains(args, span),
        "contains_key" => builtin_contains_key(args, span),
        "remove_key" => builtin_remove_key(args, span),
        "values" => builtin_values(args, span),
        "abs" => builtin_abs(args, span),
        "min" => builtin_min(args, span),
        "max" => builtin_max(args, span),
        "floor" => builtin_floor(args, span),
        "ceil" => builtin_ceil(args, span),
        "round" => builtin_round(args, span),
        _ => Err(RuntimeError {
            call_stack: Vec::new(),
            message: format!("unknown built-in '{}'", name),
            span: span.clone(),
        }),
    }
}

// --- Helpers ---

fn expect_args(name: &str, args: &[Value], count: usize, span: &Span) -> Result<(), RuntimeError> {
    if args.len() != count {
        Err(RuntimeError {
            call_stack: Vec::new(),
            message: format!(
                "{} expects {} argument(s) but got {}",
                name,
                count,
                args.len()
            ),
            span: span.clone(),
        })
    } else {
        Ok(())
    }
}

// --- Built-in implementations ---

fn builtin_length(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("length", &args, 1, span)?;
    match &args[0] {
        Value::Array(a) => Ok(Value::Integer(a.borrow().len() as i64)),
        Value::String(s) => Ok(Value::Integer(s.len() as i64)),
        Value::Map(m) => Ok(Value::Integer(m.borrow().len() as i64)),
        Value::Range(r) => {
            let len = if r.inclusive {
                (r.end - r.start).abs() + 1
            } else {
                (r.end - r.start).abs()
            };
            Ok(Value::Integer(len))
        }
        _ => Err(RuntimeError {
            call_stack: Vec::new(),
            message: format!("length not supported for {}", args[0].type_name()),
            span: span.clone(),
        }),
    }
}

fn builtin_keys(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("keys", &args, 1, span)?;
    match &args[0] {
        Value::Map(entries) => {
            let keys: Vec<Value> = entries.borrow().iter().map(|(k, _)| k.clone()).collect();
            Ok(Value::Array(Rc::new(RefCell::new(keys))))
        }
        _ => Err(RuntimeError {
            call_stack: Vec::new(),
            message: format!("keys not supported for {}", args[0].type_name()),
            span: span.clone(),
        }),
    }
}

fn builtin_type(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("type", &args, 1, span)?;
    Ok(Value::String(args[0].type_name().to_string()))
}

fn builtin_int(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("int", &args, 1, span)?;
    match &args[0] {
        Value::Integer(n) => Ok(Value::Integer(*n)),
        Value::Float(n) => {
            if !n.is_finite() {
                return Err(RuntimeError {
                    call_stack: Vec::new(),
                    message: "cannot convert Float to Integer: value is not finite".to_string(),
                    span: span.clone(),
                });
            }

            let truncated = n.trunc();
            if truncated < i64::MIN as f64 || truncated > i64::MAX as f64 {
                return Err(RuntimeError {
                    call_stack: Vec::new(),
                    message: "cannot convert Float to Integer: value out of range".to_string(),
                    span: span.clone(),
                });
            }

            Ok(Value::Integer(truncated as i64))
        }
        Value::Boolean(b) => Ok(Value::Integer(if *b { 1 } else { 0 })),
        Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return Err(RuntimeError {
                    call_stack: Vec::new(),
                    message: "cannot convert String to Integer".to_string(),
                    span: span.clone(),
                });
            }

            match trimmed.parse::<i64>() {
                Ok(n) => Ok(Value::Integer(n)),
                Err(_) => Err(RuntimeError {
                    call_stack: Vec::new(),
                    message: "cannot convert String to Integer".to_string(),
                    span: span.clone(),
                }),
            }
        }
        _ => Err(RuntimeError {
            call_stack: Vec::new(),
            message: format!("cannot convert {} to Integer", args[0].type_name()),
            span: span.clone(),
        }),
    }
}

fn builtin_float(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("float", &args, 1, span)?;
    match &args[0] {
        Value::Integer(n) => Ok(Value::Float(*n as f64)),
        Value::Float(n) => Ok(Value::Float(*n)),
        Value::Boolean(b) => Ok(Value::Float(if *b { 1.0 } else { 0.0 })),
        Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return Err(RuntimeError {
                    call_stack: Vec::new(),
                    message: "cannot convert String to Float".to_string(),
                    span: span.clone(),
                });
            }

            match trimmed.parse::<f64>() {
                Ok(n) if n.is_finite() => Ok(Value::Float(n)),
                _ => Err(RuntimeError {
                    call_stack: Vec::new(),
                    message: "cannot convert String to Float".to_string(),
                    span: span.clone(),
                }),
            }
        }
        _ => Err(RuntimeError {
            call_stack: Vec::new(),
            message: format!("cannot convert {} to Float", args[0].type_name()),
            span: span.clone(),
        }),
    }
}

fn builtin_string(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("string", &args, 1, span)?;
    Ok(Value::String(format!("{}", args[0])))
}

fn builtin_bool(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("bool", &args, 1, span)?;
    Ok(Value::Boolean(args[0].is_truthy()))
}

fn builtin_is_nil(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("is_nil", &args, 1, span)?;
    Ok(Value::Boolean(matches!(args[0], Value::Nil)))
}

fn builtin_is_number(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("is_number", &args, 1, span)?;
    Ok(Value::Boolean(matches!(
        args[0],
        Value::Integer(_) | Value::Float(_)
    )))
}

fn builtin_is_string(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("is_string", &args, 1, span)?;
    Ok(Value::Boolean(matches!(args[0], Value::String(_))))
}

fn builtin_is_boolean(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("is_boolean", &args, 1, span)?;
    Ok(Value::Boolean(matches!(args[0], Value::Boolean(_))))
}

fn builtin_is_array(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("is_array", &args, 1, span)?;
    Ok(Value::Boolean(matches!(args[0], Value::Array(_))))
}

fn builtin_is_map(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("is_map", &args, 1, span)?;
    Ok(Value::Boolean(matches!(args[0], Value::Map(_))))
}

fn builtin_is_function(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("is_function", &args, 1, span)?;
    Ok(Value::Boolean(matches!(args[0], Value::Function(_))))
}

fn builtin_is_range(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("is_range", &args, 1, span)?;
    Ok(Value::Boolean(matches!(args[0], Value::Range(_))))
}

fn builtin_entries(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("entries", &args, 1, span)?;
    match &args[0] {
        Value::Map(entries) => {
            let pairs: Vec<Value> = entries
                .borrow()
                .iter()
                .map(|(k, v)| Value::Array(Rc::new(RefCell::new(vec![k.clone(), v.clone()]))))
                .collect();
            Ok(Value::Array(Rc::new(RefCell::new(pairs))))
        }
        _ => Err(RuntimeError {
            call_stack: Vec::new(),
            message: format!("entries not supported for {}", args[0].type_name()),
            span: span.clone(),
        }),
    }
}

fn builtin_upper(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("upper", &args, 1, span)?;
    match &args[0] {
        Value::String(s) => Ok(Value::String(s.to_uppercase())),
        _ => Err(RuntimeError {
            call_stack: Vec::new(),
            message: format!("'upper' expects a String, got {}", args[0].type_name()),
            span: span.clone(),
        }),
    }
}

fn builtin_lower(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("lower", &args, 1, span)?;
    match &args[0] {
        Value::String(s) => Ok(Value::String(s.to_lowercase())),
        _ => Err(RuntimeError {
            call_stack: Vec::new(),
            message: format!("'lower' expects a String, got {}", args[0].type_name()),
            span: span.clone(),
        }),
    }
}

fn builtin_trim(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("trim", &args, 1, span)?;
    match &args[0] {
        Value::String(s) => Ok(Value::String(s.trim().to_string())),
        _ => Err(RuntimeError {
            call_stack: Vec::new(),
            message: format!("'trim' expects a String, got {}", args[0].type_name()),
            span: span.clone(),
        }),
    }
}

fn builtin_push(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("push", &args, 2, span)?;
    match &args[0] {
        Value::Array(elements) => {
            elements.borrow_mut().push(args[1].clone());
            Ok(Value::Nil)
        }
        _ => Err(RuntimeError {
            call_stack: Vec::new(),
            message: format!(
                "'push' expects an Array as first argument, got {}",
                args[0].type_name()
            ),
            span: span.clone(),
        }),
    }
}

fn builtin_pop(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("pop", &args, 1, span)?;
    match &args[0] {
        Value::Array(elements) => {
            let mut elems = elements.borrow_mut();
            if elems.is_empty() {
                Err(RuntimeError {
                    call_stack: Vec::new(),
                    message: "cannot pop from empty array".to_string(),
                    span: span.clone(),
                })
            } else {
                Ok(elems.pop().unwrap())
            }
        }
        _ => Err(RuntimeError {
            call_stack: Vec::new(),
            message: format!("'pop' expects an Array, got {}", args[0].type_name()),
            span: span.clone(),
        }),
    }
}

fn builtin_contains(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("contains", &args, 2, span)?;
    match &args[0] {
        Value::Array(elements) => {
            let elems = elements.borrow();
            let needle = &args[1];
            for elem in elems.iter() {
                if values_equal(elem, needle) {
                    return Ok(Value::Boolean(true));
                }
            }
            Ok(Value::Boolean(false))
        }
        _ => Err(RuntimeError {
            call_stack: Vec::new(),
            message: format!(
                "'contains' expects an Array as first argument, got {}",
                args[0].type_name()
            ),
            span: span.clone(),
        }),
    }
}

fn builtin_contains_key(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("contains_key", &args, 2, span)?;
    match &args[0] {
        Value::Map(entries) => {
            let entries = entries.borrow();
            let key = &args[1];
            for (k, _) in entries.iter() {
                if values_equal(k, key) {
                    return Ok(Value::Boolean(true));
                }
            }
            Ok(Value::Boolean(false))
        }
        _ => Err(RuntimeError {
            call_stack: Vec::new(),
            message: format!(
                "'contains_key' expects a Map as first argument, got {}",
                args[0].type_name()
            ),
            span: span.clone(),
        }),
    }
}

fn builtin_remove_key(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("remove_key", &args, 2, span)?;
    match &args[0] {
        Value::Map(entries) => {
            let mut entries = entries.borrow_mut();
            let key = &args[1];
            let pos = entries.iter().position(|(k, _)| values_equal(k, key));
            match pos {
                Some(i) => {
                    let (_, v) = entries.remove(i);
                    Ok(v)
                }
                None => Ok(Value::Nil),
            }
        }
        _ => Err(RuntimeError {
            call_stack: Vec::new(),
            message: format!(
                "'remove_key' expects a Map as first argument, got {}",
                args[0].type_name()
            ),
            span: span.clone(),
        }),
    }
}

fn builtin_values(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("values", &args, 1, span)?;
    match &args[0] {
        Value::Map(entries) => {
            let vals: Vec<Value> = entries.borrow().iter().map(|(_, v)| v.clone()).collect();
            Ok(Value::Array(Rc::new(RefCell::new(vals))))
        }
        _ => Err(RuntimeError {
            call_stack: Vec::new(),
            message: format!("'values' expects a Map, got {}", args[0].type_name()),
            span: span.clone(),
        }),
    }
}

fn builtin_abs(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("abs", &args, 1, span)?;
    match &args[0] {
        Value::Integer(n) => Ok(Value::Integer(n.abs())),
        Value::Float(n) => Ok(Value::Float(n.abs())),
        Value::Boolean(b) => Ok(Value::Integer(if *b { 1 } else { 0 })),
        _ => Err(RuntimeError {
            call_stack: Vec::new(),
            message: format!("'abs' expects a numeric value, got {}", args[0].type_name()),
            span: span.clone(),
        }),
    }
}

fn builtin_min(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("min", &args, 2, span)?;
    let a = to_f64(&args[0], "min", span)?;
    let b = to_f64(&args[1], "min", span)?;
    if a <= b {
        Ok(coerce_back(a, &args[0], &args[1]))
    } else {
        Ok(coerce_back(b, &args[0], &args[1]))
    }
}

fn builtin_max(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("max", &args, 2, span)?;
    let a = to_f64(&args[0], "max", span)?;
    let b = to_f64(&args[1], "max", span)?;
    if a >= b {
        Ok(coerce_back(a, &args[0], &args[1]))
    } else {
        Ok(coerce_back(b, &args[0], &args[1]))
    }
}

fn builtin_floor(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("floor", &args, 1, span)?;
    match &args[0] {
        Value::Integer(n) => Ok(Value::Integer(*n)),
        Value::Float(n) => Ok(Value::Integer(n.floor() as i64)),
        Value::Boolean(b) => Ok(Value::Integer(if *b { 1 } else { 0 })),
        _ => Err(RuntimeError {
            call_stack: Vec::new(),
            message: format!(
                "'floor' expects a numeric value, got {}",
                args[0].type_name()
            ),
            span: span.clone(),
        }),
    }
}

fn builtin_ceil(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("ceil", &args, 1, span)?;
    match &args[0] {
        Value::Integer(n) => Ok(Value::Integer(*n)),
        Value::Float(n) => Ok(Value::Integer(n.ceil() as i64)),
        Value::Boolean(b) => Ok(Value::Integer(if *b { 1 } else { 0 })),
        _ => Err(RuntimeError {
            call_stack: Vec::new(),
            message: format!(
                "'ceil' expects a numeric value, got {}",
                args[0].type_name()
            ),
            span: span.clone(),
        }),
    }
}

fn builtin_round(args: Vec<Value>, span: &Span) -> Result<Value, RuntimeError> {
    expect_args("round", &args, 1, span)?;
    match &args[0] {
        Value::Integer(n) => Ok(Value::Integer(*n)),
        Value::Float(n) => Ok(Value::Integer(n.round() as i64)),
        Value::Boolean(b) => Ok(Value::Integer(if *b { 1 } else { 0 })),
        _ => Err(RuntimeError {
            call_stack: Vec::new(),
            message: format!(
                "'round' expects a numeric value, got {}",
                args[0].type_name()
            ),
            span: span.clone(),
        }),
    }
}

// --- Internal helpers ---

fn to_f64(val: &Value, func: &str, span: &Span) -> Result<f64, RuntimeError> {
    match val {
        Value::Integer(n) => Ok(*n as f64),
        Value::Float(n) => Ok(*n),
        Value::Boolean(b) => Ok(if *b { 1.0 } else { 0.0 }),
        _ => Err(RuntimeError {
            call_stack: Vec::new(),
            message: format!("'{}' expects numeric values, got {}", func, val.type_name()),
            span: span.clone(),
        }),
    }
}

/// For min/max: if both args are integers, return integer; otherwise float.
fn coerce_back(val: f64, a: &Value, b: &Value) -> Value {
    let both_int = matches!(a, Value::Integer(_) | Value::Boolean(_))
        && matches!(b, Value::Integer(_) | Value::Boolean(_));
    if both_int {
        Value::Integer(val as i64)
    } else {
        Value::Float(val)
    }
}

/// Simple value equality for contains/contains_key.
fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Integer(a), Value::Integer(b)) => a == b,
        (Value::Float(a), Value::Float(b)) => a == b,
        (Value::Boolean(a), Value::Boolean(b)) => a == b,
        (Value::Nil, Value::Nil) => true,
        // Numeric coercion for contains
        (Value::Integer(a), Value::Float(b)) => (*a as f64) == *b,
        (Value::Float(a), Value::Integer(b)) => *a == (*b as f64),
        (Value::Boolean(a), Value::Integer(b)) => (if *a { 1 } else { 0 }) == *b,
        (Value::Integer(a), Value::Boolean(b)) => *a == (if *b { 1 } else { 0 }),
        _ => false,
    }
}
