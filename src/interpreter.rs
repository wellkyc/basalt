use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::io::{BufRead, Write};
use std::rc::Rc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::ast::*;
use crate::builtins;
use crate::error::{Error, Span};
use crate::value::{self, Array, Rng, Value};

type Result<T> = std::result::Result<T, Error>;

const MAX_CALL_DEPTH: usize = 4_000;
const MAX_ARRAY_SIZE: usize = 50_000_000;
const PRINT_ZONE: usize = 14;
const DEFAULT_SEED: u64 = 0x5EED;

enum Flow {
    Next,
    Goto(String, Span),
    Gosub(String, Span),
    Return(Span),
    Exit(ExitKind, Span),
}

struct Frame {
    vars: HashMap<String, Value>,
    function: Option<String>,
}

pub struct Interpreter<W: Write, R: BufRead> {
    out: W,
    input: R,
    globals: HashMap<String, Value>,
    consts: HashMap<String, Value>,
    shared: HashSet<String>,
    frames: Vec<Frame>,
    procedures: HashMap<String, Rc<Procedure>>,
    data: Vec<DataItem>,
    data_pos: usize,
    rng: Rng,
    column: usize,
    styled: bool,
}

fn flow_error(flow: Flow, scope: &str) -> Result<()> {
    match flow {
        Flow::Next => Ok(()),
        Flow::Goto(label, span) | Flow::Gosub(label, span) => Err(Error::runtime(
            format!("Label {label} not found in {scope}"),
            span,
        )),
        Flow::Return(span) => Err(Error::runtime("RETURN without GOSUB", span)),
        Flow::Exit(kind, span) => Err(Error::runtime(
            format!("EXIT {0} used outside of a {0} block", kind.name()),
            span,
        )),
    }
}

fn check_type(name: &str, value: &Value, span: Span) -> Result<()> {
    if name.ends_with('$') && !matches!(value, Value::Str(_)) {
        return Err(Error::runtime(
            format!(
                "Type mismatch: {name} can only hold strings, got a {}",
                value.type_name()
            ),
            span,
        ));
    }
    Ok(())
}

fn ansi_color(n: f64, base: u8, span: Span) -> Result<String> {
    const ORDER: [u8; 8] = [0, 4, 2, 6, 1, 5, 3, 7];
    if !(0.0..16.0).contains(&n) {
        return Err(Error::runtime("Colors must be between 0 and 15", span));
    }
    let n = n as usize;
    let offset = if n >= 8 { base + 60 } else { base };
    Ok((ORDER[n % 8] + offset).to_string())
}

impl<W: Write, R: BufRead> Interpreter<W, R> {
    pub fn new(out: W, input: R) -> Self {
        Self {
            out,
            input,
            globals: HashMap::new(),
            consts: HashMap::new(),
            shared: HashSet::new(),
            frames: Vec::new(),
            procedures: HashMap::new(),
            data: Vec::new(),
            data_pos: 0,
            rng: Rng::new(DEFAULT_SEED),
            column: 0,
            styled: false,
        }
    }

    pub fn into_output(self) -> W {
        self.out
    }

    pub fn input_mut(&mut self) -> &mut R {
        &mut self.input
    }

    pub fn reset(&mut self) {
        self.globals.clear();
        self.consts.clear();
        self.shared.clear();
        self.frames.clear();
        self.procedures.clear();
        self.data.clear();
        self.data_pos = 0;
        self.rng = Rng::new(DEFAULT_SEED);
        self.column = 0;
    }

    pub fn has_sub(&self, name: &str) -> bool {
        self.procedures.get(name).is_some_and(|p| !p.is_function)
    }

    pub fn run(&mut self, program: &Program) -> Result<()> {
        for procedure in &program.procedures {
            self.procedures
                .insert(procedure.name.clone(), procedure.clone());
        }
        self.data.extend(program.data.iter().cloned());
        let result = self
            .exec_block(&program.body)
            .and_then(|flow| flow_error(flow, "the program"));
        self.frames.clear();
        let finished = self.finish();
        match result {
            Err(error) if error.is_halt() => finished,
            Err(error) => Err(error),
            Ok(()) => finished,
        }
    }

