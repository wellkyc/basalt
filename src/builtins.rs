use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{Error, Span};
use crate::value::{Rng, Value, format_number};

type Result<T> = std::result::Result<T, Error>;

const MAX_STRING: f64 = 100_000_000.0;

pub fn takes_no_args(name: &str) -> bool {
    matches!(name, "RND" | "TIMER")
}

struct Args<'a> {
    name: &'a str,
    values: Vec<Value>,
    span: Span,
}

impl Args<'_> {
    fn arity(&self, min: usize, max: usize) -> Result<()> {
        let n = self.values.len();
        if (min..=max).contains(&n) {
            return Ok(());
        }
        let expected = match (min, max) {
            (min, max) if min == max => min.to_string(),
            (min, usize::MAX) => format!("at least {min}"),
            (min, max) => format!("{min} to {max}"),
        };
        Err(Error::runtime(
            format!("{} expects {expected} argument(s), got {n}", self.name),
            self.span,
        ))
    }

    fn wrong_type(&self, expected: &str, got: &Value) -> Error {
        Error::runtime(
            format!(
                "{} expects a {expected}, got a {}",
                self.name,
                got.type_name()
            ),
            self.span,
        )
    }

    fn num(&self, i: usize) -> Result<f64> {
        match &self.values[i] {
            Value::Num(n) => Ok(*n),
            other => Err(self.wrong_type("number", other)),
        }
    }

    fn str(&self, i: usize) -> Result<&str> {
        match &self.values[i] {
            Value::Str(s) => Ok(s),
            other => Err(self.wrong_type("string", other)),
        }
    }

    fn count(&self, i: usize) -> Result<usize> {
        let n = self.num(i)?;
        if !(0.0..=MAX_STRING).contains(&n) {
            return Err(self.illegal());
        }
        Ok(n as usize)
    }

    fn illegal(&self) -> Error {
        Error::runtime(format!("Illegal function call: {}", self.name), self.span)
    }
}

pub fn call(name: &str, values: Vec<Value>, rng: &mut Rng, span: Span) -> Option<Result<Value>> {
    let a = Args { name, values, span };
    let result = match name {
        "ABS" => math(&a, f64::abs),
        "SGN" => math(&a, |n| if n == 0.0 { 0.0 } else { n.signum() }),
        "INT" => math(&a, f64::floor),
        "FIX" => math(&a, f64::trunc),
        "SIN" => math(&a, f64::sin),
        "COS" => math(&a, f64::cos),
        "TAN" => math(&a, f64::tan),
        "ATN" => math(&a, f64::atan),
        "EXP" => math(&a, f64::exp),
        "SQR" => checked_math(&a, |n| n >= 0.0, f64::sqrt),
        "LOG" => checked_math(&a, |n| n > 0.0, f64::ln),
        "ROUND" => round(&a),
        "RND" => a.arity(0, 1).map(|_| Value::Num(rng.next_f64())),
        "TIMER" => a.arity(0, 0).map(|_| Value::Num(timer())),
        "LEN" => text(&a).map(|s| Value::Num(s.chars().count() as f64)),
        "LEFT$" => substring(&a, |s, n| s.chars().take(n).collect()),
        "RIGHT$" => substring(&a, |s, n| {
            let len = s.chars().count();
            s.chars().skip(len.saturating_sub(n)).collect()
        }),
        "MID$" => mid(&a),
        "UCASE$" => transform(&a, str::to_uppercase),
        "LCASE$" => transform(&a, str::to_lowercase),
        "LTRIM$" => transform(&a, |s| s.trim_start().to_string()),
        "RTRIM$" => transform(&a, |s| s.trim_end().to_string()),
        "TRIM$" => transform(&a, |s| s.trim().to_string()),
        "INSTR" => instr(&a),
        "STR$" => a
            .arity(1, 1)
            .and_then(|_| a.num(0))
            .map(|n| Value::str(format_number(n))),
        "VAL" => text(&a).map(|s| Value::Num(parse_prefix(s))),
        "CHR$" => chr(&a),
        "ASC" => asc(&a),
        "SPACE$" => a
            .arity(1, 1)
            .and_then(|_| a.count(0))
            .map(|n| Value::str(" ".repeat(n))),
        "STRING$" => repeat_char(&a),
        "MIN" | "MAX" => extreme(&a, name == "MAX"),
        "UBOUND" | "LBOUND" => bound(&a, name == "UBOUND"),
        _ => return None,
    };
    Some(result)
}

fn math(a: &Args, f: fn(f64) -> f64) -> Result<Value> {
    a.arity(1, 1)?;
    Ok(Value::Num(f(a.num(0)?)))
}

