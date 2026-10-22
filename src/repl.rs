use std::io::{self, BufRead, IsTerminal, StdinLock, Stdout, Write};
use std::process::ExitCode;

use basalt::ast::{BinOp, Expr, ExprKind};
use basalt::{Interpreter, Value, parse, parse_expression};

type Session = Interpreter<Stdout, StdinLock<'static>>;

const HELP: &str = "\
Type BASIC statements or expressions and press Enter.

  PRINT \"Hello\"         run a statement
  2 + 2 * 10            evaluate an expression
  FOR i = 1 TO 3        start a block, finish it with NEXT

Blocks (IF, FOR, WHILE, DO, SELECT, SUB, FUNCTION) continue on the
next lines until they are closed. Variables, SUBs and FUNCTIONs stay
defined for the whole session.

  :help      show this help
  :reset     forget all variables and procedures
  :quit      exit";

fn looks_like_statement(expr: &Expr, session: &Session) -> bool {
    match &expr.kind {
        ExprKind::Binary(BinOp::Eq, lhs, _) => {
            matches!(lhs.kind, ExprKind::Var(_) | ExprKind::Call(..))
        }
        ExprKind::Var(name) | ExprKind::Call(name, _) => session.has_sub(name),
        _ => false,
    }
}

fn show(value: &Value) -> String {
    match value {
        Value::Str(s) => format!("\"{s}\""),
        other => other.to_string(),
    }
}

fn execute(session: &mut Session, source: &str, color: bool) -> bool {
    let report = |error: basalt::Error| {
        if !error.is_halt() {
            eprint!("{}", error.render(source, "<repl>", color));
        }
    };
    if source.trim_end().lines().count() == 1
        && let Ok(expr) = parse_expression(source)
        && !looks_like_statement(&expr, session)
    {
        match session.eval_expression(&expr) {
            Ok(value) => println!("{}", show(&value)),
            Err(error) => report(error),
        }
        return true;
    }
    match parse(source) {
        Ok(program) => {
            if let Err(error) = session.run(&program) {
                let _ = session.end_line();
                report(error);
            } else {
                let _ = session.end_line();
            }
            true
        }
        Err(error) if error.incomplete => false,
        Err(error) => {
            report(error);
            true
        }
    }
}

fn read_line(session: &mut Session, prompt: &str) -> io::Result<Option<String>> {
    print!("{prompt}");
    io::stdout().flush()?;
    let mut line = String::new();
    if session.input_mut().read_line(&mut line)? == 0 {
        println!();
        return Ok(None);
    }
    Ok(Some(
        line.trim_start_matches('\u{feff}')
            .trim_end_matches(['\r', '\n'])
            .to_string(),
    ))
}

pub fn start() -> ExitCode {
    let color = io::stderr().is_terminal();
    let mut session = Interpreter::new(io::stdout(), io::stdin().lock());
    let mut buffer = String::new();
    println!(
        "basalt {}, :help for help, :quit to exit",
        env!("CARGO_PKG_VERSION")
    );

    loop {
        let prompt = if buffer.is_empty() { "> " } else { ". " };
        let line = match read_line(&mut session, prompt) {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(error) => {
                eprintln!("{error}");
                return ExitCode::FAILURE;
            }
        };
        if buffer.is_empty() {
            match line.trim() {
                "" => continue,
                ":quit" | ":q" | ":exit" => break,
                ":help" | ":h" => {
                    println!("{HELP}");
                    continue;
                }
                ":reset" => {
                    session.reset();
                    println!("Session cleared.");
                    continue;
                }
                _ => {}
            }
        }
        buffer.push_str(&line);
        buffer.push('\n');
        if execute(&mut session, &buffer, color) {
            buffer.clear();
        }
    }
    ExitCode::SUCCESS
}
