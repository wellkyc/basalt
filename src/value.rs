use std::cell::RefCell;
use std::cmp::Ordering;
use std::fmt;
use std::rc::Rc;

use crate::ast::{BinOp, UnaryOp};
use crate::error::{Error, Span};

#[derive(Debug, Clone)]
pub enum Value {
    Num(f64),
    Str(Rc<str>),
    Bool(bool),
    Array(Rc<RefCell<Array>>),
}

impl Value {
    pub fn str(text: impl Into<Rc<str>>) -> Self {
        Value::Str(text.into())
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Num(_) => "number",
            Value::Str(_) => "string",
            Value::Bool(_) => "boolean",
            Value::Array(_) => "array",
        }
    }

    pub fn default_for(name: &str) -> Self {
        if name.ends_with('$') {
            Value::str("")
        } else {
            Value::Num(0.0)
        }
    }

    pub fn as_number(&self, span: Span) -> Result<f64, Error> {
        match self {
            Value::Num(n) => Ok(*n),
            other => Err(Error::runtime(
                format!(
                    "Type mismatch: expected a number, got a {}",
                    other.type_name()
                ),
                span,
            )),
        }
    }

    pub fn is_truthy(&self, span: Span) -> Result<bool, Error> {
        match self {
            Value::Bool(b) => Ok(*b),
            Value::Num(n) => Ok(*n != 0.0),
            other => Err(Error::runtime(
                format!(
                    "Type mismatch: a {} cannot be used as a condition",
                    other.type_name()
                ),
                span,
            )),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Num(n) => f.write_str(&format_number(*n)),
            Value::Str(s) => f.write_str(s),
            Value::Bool(b) => f.write_str(if *b { "TRUE" } else { "FALSE" }),
            Value::Array(_) => f.write_str("<array>"),
        }
    }
}

#[derive(Debug)]
pub struct Array {
    pub bounds: Vec<usize>,
    pub data: Vec<Value>,
}

impl Array {
    pub fn new(bounds: Vec<usize>, fill: Value) -> Self {
        let size = bounds.iter().map(|b| b + 1).product();
        Self {
            bounds,
            data: vec![fill; size],
        }
    }

    pub fn offset(&self, indices: &[f64]) -> Option<usize> {
        let mut offset = 0;
        for (&index, &bound) in indices.iter().zip(&self.bounds) {
            let index = index.round();
            if index < 0.0 || index > bound as f64 {
                return None;
            }
            offset = offset * (bound + 1) + index as usize;
        }
        Some(offset)
    }
}

pub fn format_number(n: f64) -> String {
    if n.is_nan() {
        return "NaN".into();
    }
    if n.is_infinite() {
        return if n > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    let abs = n.abs();
    if n.fract() == 0.0 && abs < 1e15 {
        return format!("{}", n as i64);
    }
    if (1e-6..1e15).contains(&abs) {
        let text = format!("{n:.10}");
        return text.trim_end_matches('0').trim_end_matches('.').to_string();
    }
    format!("{n:e}")
}

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        Self(z.max(1))
    }

    pub fn next_f64(&mut self) -> f64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn mismatch(op: &str, left: &Value, right: &Value, span: Span) -> Error {
    Error::runtime(
        format!(
            "Type mismatch: cannot apply {op} to a {} and a {}",
            left.type_name(),
            right.type_name()
        ),
        span,
    )
}

pub fn compare(left: &Value, right: &Value, span: Span) -> Result<Ordering, Error> {
    match (left, right) {
        (Value::Num(a), Value::Num(b)) => Ok(a.partial_cmp(b).unwrap_or(Ordering::Equal)),
        (Value::Str(a), Value::Str(b)) => Ok(a.cmp(b)),
        (Value::Bool(a), Value::Bool(b)) => Ok(a.cmp(b)),
        _ => Err(mismatch("a comparison", left, right, span)),
    }
}

pub fn compare_op(op: BinOp, ordering: Ordering) -> bool {
    match op {
        BinOp::Eq => ordering == Ordering::Equal,
        BinOp::Ne => ordering != Ordering::Equal,
        BinOp::Lt => ordering == Ordering::Less,
        BinOp::Gt => ordering == Ordering::Greater,
        BinOp::Le => ordering != Ordering::Greater,
        BinOp::Ge => ordering != Ordering::Less,
        _ => false,
    }
}

fn op_symbol(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::IntDiv => "\\",
        BinOp::Mod => "MOD",
        BinOp::Pow => "^",
        BinOp::And => "AND",
        BinOp::Or => "OR",
        BinOp::Xor => "XOR",
        _ => "a comparison",
    }
}

pub fn binary(op: BinOp, left: Value, right: Value, span: Span) -> Result<Value, Error> {
    use BinOp::*;
    match op {
        Eq | Ne | Lt | Gt | Le | Ge => {
            Ok(Value::Bool(compare_op(op, compare(&left, &right, span)?)))
        }
        Add => match (&left, &right) {
            (Value::Num(a), Value::Num(b)) => Ok(Value::Num(a + b)),
            (Value::Str(a), Value::Str(b)) => Ok(Value::str(format!("{a}{b}"))),
            _ => Err(mismatch("+", &left, &right, span)),
        },
        And | Or | Xor => match (&left, &right) {
            (Value::Bool(a), Value::Bool(b)) => Ok(Value::Bool(match op {
                And => *a && *b,
                Or => *a || *b,
                _ => a ^ b,
            })),
            (Value::Num(a), Value::Num(b)) => {
                let (a, b) = (*a as i64, *b as i64);
                Ok(Value::Num(match op {
                    And => a & b,
                    Or => a | b,
                    _ => a ^ b,
                } as f64))
            }
            _ => Err(mismatch(op_symbol(op), &left, &right, span)),
        },
        Sub | Mul | Div | IntDiv | Mod | Pow => {
            let (Value::Num(a), Value::Num(b)) = (&left, &right) else {
                return Err(mismatch(op_symbol(op), &left, &right, span));
            };
            let (a, b) = (*a, *b);
            let division_by_zero = || Error::runtime("Division by zero", span);
            Ok(Value::Num(match op {
                Sub => a - b,
                Mul => a * b,
                Div if b == 0.0 => return Err(division_by_zero()),
                Div => a / b,
                Pow => a.powf(b),
                _ => {
                    let (a, b) = (a.round(), b.round());
                    if b == 0.0 {
                        return Err(division_by_zero());
                    }
                    if op == IntDiv { (a / b).trunc() } else { a % b }
                }
            }))
        }
    }
}

pub fn unary(op: UnaryOp, value: Value, span: Span) -> Result<Value, Error> {
    match (op, &value) {
        (UnaryOp::Neg, Value::Num(n)) => Ok(Value::Num(-n)),
        (UnaryOp::Not, Value::Bool(b)) => Ok(Value::Bool(!b)),
        (UnaryOp::Not, Value::Num(n)) => Ok(Value::Num(!(*n as i64) as f64)),
        _ => Err(Error::runtime(
            format!(
                "Type mismatch: cannot apply {} to a {}",
                if op == UnaryOp::Neg { "-" } else { "NOT" },
                value.type_name()
            ),
            span,
        )),
    }
}
