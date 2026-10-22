use std::io::Cursor;

use basalt::{Error, ErrorKind};

fn run_with_input(source: &str, input: &str) -> Result<String, Error> {
    let input = Cursor::new(input.as_bytes().to_vec());
    match basalt::run(source, Vec::new(), input) {
        Ok(output) => Ok(String::from_utf8(output).unwrap()),
        Err((_, error)) => Err(error),
    }
}

fn run(source: &str) -> String {
    run_with_input(source, "").unwrap_or_else(|error| panic!("{error}"))
}

fn fails(source: &str) -> Error {
    run_with_input(source, "").expect_err("program should fail")
}

#[test]
fn arithmetic_follows_basic_precedence() {
    assert_eq!(run("PRINT 2 + 3 * 4"), "14\n");
    assert_eq!(run("PRINT (2 + 3) * 4"), "20\n");
    assert_eq!(run("PRINT -2 ^ 2"), "-4\n");
    assert_eq!(run("PRINT 2 ^ 3 ^ 2"), "64\n");
    assert_eq!(run("PRINT 7 \\ 2, 7 MOD 3"), "3             1\n");
    assert_eq!(run("PRINT 10 / 4"), "2.5\n");
    assert_eq!(run("PRINT 1 + 2 * 3 MOD 4"), "3\n");
}

#[test]
fn numbers_are_formatted_cleanly() {
    assert_eq!(run("PRINT 0.1 + 0.2"), "0.3\n");
    assert_eq!(run("PRINT 1 / 3"), "0.3333333333\n");
    assert_eq!(run("PRINT 1E20"), "1e20\n");
    assert_eq!(run("PRINT -0"), "0\n");
    assert_eq!(run("PRINT .5"), "0.5\n");
}

#[test]
fn print_separators_control_spacing() {
    assert_eq!(run("PRINT \"a\"; \"b\""), "ab\n");
    assert_eq!(run("PRINT \"a\", \"b\""), "a             b\n");
    assert_eq!(run("PRINT \"a\";\nPRINT \"b\""), "ab\n");
    assert_eq!(run("PRINT"), "\n");
    assert_eq!(run("? \"short\""), "short\n");
}

#[test]
fn keywords_and_names_are_case_insensitive() {
    assert_eq!(run("total = 5\nprint ToTaL * 2"), "10\n");
}

#[test]
fn strings_and_string_functions() {
    assert_eq!(
        run("a$ = \"Hello\"\nPRINT a$ + \", World\""),
        "Hello, World\n"
    );
    assert_eq!(
        run("PRINT LEN(\"hello\"), UCASE$(\"abc\")"),
        "5             ABC\n"
    );
    assert_eq!(
        run("PRINT LEFT$(\"basalt\", 3); RIGHT$(\"basalt\", 3)"),
        "basalt\n"
    );
    assert_eq!(
        run("PRINT MID$(\"basalt\", 2, 3); MID$(\"basalt\", 4)"),
        "asaalt\n"
    );
    assert_eq!(
        run("PRINT INSTR(\"banana\", \"na\"); INSTR(4, \"banana\", \"na\")"),
        "35\n"
    );
    assert_eq!(run("PRINT VAL(\"12abc\") + 1; STR$(3.5)"), "133.5\n");
    assert_eq!(
        run("PRINT ASC(\"A\"); CHR$(66); STRING$(3, \"*\")"),
        "65B***\n"
    );
    assert_eq!(run("PRINT \"[\"; TRIM$(\"  x  \"); \"]\""), "[x]\n");
}

#[test]
fn math_functions() {
    assert_eq!(
        run("PRINT ABS(-3), SQR(16), INT(-2.5), FIX(-2.5)"),
        "3             4             -3            -2\n"
    );
    assert_eq!(
        run("PRINT ROUND(3.14159, 2); \" \"; MIN(4, 2, 8); MAX(4, 2, 8)"),
        "3.14 28\n"
    );
}

