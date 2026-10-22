mod repl;

use std::io::{self, BufWriter, IsTerminal};
use std::process::ExitCode;
use std::{env, fs, thread};

use basalt::{Interpreter, parse};

const STACK_SIZE: usize = 512 * 1024 * 1024;

const USAGE: &str = "\
basalt, a BASIC interpreter

Usage:
  basalt                 start the interactive REPL
  basalt <file.bas>      run a program
  basalt -e <code>       run code from the command line

Options:
  -h, --help             show this help
  -V, --version          show the version";

#[cfg(windows)]
fn enable_ansi() {
    use std::ffi::c_void;
    use std::os::windows::io::AsRawHandle;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetConsoleMode(handle: *mut c_void, mode: *mut u32) -> i32;
        fn SetConsoleMode(handle: *mut c_void, mode: u32) -> i32;
    }

    const ENABLE_VIRTUAL_TERMINAL_PROCESSING: u32 = 0x0004;
    for handle in [io::stdout().as_raw_handle(), io::stderr().as_raw_handle()] {
        let mut mode = 0;
        unsafe {
            if GetConsoleMode(handle, &mut mode) != 0 {
                SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING);
            }
        }
    }
}

#[cfg(not(windows))]
fn enable_ansi() {}

fn run_source(source: &str, path: &str) -> ExitCode {
    let color = io::stderr().is_terminal();
    let program = match parse(source) {
        Ok(program) => program,
        Err(error) => {
            eprint!("{}", error.render(source, path, color));
            return ExitCode::FAILURE;
        }
    };
    let output = BufWriter::new(io::stdout().lock());
    let mut interpreter = Interpreter::new(output, io::stdin().lock());
    match interpreter.run(&program) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let _ = interpreter.end_line();
            eprint!("{}", error.render(source, path, color));
            ExitCode::FAILURE
        }
    }
}

fn cli(args: Vec<String>) -> ExitCode {
    match args.as_slice() {
        [] => repl::start(),
        [flag] if flag == "-h" || flag == "--help" => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        [flag] if flag == "-V" || flag == "--version" => {
            println!("basalt {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        [flag, code] if flag == "-e" => run_source(code, "<eval>"),
        [path] if !path.starts_with('-') => match fs::read_to_string(path) {
            Ok(source) => run_source(&source, path),
            Err(error) => {
                eprintln!("basalt: cannot read {path}: {error}");
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

fn main() -> ExitCode {
    enable_ansi();
    let args: Vec<String> = env::args().skip(1).collect();
    thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || cli(args))
        .map(|handle| handle.join().unwrap_or(ExitCode::FAILURE))
        .unwrap_or(ExitCode::FAILURE)
}
