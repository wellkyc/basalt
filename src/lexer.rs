use crate::error::{Error, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    And,
    As,
    Beep,
    Call,
    Case,
    Cls,
    Color,
    Const,
    Data,
    Dim,
    Do,
    Else,
    ElseIf,
    End,
    Exit,
    False,
    For,
    Function,
    Gosub,
    Goto,
    If,
    Input,
    Is,
    Let,
    Locate,
    Loop,
    Mod,
    Next,
    Not,
    Or,
    Print,
    Randomize,
    Read,
    Restore,
    Return,
    Select,
    Shared,
    Sleep,
    Step,
    Sub,
    Swap,
    Then,
    To,
    True,
    Until,
    Wend,
    While,
    Xor,
}

impl Keyword {
    fn from_word(word: &str) -> Option<Self> {
        use Keyword::*;
        Some(match word {
            "AND" => And,
            "AS" => As,
            "BEEP" => Beep,
            "CALL" => Call,
            "CASE" => Case,
            "CLS" => Cls,
            "COLOR" => Color,
            "CONST" => Const,
            "DATA" => Data,
            "DIM" => Dim,
            "DO" => Do,
            "ELSE" => Else,
            "ELSEIF" => ElseIf,
            "END" | "STOP" => End,
            "EXIT" => Exit,
            "FALSE" => False,
            "FOR" => For,
            "FUNCTION" => Function,
            "GOSUB" => Gosub,
            "GOTO" => Goto,
            "IF" => If,
            "INPUT" => Input,
            "IS" => Is,
            "LET" => Let,
            "LOCATE" => Locate,
            "LOOP" => Loop,
            "MOD" => Mod,
            "NEXT" => Next,
            "NOT" => Not,
            "OR" => Or,
            "PRINT" => Print,
            "RANDOMIZE" => Randomize,
            "READ" => Read,
            "RESTORE" => Restore,
            "RETURN" => Return,
            "SELECT" => Select,
            "SHARED" => Shared,
            "SLEEP" => Sleep,
            "STEP" => Step,
            "SUB" => Sub,
            "SWAP" => Swap,
            "THEN" => Then,
            "TO" => To,
            "TRUE" => True,
            "UNTIL" => Until,
            "WEND" => Wend,
            "WHILE" => While,
            "XOR" => Xor,
            _ => return None,
        })
    }