fn checked_math(a: &Args, valid: fn(f64) -> bool, f: fn(f64) -> f64) -> Result<Value> {
    a.arity(1, 1)?;
    let n = a.num(0)?;
    if !valid(n) {
        return Err(a.illegal());
    }
    Ok(Value::Num(f(n)))
}

fn round(a: &Args) -> Result<Value> {
    a.arity(1, 2)?;
    let n = a.num(0)?;
    let digits = if a.values.len() == 2 { a.num(1)? } else { 0.0 };
    let factor = 10f64.powi(digits as i32);
    Ok(Value::Num((n * factor).round() / factor))
}

fn timer() -> f64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    (now.as_secs() % 86_400) as f64 + f64::from(now.subsec_millis()) / 1000.0
}

fn text<'a>(a: &'a Args) -> Result<&'a str> {
    a.arity(1, 1)?;
    a.str(0)
}

fn transform(a: &Args, f: fn(&str) -> String) -> Result<Value> {
    Ok(Value::str(f(text(a)?)))
}

fn substring(a: &Args, f: fn(&str, usize) -> String) -> Result<Value> {
    a.arity(2, 2)?;
    Ok(Value::str(f(a.str(0)?, a.count(1)?)))
}

fn mid(a: &Args) -> Result<Value> {
    a.arity(2, 3)?;
    let s = a.str(0)?;
    let start = a.num(1)?;
    if start < 1.0 {
        return Err(a.illegal());
    }
    let rest = s.chars().skip(start as usize - 1);
    let result: String = if a.values.len() == 3 {
        rest.take(a.count(2)?).collect()
    } else {
        rest.collect()
    };
    Ok(Value::str(result))
}

fn instr(a: &Args) -> Result<Value> {
    a.arity(2, 3)?;
    let (start, haystack, needle) = if a.values.len() == 3 {
        (a.num(0)?, a.str(1)?, a.str(2)?)
    } else {
        (1.0, a.str(0)?, a.str(1)?)
    };
    if start < 1.0 {
        return Err(a.illegal());
    }
    let haystack: Vec<char> = haystack.chars().collect();
    let needle: Vec<char> = needle.chars().collect();
    let from = start as usize - 1;
    let found = if from > haystack.len() {
        None
    } else if needle.is_empty() {
        Some(from)
    } else {
        (from..haystack.len()).find(|&i| haystack[i..].starts_with(&needle))
    };
    Ok(Value::Num(found.map_or(0.0, |i| (i + 1) as f64)))
}

fn chr(a: &Args) -> Result<Value> {
    a.arity(1, 1)?;
    let code = a.num(0)?;
    if code < 0.0 {
        return Err(a.illegal());
    }
    char::from_u32(code as u32)
        .map(|c| Value::str(c.to_string()))
        .ok_or_else(|| a.illegal())
}

fn asc(a: &Args) -> Result<Value> {
    text(a)?
        .chars()
        .next()
        .map(|c| Value::Num(f64::from(u32::from(c))))
        .ok_or_else(|| a.illegal())
}

fn repeat_char(a: &Args) -> Result<Value> {
    a.arity(2, 2)?;
    let n = a.count(0)?;
    let c = match &a.values[1] {
        Value::Num(code) if *code >= 0.0 => char::from_u32(*code as u32),
        Value::Str(s) => s.chars().next(),
        _ => None,
    }
    .ok_or_else(|| a.illegal())?;
    Ok(Value::str(c.to_string().repeat(n)))
}

fn extreme(a: &Args, max: bool) -> Result<Value> {
    a.arity(1, usize::MAX)?;
    let mut best = a.num(0)?;
    for i in 1..a.values.len() {
        let n = a.num(i)?;
        if (max && n > best) || (!max && n < best) {
            best = n;
        }
    }
    Ok(Value::Num(best))
}

fn bound(a: &Args, upper: bool) -> Result<Value> {
    a.arity(1, 2)?;
    let Value::Array(array) = &a.values[0] else {
        return Err(a.wrong_type("array", &a.values[0]));
    };
    let dimension = if a.values.len() == 2 { a.num(1)? } else { 1.0 };
    let array = array.borrow();
    let bound = (dimension >= 1.0)
        .then(|| array.bounds.get(dimension as usize - 1))
        .flatten()
        .ok_or_else(|| a.illegal())?;
    Ok(Value::Num(if upper { *bound as f64 } else { 0.0 }))
}

fn parse_prefix(text: &str) -> f64 {
    let text = text.trim_start();
    let mut best = 0.0;
    for (i, c) in text.char_indices() {
        if !matches!(c, '0'..='9' | '+' | '-' | '.' | 'e' | 'E') {
            break;
        }
        if let Ok(value) = text[..i + c.len_utf8()].parse::<f64>() {
            best = value;
        }
    }
    best
}