    pub fn eval_expression(&mut self, expr: &Expr) -> Result<Value> {
        let result = self.eval(expr);
        self.frames.clear();
        self.finish()?;
        result
    }

    pub fn end_line(&mut self) -> Result<()> {
        if self.column > 0 {
            self.write("\n")?;
        }
        self.out.flush().map_err(Error::io)
    }

    fn finish(&mut self) -> Result<()> {
        if self.styled {
            self.styled = false;
            self.write_raw("\x1b[0m")?;
        }
        self.out.flush().map_err(Error::io)
    }

    fn write_raw(&mut self, text: &str) -> Result<()> {
        self.out.write_all(text.as_bytes()).map_err(Error::io)
    }

    fn write(&mut self, text: &str) -> Result<()> {
        self.write_raw(text)?;
        match text.rfind('\n') {
            Some(i) => self.column = text[i + 1..].chars().count(),
            None => self.column += text.chars().count(),
        }
        Ok(())
    }

    fn exec_block(&mut self, block: &Block) -> Result<Flow> {
        let mut pc = 0;
        let mut returns = Vec::new();
        while pc < block.stmts.len() {
            match self.exec(&block.stmts[pc])? {
                Flow::Next => pc += 1,
                Flow::Goto(label, span) => match block.labels.get(&label) {
                    Some(&target) => pc = target,
                    None => return Ok(Flow::Goto(label, span)),
                },
                Flow::Gosub(label, span) => match block.labels.get(&label) {
                    Some(&target) => {
                        returns.push(pc + 1);
                        pc = target;
                    }
                    None => return Ok(Flow::Gosub(label, span)),
                },
                Flow::Return(span) => match returns.pop() {
                    Some(target) => pc = target,
                    None => return Ok(Flow::Return(span)),
                },
                exit => return Ok(exit),
            }
        }
        Ok(Flow::Next)
    }