    pub fn name(self) -> String {
        format!("{self:?}").to_uppercase()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Number(f64),
    Str(String),
    Ident(String),
    Keyword(Keyword),
    Plus,
    Minus,
    Star,
    Slash,
    Backslash,
    Caret,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    LParen,
    RParen,
    Comma,
    Semicolon,
    Colon,
    Newline,
    Eof,
}

impl Tok {
    pub fn describe(&self) -> String {
        match self {
            Tok::Number(n) => format!("number {n}"),
            Tok::Str(s) => format!("string \"{s}\""),
            Tok::Ident(name) => format!("'{name}'"),
            Tok::Keyword(k) => k.name(),
            Tok::Plus => "'+'".into(),
            Tok::Minus => "'-'".into(),
            Tok::Star => "'*'".into(),
            Tok::Slash => "'/'".into(),
            Tok::Backslash => "'\\'".into(),
            Tok::Caret => "'^'".into(),
            Tok::Eq => "'='".into(),
            Tok::Ne => "'<>'".into(),
            Tok::Lt => "'<'".into(),
            Tok::Gt => "'>'".into(),
            Tok::Le => "'<='".into(),
            Tok::Ge => "'>='".into(),
            Tok::LParen => "'('".into(),
            Tok::RParen => "')'".into(),
            Tok::Comma => "','".into(),
            Tok::Semicolon => "';'".into(),
            Tok::Colon => "':'".into(),
            Tok::Newline => "end of line".into(),
            Tok::Eof => "end of input".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Token {
    pub tok: Tok,
    pub span: Span,
}

pub fn tokenize(source: &str) -> Result<Vec<Token>, Error> {
    let chars: Vec<char> = source.chars().collect();
    let len = chars.len();
    let mut tokens = Vec::new();
    let (mut i, mut line, mut col) = (0, 1, 1);
    let digit_at = |j: usize| j < len && chars[j].is_ascii_digit();

    while i < len {
        let c = chars[i];
        let start = i;
        let tok = match c {
            ' ' | '\t' | '\r' | '\u{feff}' => {
                i += 1;
                col += 1;
                continue;
            }
            '\n' => {
                tokens.push(Token {
                    tok: Tok::Newline,
                    span: Span::new(line, col, 1),
                });
                i += 1;
                line += 1;
                col = 1;
                continue;
            }
            '\'' => {
                while i < len && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            }
            '"' => {
                i += 1;
                while i < len && chars[i] != '"' && chars[i] != '\n' {
                    i += 1;
                }
                if i >= len || chars[i] == '\n' {
                    return Err(Error::syntax(
                        "Unterminated string literal",
                        Span::new(line, col, i - start),
                    ));
                }
                i += 1;
                Tok::Str(chars[start + 1..i - 1].iter().collect())
            }
            c if c.is_ascii_digit() || (c == '.' && digit_at(i + 1)) => {
                while digit_at(i) {
                    i += 1;
                }
                if i < len && chars[i] == '.' {
                    i += 1;
                    while digit_at(i) {
                        i += 1;
                    }
                }
                if i < len && (chars[i] == 'e' || chars[i] == 'E') {
                    let mut j = i + 1;
                    if j < len && (chars[j] == '+' || chars[j] == '-') {
                        j += 1;
                    }
                    if digit_at(j) {
                        i = j;
                        while digit_at(i) {
                            i += 1;
                        }
                    }
                }
                let text: String = chars[start..i].iter().collect();
                let span = Span::new(line, col, i - start);
                Tok::Number(
                    text.parse()
                        .map_err(|_| Error::syntax("Invalid number", span))?,
                )
            }
            c if c.is_alphabetic() || c == '_' => {
                while i < len && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                if i < len && chars[i] == '$' {
                    i += 1;
                }
                let word = chars[start..i].iter().collect::<String>().to_uppercase();
                if word == "REM" {
                    while i < len && chars[i] != '\n' {
                        i += 1;
                    }
                    continue;
                }
                match Keyword::from_word(&word) {
                    Some(keyword) => Tok::Keyword(keyword),
                    None => Tok::Ident(word),
                }
            }
            '<' => {
                i += 1;
                match chars.get(i) {
                    Some('=') => {
                        i += 1;
                        Tok::Le
                    }
                    Some('>') => {
                        i += 1;
                        Tok::Ne
                    }
                    _ => Tok::Lt,
                }
            }
            '>' => {
                i += 1;
                if chars.get(i) == Some(&'=') {
                    i += 1;
                    Tok::Ge
                } else {
                    Tok::Gt
                }
            }
            _ => {
                let tok = match c {
                    '?' => Tok::Keyword(Keyword::Print),
                    '+' => Tok::Plus,
                    '-' => Tok::Minus,
                    '*' => Tok::Star,
                    '/' => Tok::Slash,
                    '\\' => Tok::Backslash,
                    '^' => Tok::Caret,
                    '=' => Tok::Eq,
                    '(' => Tok::LParen,
                    ')' => Tok::RParen,
                    ',' => Tok::Comma,
                    ';' => Tok::Semicolon,
                    ':' => Tok::Colon,
                    _ => {
                        return Err(Error::syntax(
                            format!("Unexpected character '{c}'"),
                            Span::new(line, col, 1),
                        ));
                    }
                };
                i += 1;
                tok
            }
        };
        let width = i - start;
        tokens.push(Token {
            tok,
            span: Span::new(line, col, width),
        });
        col += width;
    }

    tokens.push(Token {
        tok: Tok::Eof,
        span: Span::new(line, col, 0),
    });
    Ok(tokens)
}
