use std::io::Cursor;

fn run_example(source: &str, input: &str) -> String {
    let input = Cursor::new(input.as_bytes().to_vec());
    match basalt::run(source, Vec::new(), input) {
        Ok(output) => String::from_utf8(output).unwrap(),
        Err((_, error)) => panic!("{error}"),
    }
}

#[test]
fn hello() {
    assert_eq!(
        run_example(include_str!("../examples/hello.bas"), ""),
        "Hello, World!\n"
    );
}

#[test]
fn fizzbuzz() {
    let output = run_example(include_str!("../examples/fizzbuzz.bas"), "");
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 20);
    assert_eq!(&lines[..5], ["1", "2", "Fizz", "4", "Buzz"]);
    assert_eq!(lines[14], "FizzBuzz");
}

#[test]
fn fibonacci() {
    let output = run_example(include_str!("../examples/fibonacci.bas"), "");
    assert!(output.starts_with("Fib(0) = 0\nFib(1) = 1\nFib(2) = 1\n"));
    assert!(output.ends_with("Fib(15) = 610\n"));
}

#[test]
fn primes() {
    let output = run_example(include_str!("../examples/primes.bas"), "");
    assert!(output.starts_with("    2    3    5    7   11"));
    assert!(output.ends_with("46 primes up to 200\n"));
}

#[test]
fn mandelbrot() {
    let output = run_example(include_str!("../examples/mandelbrot.bas"), "");
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 25);
    assert!(lines.iter().all(|line| line.chars().count() == 78));
    assert!(lines[12].contains("@@@@@@@@"));
}

#[test]
fn classic() {
    let output = run_example(include_str!("../examples/classic.bas"), "");
    assert_eq!(
        output,
        "Counting down the classic way\n5...4...3...2...1...liftoff!\nGOSUB and RETURN work too\n"
    );
}

#[test]
fn guess_finishes_when_the_number_is_found() {
    let guesses: String = (1..=100).map(|n| format!("{n}\n")).collect();
    let output = run_example(include_str!("../examples/guess.bas"), &guesses);
    assert!(output.contains("Correct! You got it in "));
}