    fn exec(&mut self, stmt: &Stmt) -> Result<Flow> {
        let span = stmt.span;
        match &stmt.kind {
            StmtKind::Nop | StmtKind::Label(_) => {}
            StmtKind::Assign(target, expr) => {
                let value = self.eval(expr)?;
                self.assign(target, value)?;
            }
            StmtKind::Print { items, newline } => self.print(items, *newline)?,
            StmtKind::Input {
                prompt,
                question,
                targets,
            } => self.input(prompt.as_deref(), *question, targets, span)?,
            StmtKind::If {
                branches,
                otherwise,
            } => {
                for (cond, block) in branches {
                    if self.truthy(cond)? {
                        return self.exec_block(block);
                    }
                }
                if let Some(block) = otherwise {
                    return self.exec_block(block);
                }
            }
            StmtKind::For {
                var,
                start,
                end,
                step,
                body,
            } => return self.exec_for(var, start, end, step.as_ref(), body),
            StmtKind::While { cond, body } => {
                while self.truthy(cond)? {
                    match self.exec_block(body)? {
                        Flow::Next => {}
                        Flow::Exit(ExitKind::While, _) => break,
                        other => return Ok(other),
                    }
                }
            }
            StmtKind::Do { pre, post, body } => loop {
                if let Some(cond) = pre
                    && !self.loop_continues(cond)?
                {
                    break;
                }
                match self.exec_block(body)? {
                    Flow::Next => {}
                    Flow::Exit(ExitKind::Do, _) => break,
                    other => return Ok(other),
                }
                if let Some(cond) = post
                    && !self.loop_continues(cond)?
                {
                    break;
                }
            },
            StmtKind::Select {
                subject,
                cases,
                otherwise,
            } => {
                let value = self.eval(subject)?;
                for (tests, block) in cases {
                    for test in tests {
                        if self.case_matches(&value, test)? {
                            return self.exec_block(block);
                        }
                    }
                }
                if let Some(block) = otherwise {
                    return self.exec_block(block);
                }
            }
            StmtKind::Exit(kind) => return Ok(Flow::Exit(*kind, span)),
            StmtKind::Call(name, args) => match self.procedures.get(name).cloned() {
                Some(procedure) => {
                    self.invoke(&procedure, args, span)?;
                }
                None => return Err(Error::runtime(format!("Undefined SUB {name}"), span)),
            },
            StmtKind::Dim { shared, decls } => self.dim(*shared, decls)?,
            StmtKind::Const(name, expr) => {
                let value = self.eval(expr)?;
                self.consts.insert(name.clone(), value);
            }
            StmtKind::Goto(label) => return Ok(Flow::Goto(label.clone(), span)),
            StmtKind::Gosub(label) => return Ok(Flow::Gosub(label.clone(), span)),
            StmtKind::Return => return Ok(Flow::Return(span)),
            StmtKind::Swap(a, b) => {
                let first = self.read_target(a)?;
                let second = self.read_target(b)?;
                self.assign(a, second)?;
                self.assign(b, first)?;
            }
            StmtKind::Randomize(seed) => {
                let seed = match seed {
                    Some(expr) => self.number(expr)?.to_bits(),
                    None => SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_nanos() as u64,
                };
                self.rng = Rng::new(seed);
            }
            StmtKind::Cls => {
                self.write_raw("\x1b[2J\x1b[H")?;
                self.column = 0;
            }
            StmtKind::Color(fg, bg) => self.color(fg.as_ref(), bg.as_ref())?,
            StmtKind::Locate(row, col) => {
                let row = self.number(row)?.max(1.0) as usize;
                let col = self.number(col)?.max(1.0) as usize;
                self.write_raw(&format!("\x1b[{row};{col}H"))?;
                self.column = col - 1;
            }
            StmtKind::Sleep(seconds) => {
                let seconds = match seconds {
                    Some(expr) => self.number(expr)?,
                    None => 1.0,
                };
                self.out.flush().map_err(Error::io)?;
                if seconds > 0.0 {
                    std::thread::sleep(Duration::from_secs_f64(seconds.min(86_400.0)));
                }
            }
            StmtKind::Beep => self.write_raw("\x07")?,
            StmtKind::Read(targets) => {
                for target in targets {
                    let item = self
                        .data
                        .get(self.data_pos)
                        .cloned()
                        .ok_or_else(|| Error::runtime("Out of DATA", target.span))?;
                    self.data_pos += 1;
                    let value = match item {
                        DataItem::Num(n) => Value::Num(n),
                        DataItem::Str(s) => Value::str(s),
                    };
                    self.assign(target, value)?;
                }
            }
            StmtKind::Restore => self.data_pos = 0,
            StmtKind::End => return Err(Error::halt()),
        }
        Ok(Flow::Next)
    }

    fn exec_for(
        &mut self,
        var: &str,
        start: &Expr,
        end: &Expr,
        step: Option<&Expr>,
        body: &Block,
    ) -> Result<Flow> {
        let first = self.number(start)?;
        let last = self.number(end)?;
        let step = match step {
            Some(expr) => self.number(expr)?,
            None => 1.0,
        };
        self.set_var(var, Value::Num(first), start.span)?;
        loop {
            let current = self.get_var(var).as_number(start.span)?;
            if (step >= 0.0 && current > last) || (step < 0.0 && current < last) {
                break;
            }
            match self.exec_block(body)? {
                Flow::Next => {}
                Flow::Exit(ExitKind::For, _) => break,
                other => return Ok(other),
            }
            let current = self.get_var(var).as_number(start.span)?;
            self.set_var(var, Value::Num(current + step), start.span)?;
        }
        Ok(Flow::Next)
    }

    fn loop_continues(&mut self, cond: &LoopCondition) -> Result<bool> {
        let value = self.truthy(&cond.expr)?;
        Ok(value != cond.until)
    }

    fn case_matches(&mut self, value: &Value, test: &CaseTest) -> Result<bool> {
        match test {
            CaseTest::Value(expr) => {
                let other = self.eval(expr)?;
                Ok(value::compare(value, &other, expr.span)?.is_eq())
            }
            CaseTest::Range(low, high) => {
                let low_value = self.eval(low)?;
                let high_value = self.eval(high)?;
                Ok(value::compare(value, &low_value, low.span)?.is_ge()
                    && value::compare(value, &high_value, high.span)?.is_le())
            }
            CaseTest::Is(op, expr) => {
                let other = self.eval(expr)?;
                Ok(value::compare_op(
                    *op,
                    value::compare(value, &other, expr.span)?,
                ))
            }
        }
    }