#[test]
fn block_if_with_elseif_and_else() {
    let program = "
FOR n = 1 TO 3
    IF n = 1 THEN
        PRINT \"one\"
    ELSEIF n = 2 THEN
        PRINT \"two\"
    ELSE
        PRINT \"many\"
    END IF
NEXT";
    assert_eq!(run(program), "one\ntwo\nmany\n");
}

#[test]
fn single_line_if() {
    assert_eq!(
        run("x = 5\nIF x > 3 THEN PRINT \"big\" ELSE PRINT \"small\""),
        "big\n"
    );
    assert_eq!(
        run("x = 1\nIF x > 3 THEN PRINT \"big\": PRINT \"!\" ELSE PRINT \"small\""),
        "small\n"
    );
}

#[test]
fn logical_operators_short_circuit_on_booleans() {
    assert_eq!(
        run("PRINT 1 < 2 AND 2 < 3, NOT 1 > 2, TRUE XOR TRUE"),
        "TRUE          TRUE          FALSE\n"
    );
    assert_eq!(run("PRINT 1 > 2 AND 1 / 0 = 1"), "FALSE\n");
    assert_eq!(run("PRINT 12 AND 10, 12 OR 3"), "8             15\n");
}

#[test]
fn for_loops_with_steps() {
    assert_eq!(run("FOR i = 1 TO 5 STEP 2: PRINT i;: NEXT"), "135");
    assert_eq!(run("FOR i = 3 TO 1 STEP -1: PRINT i;: NEXT i"), "321");
    assert_eq!(
        run("FOR i = 5 TO 1: PRINT i: NEXT\nPRINT \"done\""),
        "done\n"
    );
}

#[test]
fn while_and_do_loops() {
    assert_eq!(
        run("i = 0\nWHILE i < 3\n    i = i + 1\n    PRINT i;\nWEND"),
        "123"
    );
    assert_eq!(
        run("i = 0\nDO\n    i = i + 1\nLOOP UNTIL i >= 4\nPRINT i"),
        "4\n"
    );
    assert_eq!(
        run("i = 10\nDO WHILE i < 3\n    i = i + 1\nLOOP\nPRINT i"),
        "10\n"
    );
}

#[test]
fn exit_leaves_the_innermost_loop() {
    let program = "
FOR i = 1 TO 3
    FOR j = 1 TO 10
        IF j > i THEN EXIT FOR
        PRINT j;
    NEXT
    PRINT \"|\";
NEXT";
    assert_eq!(run(program), "1|12|123|");
    assert_eq!(
        run("DO\n    n = n + 1\n    IF n = 4 THEN EXIT DO\nLOOP\nPRINT n"),
        "4\n"
    );
}

#[test]
fn select_case_supports_values_ranges_and_is() {
    let program = "
FOR n = 0 TO 12 STEP 4
    SELECT CASE n
        CASE 0
            PRINT \"zero\"
        CASE 1, 2, 3 TO 5
            PRINT \"small\"
        CASE IS > 10
            PRINT \"huge\"
        CASE ELSE
            PRINT \"medium\"
    END SELECT
NEXT";
    assert_eq!(run(program), "zero\nsmall\nmedium\nhuge\n");
}

#[test]
fn functions_support_recursion() {
    let program = "
FUNCTION Factorial (n)
    IF n <= 1 THEN Factorial = 1 ELSE Factorial = n * Factorial(n - 1)
END FUNCTION
PRINT Factorial(10)";
    assert_eq!(run(program), "3628800\n");
}

