# basalt

A BASIC interpreter in Rust. Syntax is mostly QBasic, with a REPL and errors that point at the line and column. No crates, only std.

```
$ basalt examples/mandelbrot.bas
                                           .....:-=:....
                                        ......:*@@@@=:.....
                                     ........::#@@@@-:.......
                                 ......:@-::@=%@@@@@@*-@=:::=..
                            ...........:+@@@@@@@@@@@@@@@@=@@-...
                        .............-@=@@@@@@@@@@@@@@@@@@@=:...
                     ...::...::.....:*@@@@@@@@@@@@@@@@@@@@@@=+@.
                  .......:-@-=@=#-::-@@@@@@@@@@@@@@@@@@@@@@@@:..
                ........--@@@@@@@@@-@@@@@@@@@@@@@@@@@@@@@@@@@:..
              .......:@--@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@+...
         @@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@+:....
              .......:@--@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@@+...
                ........--@@@@@@@@@-@@@@@@@@@@@@@@@@@@@@@@@@@:..
                  .......:-@-=@=#-::-@@@@@@@@@@@@@@@@@@@@@@@@:..
                     ...::...::.....:*@@@@@@@@@@@@@@@@@@@@@@=+@.
                        .............-@=@@@@@@@@@@@@@@@@@@@=:...
                            ...........:+@@@@@@@@@@@@@@@@=@@-...
                                 ......:@-::@=%@@@@@@*-@=:::=..
                                     ........::#@@@@-:.......
                                        ......:*@@@@=:.....
                                           .....:-=:....
```

## Install

```bash
git clone https://github.com/wellkyc/basalt.git
cd basalt
cargo install --path .
```

Needs Rust 1.88+.

## Usage

```bash
basalt                   # REPL
basalt program.bas       # run a file
basalt -e "PRINT 2 ^ 10"
```

In the REPL you can type statements or plain expressions. Blocks like `FOR` or `FUNCTION` keep reading lines until you close them, and everything you define stays around until `:reset`.

```
basalt 0.1.0, :help for help, :quit to exit
> 2 + 2 * 10
22
> name$ = "Basalt"
> UCASE$(name$) + "!"
"BASALT!"
> FUNCTION Square (x)
.     Square = x * x
. END FUNCTION
> Square(12)
144
```

Errors look like this:

```
runtime error: Type mismatch: cannot apply + to a number and a string
 --> bad.bas:2:7
  |
2 | PRINT x + "oops"
  |       ^^^^^^^^^^
```

## Writing a program

Put your code in a `.bas` file and run it with `basalt file.bas`. One statement per line, or several on one line separated by `:`.

Print stuff and store values. No declarations needed, a `$` at the end of a name means it's a string:

```basic
name$ = "Ann"
age = 30
PRINT "Hi, "; name$; "! Next year you'll be "; age + 1
```

Ask the user for input:

```basic
INPUT "What's your name"; name$
INPUT "How old are you"; age
```

Branches and loops:

```basic
IF age >= 18 THEN
    PRINT "adult"
ELSE
    PRINT "kid"
END IF

FOR i = 1 TO 5
    PRINT i;
NEXT

DO
    INPUT "Guess"; g
LOOP UNTIL g = 7
```

Arrays start at index 0 and `DIM a(5)` gives you `a(0)` through `a(5)`:

```basic
DIM scores(5)
FOR i = 0 TO 5
    scores(i) = i * 10
NEXT
PRINT scores(3)
```

`SUB` is a procedure, `FUNCTION` returns a value by assigning to its own name. Variables inside them are local:

```basic
SUB Greet (who$)
    PRINT "Hello, "; who$
END SUB

FUNCTION Area (w, h)
    Area = w * h
END FUNCTION

Greet "Bob"
PRINT Area(3, 4)
```

For random numbers call `RANDOMIZE TIMER` once, then `INT(RND * 6) + 1` gives a dice roll. `COLOR`, `LOCATE` and `CLS` let you draw in the terminal, see `examples/bounce.bas`.

## Examples

```basic
FUNCTION Fib (n)
    IF n < 2 THEN
        Fib = n
    ELSE
        Fib = Fib(n - 1) + Fib(n - 2)
    END IF
END FUNCTION

FOR i = 0 TO 10
    PRINT "Fib("; i; ") = "; Fib(i)
NEXT
```

