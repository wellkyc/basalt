use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub line: usize,
    pub col: usize,
    pub len: usize,
}

impl Span {
    pub fn new(line: usize, col: usize, len: usize) -> Self {
        Self { line, col, len }
    }

    pub fn to(self, end: Span) -> Span {
        if end.line == self.line && end.col + end.len >= self.col {
            Span {
                len: end.col + end.len - self.col,
                ..self
            }
        } else {
            self
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Syntax,
    Runtime,
    Halt,
}

#[derive(Debug, Clone)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
    pub span: Option<Span>,
    pub incomplete: bool,
}

impl Error {
    pub fn syntax(message: impl Into<String>, span: Span) -> Self {
        Self {
            kind: ErrorKind::Syntax,
            message: message.into(),
            span: Some(span),
            incomplete: false,
        }
    }

    pub fn incomplete(message: impl Into<String>, span: Span) -> Self {
        Self {
            incomplete: true,
            ..Self::syntax(message, span)
        }
    }

    pub fn runtime(message: impl Into<String>, span: Span) -> Self {
        Self {
            kind: ErrorKind::Runtime,
            message: message.into(),
            span: Some(span),
            incomplete: false,
        }
    }

    pub fn io(error: std::io::Error) -> Self {
        Self {
            kind: ErrorKind::Runtime,
            message: format!("I/O error: {error}"),
            span: None,
            incomplete: false,
        }
    }

    pub(crate) fn halt() -> Self {
        Self {
            kind: ErrorKind::Halt,
            message: String::new(),
            span: None,
            incomplete: false,
        }
    }

    pub fn is_halt(&self) -> bool {
        self.kind == ErrorKind::Halt
    }

    fn label(&self) -> &'static str {
        match self.kind {
            ErrorKind::Syntax => "syntax error",
            _ => "runtime error",
        }
    }

    pub fn render(&self, source: &str, path: &str, color: bool) -> String {
        let (red, blue, bold, reset) = if color {
            ("\x1b[1;31m", "\x1b[1;34m", "\x1b[1m", "\x1b[0m")
        } else {
            ("", "", "", "")
        };
        let mut out = format!(
            "{red}{}{reset}{bold}: {}{reset}\n",
            self.label(),
            self.message
        );
        let Some(span) = self.span else {
            return out;
        };
        let number = span.line.to_string();
        let pad = " ".repeat(number.len());
        out += &format!("{blue}{pad}--> {reset}{path}:{}:{}\n", span.line, span.col);
        if let Some(text) = source.lines().nth(span.line.saturating_sub(1)) {
            let text = text.trim_end().replace('\t', " ");
            let caret = "^".repeat(span.len.max(1));
            let indent = " ".repeat(span.col.saturating_sub(1));
            out += &format!("{blue}{pad} |{reset}\n");
            out += &format!("{blue}{number} |{reset} {text}\n");
            out += &format!("{blue}{pad} |{reset} {indent}{red}{caret}{reset}\n");
        }
        out
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.span {
            Some(span) => write!(
                f,
                "{} at {}:{}: {}",
                self.label(),
                span.line,
                span.col,
                self.message
            ),
            None => write!(f, "{}: {}", self.label(), self.message),
        }
    }
}

impl std::error::Error for Error {}