#[test]
fn subs_accept_arguments_and_can_be_called_three_ways() {
    let program = "
SUB Greet (name$, times)
    FOR i = 1 TO times
        PRINT \"Hi \"; name$
    NEXT
END SUB
Greet \"Ann\", 1
Greet(\"Bob\", 1)
CALL Greet(\"Cid\", 2)";
    assert_eq!(run(program), "Hi Ann\nHi Bob\nHi Cid\nHi Cid\n");
}

#[test]
fn procedures_can_be_called_before_they_are_defined() {
    assert_eq!(
        run("PRINT Twice(21)\nFUNCTION Twice (x)\n    Twice = x * 2\nEND FUNCTION"),
        "42\n"
    );
}

#[test]
fn procedure_variables_are_local_unless_shared() {
    let program = "
DIM SHARED counter
total = 100
SUB Bump
    counter = counter + 1
    total = 1
END SUB
Bump
Bump
PRINT counter; \" \"; total";
    assert_eq!(run(program), "2 100\n");
}

#[test]
fn exit_sub_returns_early() {
    let program = "
SUB Check (n)
    IF n < 0 THEN EXIT SUB
    PRINT \"ok\"; n
END SUB
Check -1
Check 2";
    assert_eq!(run(program), "ok2\n");
}

#[test]
fn arrays_with_one_and_two_dimensions() {
    assert_eq!(
        run("DIM a(5)\nFOR i = 0 TO 5: a(i) = i * i: NEXT\nPRINT a(4); UBOUND(a)"),
        "165\n"
    );
    let grid = "
DIM grid(2, 3)
grid(1, 2) = 7
grid(2, 3) = 9
PRINT grid(1, 2) + grid(2, 3); grid(0, 0)";
    assert_eq!(run(grid), "160\n");
    assert_eq!(
        run("DIM names$(2)\nnames$(1) = \"x\"\nPRINT \"[\"; names$(0); names$(1); \"]\""),
        "[x]\n"
    );
}

#[test]
fn arrays_are_created_automatically_with_ten_elements() {
    assert_eq!(
        run("scores(3) = 42\nPRINT scores(3); UBOUND(scores)"),
        "4210\n"
    );
}

#[test]
fn goto_and_line_numbers() {
    let program = "
10 i = i + 1
20 IF i < 3 THEN GOTO 10
30 PRINT i";
    assert_eq!(run(program), "3\n");
    assert_eq!(
        run("GOTO skip\nPRINT \"never\"\nskip:\nPRINT \"after\""),
        "after\n"
    );
}

#[test]
fn gosub_and_return() {
    let program = "
GOSUB greet
GOSUB greet
END
greet:
    PRINT \"hi\"
    RETURN";
    assert_eq!(run(program), "hi\nhi\n");
}

#[test]
fn data_read_and_restore() {
    let program = "
DATA 1, 2, \"three\", -4
READ a, b, c$, d
PRINT a + b; c$; d
RESTORE
READ x
PRINT x";
    assert_eq!(run(program), "3three-4\n1\n");
}

#[test]
fn constants_and_swap() {
    assert_eq!(run("CONST PI = 3.14\nPRINT PI * 2"), "6.28\n");
    assert_eq!(run("a = 1: b = 2\nSWAP a, b\nPRINT a; b"), "21\n");
}

#[test]
fn end_stops_the_program() {
    assert_eq!(run("PRINT \"a\"\nEND\nPRINT \"b\""), "a\n");
}

#[test]
fn input_reads_numbers_and_strings() {
    let program = "INPUT \"Name\"; name$\nINPUT \"Age\", age\nPRINT name$; \" is \"; age + 1";
    let output = run_with_input(program, "Ada\n36\n").unwrap();
    assert_eq!(output, "Name? AgeAda is 37\n");
}

#[test]
fn input_asks_again_on_invalid_numbers() {
    let output = run_with_input("INPUT n\nPRINT n * 2", "abc\n21\n").unwrap();
    assert_eq!(output, "? ?Redo from start\n? 42\n");
}

#[test]
fn input_reads_several_values_from_one_line() {
    let output = run_with_input("INPUT a, b\nPRINT a + b", "2, 3\n").unwrap();
    assert_eq!(output, "? 5\n");
}

#[test]
fn rnd_is_reproducible_with_a_seed() {
    let program = "RANDOMIZE 42\nFOR i = 1 TO 5: PRINT INT(RND * 100);\" \";: NEXT";
    let first = run(program);
    assert_eq!(first, run(program));
    assert!(run("PRINT RND < 1 AND RND >= 0").starts_with("TRUE"));
}

#[test]
fn colors_emit_ansi_codes_and_reset_at_the_end() {
    assert_eq!(run("COLOR 12\nPRINT \"x\""), "\x1b[91mx\n\x1b[0m");
}

#[test]
fn runtime_errors_point_at_the_source() {
    let error = fails("x = 1\nPRINT x / 0");
    assert_eq!(error.kind, ErrorKind::Runtime);
    assert_eq!(error.message, "Division by zero");
    let span = error.span.unwrap();
    assert_eq!((span.line, span.col, span.len), (2, 7, 5));
}

#[test]
fn rendered_errors_show_a_caret() {
    let source = "PRINT 1 +";
    let error = basalt::parse(source).unwrap_err();
    let rendered = error.render(source, "test.bas", false);
    assert!(rendered.starts_with("syntax error: Expected an expression"));
    assert!(rendered.contains("--> test.bas:1:10"));
    assert!(rendered.contains("1 | PRINT 1 +"));
}

#[test]
fn type_errors_are_reported() {
    assert!(fails("PRINT \"a\" + 1").message.contains("Type mismatch"));
    assert!(fails("a$ = 5").message.contains("can only hold strings"));
    assert!(
        fails("IF \"yes\" THEN PRINT 1")
            .message
            .contains("condition")
    );
}

#[test]
fn common_mistakes_produce_clear_errors() {
    assert_eq!(
        fails("DIM a(3)\na(4) = 1").message,
        "Subscript out of range"
    );
    assert_eq!(
        fails("CONST X = 1\nX = 2").message,
        "Cannot assign to constant X"
    );
    assert_eq!(fails("Missing 1").message, "Undefined SUB MISSING");
    assert_eq!(
        fails("PRINT Nope(1)").message,
        "Undefined function or array NOPE"
    );
    assert_eq!(
        fails("GOTO 99").message,
        "Label 99 not found in the program"
    );
    assert_eq!(fails("RETURN").message, "RETURN without GOSUB");
    assert_eq!(fails("READ x").message, "Out of DATA");
    assert_eq!(fails("PRINT SQR(-1)").message, "Illegal function call: SQR");
    assert_eq!(
        fails("PRINT LEFT$(\"a\")").message,
        "LEFT$ expects 2 argument(s), got 1"
    );
}

#[test]
fn deep_recursion_is_caught() {
    let error = std::thread::Builder::new()
        .stack_size(512 * 1024 * 1024)
        .spawn(|| fails("FUNCTION Down (n)\n    Down = Down(n + 1)\nEND FUNCTION\nPRINT Down(0)"))
        .unwrap()
        .join()
        .unwrap();
    assert!(error.message.contains("Stack overflow"));
}

#[test]
fn unfinished_blocks_are_marked_incomplete() {
    for source in [
        "IF x THEN\nPRINT 1",
        "FOR i = 1 TO 3",
        "SUB Foo",
        "PRINT (1 +",
        "DO",
    ] {
        let error = basalt::parse(source).unwrap_err();
        assert!(error.incomplete, "{source:?} should be incomplete");
    }
    let error = basalt::parse("PRINT 1 +* 2").unwrap_err();
    assert!(!error.incomplete);
}

#[test]
fn syntax_errors_for_mismatched_blocks() {
    assert!(
        basalt::parse("END IF")
            .unwrap_err()
            .message
            .contains("without a matching")
    );
    assert!(
        basalt::parse("FOR i = 1 TO 2\nNEXT j")
            .unwrap_err()
            .message
            .contains("does not match")
    );
    assert!(basalt::parse("IF 1 THEN\n    SUB Inner\n    END SUB\nEND IF").is_err());
}

#[test]
fn comments_are_ignored() {
    assert_eq!(run("REM setup\nx = 1 ' trailing\nPRINT x"), "1\n");
}