```basic
FOR i = 1 TO 15
    SELECT CASE 0
        CASE i MOD 15: PRINT "FizzBuzz"
        CASE i MOD 3:  PRINT "Fizz"
        CASE i MOD 5:  PRINT "Buzz"
        CASE ELSE:     PRINT i
    END SELECT
NEXT
```

Line numbers and `GOTO` work too:

```basic
10 PRINT "Old school still works"
20 n = n + 1
30 IF n < 3 THEN 10
```

There are more in [`examples/`](examples): a prime sieve, a number guessing game, the Mandelbrot set above and a bouncing ball that uses `LOCATE` and `COLOR`.

## The language

Names are case-insensitive. A name ending in `$` holds a string, everything else can hold a number, string or boolean. Variables you never assigned are `0` (or `""` for `$` names). Writing to an array you didn't `DIM` creates one with indexes `0..10`, same as QBasic.

Operators, lowest precedence first: `XOR`, `OR`, `AND`, `NOT`, comparisons (`= <> < > <= >=`), `+ -`, `MOD`, `\` (integer division), `* /`, unary minus, `^`. `AND`/`OR` short-circuit on booleans and work bitwise on numbers.

Statements:

- `PRINT` (or `?`): `;` glues values together, `,` jumps to the next 14-column zone, a trailing `;` keeps the cursor on the line
- `INPUT "Name"; name$` or `INPUT a, b`, asks again if you type something that isn't a number
- `IF ... THEN ... ELSE` on one line, or a block with `ELSEIF` / `ELSE` / `END IF`
- `FOR i = 10 TO 1 STEP -2 ... NEXT`, `WHILE ... WEND`, `DO [WHILE|UNTIL] ... LOOP [WHILE|UNTIL]`
- `SELECT CASE` with `CASE 1, 2`, `CASE 3 TO 5`, `CASE IS > 10`, `CASE ELSE`
- `EXIT FOR / DO / WHILE / SUB / FUNCTION`
- `SUB` and `FUNCTION`; a function returns by assigning to its own name. Call subs with `CALL Draw(1, 2)`, `Draw 1, 2` or `Draw(1, 2)`
- `DIM a(10)`, `DIM grid(5, 5)`, `DIM s AS STRING`, `DIM SHARED total` for globals that subs can see
- `CONST`, `SWAP`, `DATA` / `READ` / `RESTORE`
- `GOTO` and `GOSUB` / `RETURN` to a line number or a `label:`
- `RANDOMIZE`, `CLS`, `COLOR fg, bg` (0-15 like QBasic), `LOCATE row, col`, `SLEEP seconds`, `BEEP`
- `END` / `STOP`, comments with `REM` or `'`

Built-in functions: `ABS SGN INT FIX ROUND SQR SIN COS TAN ATN EXP LOG MIN MAX RND TIMER`, `LEN LEFT$ RIGHT$ MID$ INSTR UCASE$ LCASE$ LTRIM$ RTRIM$ TRIM$ SPACE$ STRING$`, `STR$ VAL CHR$ ASC`, `UBOUND LBOUND`.

Things that work differently from QBasic:

- comparisons give real `TRUE`/`FALSE`, not `-1`/`0`
- numbers print without the extra leading space
- arguments are passed by value (arrays by reference)
- `SLEEP` with no argument waits one second instead of waiting for a key

## Inside

`lexer.rs` turns the source into tokens and remembers line/column for each one. `parser.rs` is recursive descent for statements and a Pratt parser for expressions, and it builds the tree from `ast.rs`. `interpreter.rs` walks that tree. Values and operators are in `value.rs`, built-ins in `builtins.rs`, error printing in `error.rs`, the REPL in `repl.rs`.

`GOTO` in a tree-walking interpreter is the fun part. Every block keeps a map of its labels. A jump is returned as a signal that bubbles up through the enclosing blocks until one of them has the label, and that block continues from there. `GOSUB` works the same way plus a return stack.

The REPL knows when to wait for more lines because the parser marks errors that hit the end of input as "incomplete".

## Tests

```bash
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```

CI runs these on Linux, Windows and macOS.

## License

MIT