    fn print(&mut self, items: &[PrintItem], newline: bool) -> Result<()> {
        for item in items {
            match item {
                PrintItem::Expr(expr) => {
                    let value = self.eval(expr)?;
                    if let Value::Array(_) = value {
                        return Err(Error::runtime("Cannot PRINT a whole array", expr.span));
                    }
                    self.write(&value.to_string())?;
                }
                PrintItem::Semicolon => {}
                PrintItem::Comma => {
                    let pad = PRINT_ZONE - self.column % PRINT_ZONE;
                    self.write(&" ".repeat(pad))?;
                }
            }
        }
        if newline {
            self.write("\n")?;
        }
        Ok(())
    }

    fn input(
        &mut self,
        prompt: Option<&str>,
        question: bool,
        targets: &[Target],
        span: Span,
    ) -> Result<()> {
        loop {
            if let Some(prompt) = prompt {
                self.write(prompt)?;
            }
            if question {
                self.write("? ")?;
            }
            self.out.flush().map_err(Error::io)?;
            let mut line = String::new();
            if self.input.read_line(&mut line).map_err(Error::io)? == 0 {
                return Err(Error::runtime("Input ended unexpectedly", span));
            }
            self.column = 0;
            let line = line.trim_end_matches(['\r', '\n']);
            let parts: Vec<&str> = if targets.len() == 1 {
                vec![line]
            } else {
                line.split(',').collect()
            };
            if parts.len() == targets.len() {
                let values: Option<Vec<Value>> = parts
                    .iter()
                    .zip(targets)
                    .map(|(text, target)| self.parse_input(text, target))
                    .collect();
                if let Some(values) = values {
                    for (target, value) in targets.iter().zip(values) {
                        self.assign(target, value)?;
                    }
                    return Ok(());
                }
            }
            self.write("?Redo from start\n")?;
        }
    }

    fn parse_input(&self, text: &str, target: &Target) -> Option<Value> {
        let wants_string = target.name.ends_with('$')
            || match self.lookup(&target.name) {
                Some(Value::Str(_)) => true,
                Some(Value::Array(array)) => {
                    matches!(array.borrow().data.first(), Some(Value::Str(_)))
                }
                _ => false,
            };
        if wants_string {
            Some(Value::str(text))
        } else {
            text.trim().parse().ok().map(Value::Num)
        }
    }

    fn color(&mut self, fg: Option<&Expr>, bg: Option<&Expr>) -> Result<()> {
        if fg.is_none() && bg.is_none() {
            self.styled = false;
            return self.write_raw("\x1b[0m");
        }
        let mut codes = Vec::new();
        if let Some(expr) = fg {
            codes.push(ansi_color(self.number(expr)?, 30, expr.span)?);
        }
        if let Some(expr) = bg {
            codes.push(ansi_color(self.number(expr)?, 40, expr.span)?);
        }
        self.styled = true;
        self.write_raw(&format!("\x1b[{}m", codes.join(";")))
    }

    fn dim(&mut self, shared: bool, decls: &[DimDecl]) -> Result<()> {
        for decl in decls {
            if shared && !self.frames.is_empty() {
                return Err(Error::runtime(
                    "DIM SHARED is only allowed at the top level",
                    decl.span,
                ));
            }
            let fill = if decl.string {
                Value::str("")
            } else {
                Value::Num(0.0)
            };
            let value = match &decl.bounds {
                Some(bounds) => self.new_array(bounds, fill, decl.span)?,
                None => fill,
            };
            if shared {
                self.shared.insert(decl.name.clone());
            }
            match self.frames.last_mut() {
                Some(frame) => frame.vars.insert(decl.name.clone(), value),
                None => self.globals.insert(decl.name.clone(), value),
            };
        }
        Ok(())
    }

    fn new_array(&mut self, bounds: &[Expr], fill: Value, span: Span) -> Result<Value> {
        if bounds.is_empty() {
            return Err(Error::runtime(
                "An array needs at least one dimension",
                span,
            ));
        }
        let mut dims = Vec::new();
        let mut total: usize = 1;
        for bound in bounds {
            let n = self.number(bound)?;
            if n < 0.0 || n >= MAX_ARRAY_SIZE as f64 {
                return Err(Error::runtime("Invalid array size", bound.span));
            }
            total = total.saturating_mul(n as usize + 1);
            dims.push(n as usize);
        }
        if total > MAX_ARRAY_SIZE {
            return Err(Error::runtime("Array is too large", span));
        }
        Ok(Value::Array(Rc::new(RefCell::new(Array::new(dims, fill)))))
    }

    fn lookup(&self, name: &str) -> Option<&Value> {
        let found = match self.frames.last() {
            Some(frame) => frame.vars.get(name).or_else(|| {
                if self.shared.contains(name) {
                    self.globals.get(name)
                } else {
                    None
                }
            }),
            None => self.globals.get(name),
        };
        found.or_else(|| self.consts.get(name))
    }

    fn get_var(&self, name: &str) -> Value {
        self.lookup(name)
            .cloned()
            .unwrap_or_else(|| Value::default_for(name))
    }

    fn scope_mut(&mut self, name: &str) -> &mut HashMap<String, Value> {
        let local = self
            .frames
            .last()
            .is_some_and(|frame| frame.vars.contains_key(name) || !self.shared.contains(name));
        match self.frames.last_mut() {
            Some(frame) if local => &mut frame.vars,
            _ => &mut self.globals,
        }
    }

    fn set_var(&mut self, name: &str, value: Value, span: Span) -> Result<()> {
        if self.consts.contains_key(name) {
            return Err(Error::runtime(
                format!("Cannot assign to constant {name}"),
                span,
            ));
        }
        check_type(name, &value, span)?;
        self.scope_mut(name).insert(name.to_string(), value);
        Ok(())
    }

    fn assign(&mut self, target: &Target, value: Value) -> Result<()> {
        let Some(indices) = &target.indices else {
            return self.set_var(&target.name, value, target.span);
        };
        check_type(&target.name, &value, target.span)?;
        let array = match self.lookup(&target.name).cloned() {
            Some(Value::Array(array)) => array,
            Some(_) => {
                return Err(Error::runtime(
                    format!("{} is not an array", target.name),
                    target.span,
                ));
            }
            None => {
                let array = Rc::new(RefCell::new(Array::new(
                    vec![10; indices.len()],
                    Value::default_for(&target.name),
                )));
                self.scope_mut(&target.name)
                    .insert(target.name.clone(), Value::Array(array.clone()));
                array
            }
        };
        let offset = self.offset(&array, indices, target.span)?;
        array.borrow_mut().data[offset] = value;
        Ok(())
    }

    fn read_target(&mut self, target: &Target) -> Result<Value> {
        match &target.indices {
            None => Ok(self.get_var(&target.name)),
            Some(indices) => self.eval_call(&target.name, indices, target.span),
        }
    }

    fn offset(
        &mut self,
        array: &Rc<RefCell<Array>>,
        indices: &[Expr],
        span: Span,
    ) -> Result<usize> {
        let mut values = Vec::with_capacity(indices.len());
        for index in indices {
            values.push(self.number(index)?);
        }
        let array = array.borrow();
        if values.len() != array.bounds.len() {
            return Err(Error::runtime(
                format!(
                    "Wrong number of dimensions: expected {}, got {}",
                    array.bounds.len(),
                    values.len()
                ),
                span,
            ));
        }
        array
            .offset(&values)
            .ok_or_else(|| Error::runtime("Subscript out of range", span))
    }

    fn number(&mut self, expr: &Expr) -> Result<f64> {
        self.eval(expr)?.as_number(expr.span)
    }

    fn truthy(&mut self, expr: &Expr) -> Result<bool> {
        self.eval(expr)?.is_truthy(expr.span)
    }

    fn eval(&mut self, expr: &Expr) -> Result<Value> {
        match &expr.kind {
            ExprKind::Number(n) => Ok(Value::Num(*n)),
            ExprKind::Str(s) => Ok(Value::Str(s.clone())),
            ExprKind::Bool(b) => Ok(Value::Bool(*b)),
            ExprKind::Var(name) => self.eval_var(name, expr.span),
            ExprKind::Call(name, args) => self.eval_call(name, args, expr.span),
            ExprKind::Unary(op, operand) => {
                let value = self.eval(operand)?;
                value::unary(*op, value, expr.span)
            }
            ExprKind::Binary(op, lhs, rhs) => self.eval_binary(*op, lhs, rhs, expr.span),
        }
    }

    fn eval_var(&mut self, name: &str, span: Span) -> Result<Value> {
        let in_own_function = self
            .frames
            .last()
            .is_some_and(|frame| frame.function.as_deref() == Some(name));
        if !in_own_function {
            if let Some(procedure) = self.procedures.get(name).cloned()
                && procedure.is_function
            {
                return self.invoke(&procedure, &[], span);
            }
            if builtins::takes_no_args(name)
                && self.lookup(name).is_none()
                && let Some(result) = builtins::call(name, Vec::new(), &mut self.rng, span)
            {
                return result;
            }
        }
        Ok(self.get_var(name))
    }

    fn eval_call(&mut self, name: &str, args: &[Expr], span: Span) -> Result<Value> {
        if let Some(procedure) = self.procedures.get(name).cloned() {
            if !procedure.is_function {
                return Err(Error::runtime(
                    format!("{name} is a SUB and does not return a value"),
                    span,
                ));
            }
            return self.invoke(&procedure, args, span);
        }
        if let Some(Value::Array(array)) = self.lookup(name).cloned() {
            let offset = self.offset(&array, args, span)?;
            return Ok(array.borrow().data[offset].clone());
        }
        let mut values = Vec::with_capacity(args.len());
        for arg in args {
            values.push(self.eval(arg)?);
        }
        builtins::call(name, values, &mut self.rng, span).unwrap_or_else(|| {
            Err(Error::runtime(
                format!("Undefined function or array {name}"),
                span,
            ))
        })
    }

    fn eval_binary(&mut self, op: BinOp, lhs: &Expr, rhs: &Expr, span: Span) -> Result<Value> {
        let left = self.eval(lhs)?;
        if let (BinOp::And | BinOp::Or, Value::Bool(l)) = (op, &left) {
            if (op == BinOp::And) != *l {
                return Ok(Value::Bool(*l));
            }
            let right = self.eval(rhs)?;
            return match right {
                Value::Bool(r) => Ok(Value::Bool(r)),
                other => Err(Error::runtime(
                    format!(
                        "Type mismatch: expected a boolean, got a {}",
                        other.type_name()
                    ),
                    rhs.span,
                )),
            };
        }
        let right = self.eval(rhs)?;
        value::binary(op, left, right, span)
    }

    fn invoke(&mut self, procedure: &Procedure, args: &[Expr], span: Span) -> Result<Value> {
        if args.len() != procedure.params.len() {
            return Err(Error::runtime(
                format!(
                    "{} expects {} argument(s), got {}",
                    procedure.name,
                    procedure.params.len(),
                    args.len()
                ),
                span,
            ));
        }
        if self.frames.len() >= MAX_CALL_DEPTH {
            return Err(Error::runtime(
                "Stack overflow: recursion is too deep",
                span,
            ));
        }
        let mut vars = HashMap::new();
        for (param, arg) in procedure.params.iter().zip(args) {
            let value = self.eval(arg)?;
            check_type(param, &value, arg.span)?;
            vars.insert(param.clone(), value);
        }
        let function = procedure.is_function.then(|| procedure.name.clone());
        if let Some(name) = &function {
            vars.entry(name.clone())
                .or_insert_with(|| Value::default_for(name));
        }
        self.frames.push(Frame { vars, function });
        let flow = self.exec_block(&procedure.body);
        let mut frame = self.frames.pop().expect("call frame");
        match flow? {
            Flow::Exit(ExitKind::Sub | ExitKind::Function, _) => {}
            other => flow_error(other, &procedure.name)?,
        }
        Ok(frame
            .vars
            .remove(&procedure.name)
            .filter(|_| procedure.is_function)
            .unwrap_or(Value::Num(0.0)))
    }
}
