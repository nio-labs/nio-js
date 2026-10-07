//! Validated numeric AST lowered to machine code by the Rust Cranelift backend.
use anyhow::{Result, bail, ensure};
use cranelift_codegen::ir::{self, AbiParam, InstBuilder, MemFlags, condcodes::FloatCC, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};
use oxc_allocator::Allocator;
use oxc_ast::ast::*;
use oxc_parser::Parser;
use oxc_span::SourceType;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, time::Instant};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Expr {
    Number(f64),
    Local(usize),
    Unary(String, Box<Expr>),
    Binary(String, Box<Expr>, Box<Expr>),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Op {
    Set(usize, Expr),
    If(Expr, Vec<Op>, Vec<Op>),
    Loop(Expr, Vec<Op>, Vec<Op>),
    Return(Expr),
    Break,
    Continue,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Numeric {
    pub params: Vec<String>,
    pub locals: usize,
    pub ops: Vec<Op>,
    pub string: bool,
    pub builtins: Vec<String>,
}
struct Lower {
    names: HashMap<String, usize>,
    next: usize,
    immutable: std::collections::HashSet<usize>,
    boolean: std::collections::HashSet<usize>,
    builtins: std::cell::RefCell<std::collections::HashSet<String>>,
}
impl Lower {
    fn lookup(&self, name: &str) -> Result<usize> {
        self.names
            .get(name)
            .copied()
            .ok_or_else(|| anyhow::anyhow!("native free variable: {name}"))
    }
    fn is_boolean(&self, e: &Expression) -> bool {
        match e {
            Expression::BooleanLiteral(_) => true,
            Expression::Identifier(id) => self
                .names
                .get(id.name.as_str())
                .is_some_and(|i| self.boolean.contains(i)),
            Expression::ParenthesizedExpression(p) => self.is_boolean(&p.expression),
            Expression::UnaryExpression(u) => u.operator.as_str() == "!",
            Expression::BinaryExpression(b) => {
                ["<", "<=", ">", ">=", "==", "===", "!=", "!=="].contains(&b.operator.as_str())
            }
            _ => false,
        }
    }
    fn expr(&self, e: &Expression) -> Result<Expr> {
        Ok(match e {
            Expression::NumericLiteral(n) => Expr::Number(n.value),
            Expression::BooleanLiteral(n) => Expr::Number(if n.value { 1.0 } else { 0.0 }),
            Expression::Identifier(n) => Expr::Local(self.lookup(&n.name)?),
            Expression::ParenthesizedExpression(n) => self.expr(&n.expression)?,
            Expression::UnaryExpression(n) => {
                let op = n.operator.as_str();
                ensure!(
                    ["+", "-", "!", "~"].contains(&op),
                    "unsupported native unary operator"
                );
                Expr::Unary(op.into(), Box::new(self.expr(&n.argument)?))
            }
            Expression::BinaryExpression(n) => {
                let op = n.operator.as_str();
                ensure!(
                    [
                        "+", "-", "*", "/", "%", "<", "<=", ">", ">=", "===", "!==", "==", "!=",
                        "|", "&", "^", "<<", ">>", ">>>"
                    ]
                    .contains(&op),
                    "unsupported native operator"
                );
                if ["===", "!=="].contains(&op)
                    && self.is_boolean(&n.left) != self.is_boolean(&n.right)
                {
                    return Ok(Expr::Number(if op == "!==" { 1.0 } else { 0.0 }));
                }
                Expr::Binary(
                    op.into(),
                    Box::new(self.expr(&n.left)?),
                    Box::new(self.expr(&n.right)?),
                )
            }
            Expression::CallExpression(c) => {
                let Expression::StaticMemberExpression(m) = &c.callee else {
                    bail!("native call")
                };
                ensure!(!c.optional && !m.optional, "native optional call");
                let Expression::Identifier(object) = &m.object else {
                    bail!("native call receiver")
                };
                ensure!(object.name == "Math", "native call receiver");
                let name = m.property.name.as_str();
                if name == "imul" && c.arguments.len() == 2 {
                    self.builtins.borrow_mut().insert(name.into());
                    Expr::Binary(
                        "imul".into(),
                        Box::new(
                            self.expr(
                                c.arguments[0]
                                    .as_expression()
                                    .ok_or_else(|| anyhow::anyhow!("native spread argument"))?,
                            )?,
                        ),
                        Box::new(
                            self.expr(
                                c.arguments[1]
                                    .as_expression()
                                    .ok_or_else(|| anyhow::anyhow!("native spread argument"))?,
                            )?,
                        ),
                    )
                } else {
                    ensure!(
                        ["sqrt", "floor", "ceil", "trunc", "abs"].contains(&name)
                            && c.arguments.len() == 1,
                        "native Math method"
                    );
                    self.builtins.borrow_mut().insert(name.into());
                    Expr::Unary(
                        name.into(),
                        Box::new(
                            self.expr(
                                c.arguments[0]
                                    .as_expression()
                                    .ok_or_else(|| anyhow::anyhow!("native spread argument"))?,
                            )?,
                        ),
                    )
                }
            }
            _ => bail!("unsupported native expression"),
        })
    }
    fn decl(&mut self, d: &VariableDeclaration) -> Result<Vec<Op>> {
        ensure!(
            matches!(
                d.kind,
                VariableDeclarationKind::Let | VariableDeclarationKind::Const
            ),
            "native var/using declarations unsupported"
        );
        let mut out = Vec::new();
        for item in &d.declarations {
            let BindingPattern::BindingIdentifier(id) = &item.id else {
                bail!("native destructuring is unsupported")
            };
            // Duplicate/shadowed locals stay in JS rather than changing lexical semantics.
            ensure!(
                !self.names.contains_key(id.name.as_str()),
                "native shadowed local"
            );
            let value = self.expr(
                item.init
                    .as_ref()
                    .ok_or_else(|| anyhow::anyhow!("native local requires initializer"))?,
            )?;
            let index = self.next;
            self.next += 1;
            if d.kind == VariableDeclarationKind::Const {
                self.immutable.insert(index);
            }
            if item.init.as_ref().is_some_and(|e| self.is_boolean(e)) {
                self.boolean.insert(index);
            }
            self.names.insert(id.name.to_string(), index);
            out.push(Op::Set(index, value));
        }
        Ok(out)
    }
    fn update(&mut self, e: &Expression) -> Result<Op> {
        Ok(match e {
            Expression::AssignmentExpression(a) => {
                let AssignmentTarget::AssignmentTargetIdentifier(id) = &a.left else {
                    bail!("native assignment target")
                };
                let index = self.lookup(&id.name)?;
                ensure!(
                    !self.immutable.contains(&index),
                    "native assignment to const"
                );
                if a.operator.as_str() == "=" {
                    ensure!(
                        self.is_boolean(&a.right) == self.boolean.contains(&index),
                        "native changing local type"
                    );
                }
                let right = self.expr(&a.right)?;
                let op = a.operator.as_str();
                let value = if op == "=" {
                    right
                } else {
                    let binary = op.strip_suffix('=').unwrap_or(op);
                    ensure!(
                        ["+", "-", "*", "/", "%", "|", "&", "^", "<<", ">>", ">>>"]
                            .contains(&binary),
                        "native assignment operator"
                    );
                    Expr::Binary(binary.into(), Box::new(Expr::Local(index)), Box::new(right))
                };
                Op::Set(index, value)
            }
            Expression::UpdateExpression(a) => {
                let SimpleAssignmentTarget::AssignmentTargetIdentifier(id) = &a.argument else {
                    bail!("native update target")
                };
                let index = self.lookup(&id.name)?;
                ensure!(
                    !self.immutable.contains(&index),
                    "native assignment to const"
                );
                ensure!(!self.boolean.contains(&index), "native boolean increment");
                Op::Set(
                    index,
                    Expr::Binary(
                        if a.operator.as_str() == "++" {
                            "+"
                        } else {
                            "-"
                        }
                        .into(),
                        Box::new(Expr::Local(index)),
                        Box::new(Expr::Number(1.0)),
                    ),
                )
            }
            _ => bail!("unsupported native statement expression"),
        })
    }
    fn stmt(&mut self, s: &Statement) -> Result<Vec<Op>> {
        Ok(match s {
            Statement::VariableDeclaration(d) => self.decl(d)?,
            Statement::BlockStatement(b) => {
                let names = self.names.clone();
                let out = self.block(&b.body)?;
                self.names = names;
                out
            }
            Statement::ExpressionStatement(e) => vec![self.update(&e.expression)?],
            Statement::IfStatement(i) => vec![Op::If(
                self.expr(&i.test)?,
                self.stmt(&i.consequent)?,
                match &i.alternate {
                    Some(s) => self.stmt(s)?,
                    None => vec![],
                },
            )],
            Statement::WhileStatement(w) => {
                vec![Op::Loop(self.expr(&w.test)?, self.stmt(&w.body)?, vec![])]
            }
            Statement::ForStatement(f) => {
                let names = self.names.clone();
                let mut out = match &f.init {
                    Some(ForStatementInit::VariableDeclaration(d)) => self.decl(d)?,
                    None => vec![],
                    _ => bail!("native for initializer"),
                };
                let condition = match &f.test {
                    Some(e) => self.expr(e)?,
                    None => Expr::Number(1.0),
                };
                let body = self.stmt(&f.body)?;
                let update = match &f.update {
                    Some(e) => vec![self.update(e)?],
                    None => vec![],
                };
                out.push(Op::Loop(condition, body, update));
                self.names = names;
                out
            }
            Statement::ReturnStatement(r) => {
                ensure!(
                    !r.argument.as_ref().is_some_and(|e| self.is_boolean(e)),
                    "native mixed boolean return"
                );
                vec![Op::Return(self.expr(r.argument.as_ref().ok_or_else(
                    || anyhow::anyhow!("native return requires value"),
                )?)?)]
            }
            Statement::BreakStatement(b) if b.label.is_none() => vec![Op::Break],
            Statement::ContinueStatement(c) if c.label.is_none() => vec![Op::Continue],
            Statement::EmptyStatement(_) => vec![],
            _ => bail!("unsupported native statement"),
        })
    }
    fn block(&mut self, body: &[Statement]) -> Result<Vec<Op>> {
        let mut out = Vec::new();
        for stmt in body {
            out.extend(self.stmt(stmt)?);
        }
        Ok(out)
    }
}

pub fn numeric(source: &str) -> Result<Numeric> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::mjs()).parse();
    ensure!(parsed.diagnostics.is_empty(), "native syntax error");
    let Some(Statement::FunctionDeclaration(f)) = parsed.program.body.first() else {
        bail!("native function expected")
    };
    ensure!(
        !f.r#async && !f.generator && f.params.rest.is_none(),
        "native function shape"
    );
    let mut lower = Lower {
        names: HashMap::new(),
        next: 0,
        immutable: Default::default(),
        boolean: Default::default(),
        builtins: Default::default(),
    };
    let mut params = Vec::new();
    for p in &f.params.items {
        ensure!(p.initializer.is_none(), "native default argument");
        let BindingPattern::BindingIdentifier(id) = &p.pattern else {
            bail!("native parameter binding")
        };
        ensure!(
            !lower.names.contains_key(id.name.as_str()),
            "native duplicate parameter"
        );
        lower.names.insert(id.name.to_string(), lower.next);
        lower.next += 1;
        params.push(id.name.to_string());
    }
    ensure!(params.len() <= 16, "too many native parameters");
    let body = f
        .body
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("native body missing"))?;
    ensure!(body.directives.is_empty(), "native directives unsupported");
    let Some(Statement::ReturnStatement(ret)) = body.statements.last() else {
        bail!("native function requires final return")
    };
    // A numeric .toString() final return remains a string at the JS boundary.
    let string_expr = ret.argument.as_ref().and_then(|e| {
        if let Expression::CallExpression(c) = e
            && let Expression::StaticMemberExpression(m) = &c.callee
            && m.property.name == "toString"
            && c.arguments.is_empty()
            && !c.optional
            && !m.optional
        {
            Some(&m.object)
        } else {
            None
        }
    });
    let string_call = ret.argument.as_ref().and_then(|e| {
        if let Expression::CallExpression(c) = e {
            if matches!(&c.callee,Expression::Identifier(id) if id.name=="String")
                && c.arguments.len() == 1
                && !c.optional
            {
                return c.arguments[0].as_expression();
            }
            None
        } else {
            None
        }
    });
    let string_expr = string_expr.or(string_call);
    let mut ops = lower.block(&body.statements[..body.statements.len() - 1])?;
    ensure!(
        !lower.is_boolean(
            string_expr
                .or(ret.argument.as_ref())
                .ok_or_else(|| anyhow::anyhow!("native return missing"))?
        ),
        "boolean native returns stay in JavaScript"
    );
    ops.push(Op::Return(
        lower.expr(
            string_expr
                .or(ret.argument.as_ref())
                .ok_or_else(|| anyhow::anyhow!("native return missing"))?,
        )?,
    ));
    ensure!(lower.next <= 256, "too many native locals");
    Ok(Numeric {
        params,
        locals: lower.next,
        ops,
        string: string_expr.is_some(),
        builtins: {
            let mut names: Vec<String> = lower.builtins.borrow().iter().cloned().collect();
            if string_call.is_some() {
                names.push("String".into());
            }
            names.sort();
            names
        },
    })
}

pub struct RunState {
    pub deadline: Instant,
    pub failed: bool,
}
extern "C" fn tick(state: *mut RunState) -> i8 {
    // Generated functions receive this pointer only from Executable::run.
    let state = unsafe { &mut *state };
    if Instant::now() >= state.deadline {
        state.failed = true;
        0
    } else {
        1
    }
}
fn int32(value: f64) -> i32 {
    if !value.is_finite() || value == 0.0 {
        return 0;
    }
    let truncated = value.trunc().rem_euclid(4294967296.0);
    (truncated as u32) as i32
}
extern "C" fn bitwise(a: f64, b: f64, op: i32) -> f64 {
    let a = int32(a);
    let b = int32(b);
    let shift = b as u32 & 31;
    match op {
        0 => (a | b) as f64,
        1 => (a & b) as f64,
        2 => (a ^ b) as f64,
        3 => a.wrapping_shl(shift) as f64,
        4 => (a >> shift) as f64,
        5 => ((a as u32) >> shift) as f64,
        6 => a.wrapping_mul(b) as f64,
        _ => 0.0,
    }
}
extern "C" fn remainder(a: f64, b: f64) -> f64 {
    a % b
}

struct Emit<'a, 'b> {
    b: FunctionBuilder<'a>,
    vars: Vec<Variable>,
    state: ir::Value,
    tick: ir::FuncRef,
    bits: ir::FuncRef,
    rem: ir::FuncRef,
    loops: Vec<(ir::Block, ir::Block)>,
    budget: &'b mut usize,
}
impl Emit<'_, '_> {
    fn spend(&mut self) -> Result<()> {
        ensure!(*self.budget > 0, "native program too complex");
        *self.budget -= 1;
        Ok(())
    }
    fn local(&self, index: usize) -> Result<Variable> {
        self.vars
            .get(index)
            .copied()
            .ok_or_else(|| anyhow::anyhow!("invalid native local"))
    }
    fn truth(&mut self, value: ir::Value) -> ir::Value {
        let zero = self.b.ins().f64const(0.0);
        // Ordered nonzero: NaN is false, as in JavaScript.
        self.b.ins().fcmp(FloatCC::OrderedNotEqual, value, zero)
    }
    fn expr(&mut self, e: &Expr, depth: usize) -> Result<ir::Value> {
        self.spend()?;
        ensure!(depth < 64, "native expression too deep");
        Ok(match e {
            Expr::Number(n) => self.b.ins().f64const(*n),
            Expr::Local(i) => {
                let v = self.local(*i)?;
                self.b.use_var(v)
            }
            Expr::Unary(op, e) => {
                let v = self.expr(e, depth + 1)?;
                match op.as_str() {
                    "+" => v,
                    "-" => self.b.ins().fneg(v),
                    "!" => {
                        let truth = self.truth(v);
                        let no = self.b.ins().bnot(truth);
                        let n = self.b.ins().band_imm(no, 1);
                        self.b.ins().fcvt_from_uint(types::F64, n)
                    }
                    "sqrt" => self.b.ins().sqrt(v),
                    "floor" => self.b.ins().floor(v),
                    "ceil" => self.b.ins().ceil(v),
                    "trunc" => self.b.ins().trunc(v),
                    "abs" => self.b.ins().fabs(v),
                    "~" => {
                        let minus = self.b.ins().f64const(-1.0);
                        let op = self.b.ins().iconst(types::I32, 2);
                        let call = self.b.ins().call(self.bits, &[v, minus, op]);
                        self.b.inst_results(call)[0]
                    }
                    _ => bail!("invalid native unary operator"),
                }
            }
            Expr::Binary(op, a, b) => {
                let a = self.expr(a, depth + 1)?;
                let b = self.expr(b, depth + 1)?;
                match op.as_str() {
                    "+" => self.b.ins().fadd(a, b),
                    "-" => self.b.ins().fsub(a, b),
                    "*" => self.b.ins().fmul(a, b),
                    "/" => self.b.ins().fdiv(a, b),
                    "%" => {
                        let call = self.b.ins().call(self.rem, &[a, b]);
                        self.b.inst_results(call)[0]
                    }
                    "|" | "&" | "^" | "<<" | ">>" | ">>>" | "imul" => {
                        let code = match op.as_str() {
                            "|" => 0,
                            "&" => 1,
                            "^" => 2,
                            "<<" => 3,
                            ">>" => 4,
                            "imul" => 6,
                            _ => 5,
                        };
                        let code = self.b.ins().iconst(types::I32, code);
                        let call = self.b.ins().call(self.bits, &[a, b, code]);
                        self.b.inst_results(call)[0]
                    }
                    "<" | "<=" | ">" | ">=" | "==" | "===" | "!=" | "!==" => {
                        let cc = match op.as_str() {
                            "<" => FloatCC::LessThan,
                            "<=" => FloatCC::LessThanOrEqual,
                            ">" => FloatCC::GreaterThan,
                            ">=" => FloatCC::GreaterThanOrEqual,
                            "==" | "===" => FloatCC::Equal,
                            _ => FloatCC::NotEqual,
                        };
                        let test = self.b.ins().fcmp(cc, a, b);
                        self.b.ins().fcvt_from_uint(types::F64, test)
                    }
                    _ => bail!("invalid native binary operator"),
                }
            }
        })
    }
    fn block(&mut self, ops: &[Op], depth: usize) -> Result<bool> {
        ensure!(depth < 64, "native control flow too deep");
        for op in ops {
            self.spend()?;
            match op {
                Op::Set(i, e) => {
                    let value = self.expr(e, 0)?;
                    let var = self.local(*i)?;
                    self.b.def_var(var, value);
                }
                Op::Return(e) => {
                    let value = self.expr(e, 0)?;
                    self.b.ins().return_(&[value]);
                    return Ok(true);
                }
                Op::Break | Op::Continue => {
                    let &(exit, next) = self
                        .loops
                        .last()
                        .ok_or_else(|| anyhow::anyhow!("native break/continue outside loop"))?;
                    self.b
                        .ins()
                        .jump(if matches!(op, Op::Break) { exit } else { next }, &[]);
                    return Ok(true);
                }
                Op::If(test, yes, no) => {
                    let v = self.expr(test, 0)?;
                    let t = self.truth(v);
                    let yes_b = self.b.create_block();
                    let no_b = self.b.create_block();
                    let join = self.b.create_block();
                    self.b.ins().brif(t, yes_b, &[], no_b, &[]);
                    self.b.switch_to_block(yes_b);
                    let yes_end = self.block(yes, depth + 1)?;
                    if !yes_end {
                        self.b.ins().jump(join, &[]);
                    }
                    self.b.switch_to_block(no_b);
                    let no_end = self.block(no, depth + 1)?;
                    if !no_end {
                        self.b.ins().jump(join, &[]);
                    }
                    if yes_end && no_end {
                        return Ok(true);
                    }
                    self.b.switch_to_block(join);
                }
                Op::Loop(test, body, update) => {
                    let header = self.b.create_block();
                    let body_b = self.b.create_block();
                    let next = self.b.create_block();
                    let exit = self.b.create_block();
                    let timeout = self.b.create_block();
                    let check = self.b.create_block();
                    self.b.ins().jump(header, &[]);
                    self.b.switch_to_block(header);
                    let tick = self.b.ins().call(self.tick, &[self.state]);
                    let alive = self.b.inst_results(tick)[0];
                    self.b.ins().brif(alive, check, &[], timeout, &[]);
                    self.b.switch_to_block(timeout);
                    let nan = self.b.ins().f64const(f64::NAN);
                    self.b.ins().return_(&[nan]);
                    self.b.switch_to_block(check);
                    let v = self.expr(test, 0)?;
                    let t = self.truth(v);
                    self.b.ins().brif(t, body_b, &[], exit, &[]);
                    self.b.switch_to_block(body_b);
                    self.loops.push((exit, next));
                    let ended = self.block(body, depth + 1)?;
                    self.loops.pop();
                    if !ended {
                        self.b.ins().jump(next, &[]);
                    }
                    self.b.switch_to_block(next);
                    if !self.block(update, depth + 1)? {
                        self.b.ins().jump(header, &[]);
                    }
                    self.b.switch_to_block(exit);
                }
            }
        }
        Ok(false)
    }
}

pub struct Executable {
    module: Option<JITModule>,
    function: unsafe extern "C" fn(*const f64, *mut RunState) -> f64,
    params: usize,
}
impl Executable {
    pub fn compile(plan: &Numeric) -> Result<Self> {
        ensure!(
            plan.params.len() <= 16 && plan.locals <= 256 && plan.locals >= plan.params.len(),
            "invalid native limits"
        );
        let mut builder = JITBuilder::new(default_libcall_names())?;
        builder
            .symbol("nio_tick", tick as *const u8)
            .symbol("nio_bits", bitwise as *const u8)
            .symbol("nio_rem", remainder as *const u8);
        let mut module = JITModule::new(builder);
        let ptr = module.target_config().pointer_type();
        let mut sig = module.make_signature();
        sig.params.push(AbiParam::new(ptr));
        sig.returns.push(AbiParam::new(types::I8));
        let tick_id = module.declare_function("nio_tick", Linkage::Import, &sig)?;
        let mut sig = module.make_signature();
        sig.params.extend([
            AbiParam::new(types::F64),
            AbiParam::new(types::F64),
            AbiParam::new(types::I32),
        ]);
        sig.returns.push(AbiParam::new(types::F64));
        let bits_id = module.declare_function("nio_bits", Linkage::Import, &sig)?;
        let mut sig = module.make_signature();
        sig.params
            .extend([AbiParam::new(types::F64), AbiParam::new(types::F64)]);
        sig.returns.push(AbiParam::new(types::F64));
        let rem_id = module.declare_function("nio_rem", Linkage::Import, &sig)?;
        let mut ctx = module.make_context();
        ctx.func
            .signature
            .params
            .extend([AbiParam::new(ptr), AbiParam::new(ptr)]);
        ctx.func.signature.returns.push(AbiParam::new(types::F64));
        let id = module.declare_function("native", Linkage::Local, &ctx.func.signature)?;
        let tick = module.declare_func_in_func(tick_id, &mut ctx.func);
        let bits = module.declare_func_in_func(bits_id, &mut ctx.func);
        let rem = module.declare_func_in_func(rem_id, &mut ctx.func);
        let mut frontend = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut ctx.func, &mut frontend);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let args = b.block_params(entry)[0];
        let state = b.block_params(entry)[1];
        let mut vars = Vec::new();
        for index in 0..plan.locals {
            let var = b.declare_var(types::F64);
            let value = if index < plan.params.len() {
                b.ins()
                    .load(types::F64, MemFlags::new(), args, (index * 8) as i32)
            } else {
                b.ins().f64const(f64::NAN)
            };
            b.def_var(var, value);
            vars.push(var);
        }
        let mut budget = 4096;
        let mut emitter = Emit {
            b,
            vars,
            state,
            tick,
            bits,
            rem,
            loops: vec![],
            budget: &mut budget,
        };
        if !emitter.block(&plan.ops, 0)? {
            let nan = emitter.b.ins().f64const(f64::NAN);
            emitter.b.ins().return_(&[nan]);
        }
        emitter.b.seal_all_blocks();
        emitter.b.finalize();
        module.define_function(id, &mut ctx)?;
        module.finalize_definitions()?;
        // Signature is fixed above, and the executable owns the allocation for its entire lifetime.
        let function = unsafe {
            std::mem::transmute::<*const u8, unsafe extern "C" fn(*const f64, *mut RunState) -> f64>(
                module.get_finalized_function(id),
            )
        };
        Ok(Self {
            module: Some(module),
            function,
            params: plan.params.len(),
        })
    }
    pub fn run(&self, args: &[f64], deadline: Instant) -> Result<f64> {
        ensure!(args.len() == self.params, "native argument count mismatch");
        ensure!(
            Instant::now() < deadline,
            "native execution deadline exceeded"
        );
        let mut state = RunState {
            deadline,
            failed: false,
        };
        let result = unsafe { (self.function)(args.as_ptr(), &mut state) };
        ensure!(!state.failed, "native execution deadline exceeded");
        Ok(result)
    }
}
impl Drop for Executable {
    fn drop(&mut self) {
        if let Some(module) = self.module.take() {
            unsafe { module.free_memory() }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum JsonExpr {
    Literal(serde_json::Value),
    Number(Expr),
    Object(Vec<(String, JsonExpr)>),
    Array(Vec<JsonExpr>),
    Template(Vec<String>, Vec<Expr>),
    Range(Expr, usize, Box<JsonExpr>),
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonPlan {
    pub params: Vec<String>,
    pub locals: usize,
    pub value: JsonExpr,
    pub array_from: bool,
    pub builtins: Vec<String>,
}
impl Lower {
    fn json(&mut self, e: &Expression, array_from: &mut bool) -> Result<JsonExpr> {
        Ok(match e {
            Expression::ParenthesizedExpression(p) => self.json(&p.expression, array_from)?,
            Expression::StringLiteral(s) => {
                ensure!(!s.lone_surrogates, "native JSON lone surrogate");
                JsonExpr::Literal(serde_json::json!(s.value.as_str()))
            }
            Expression::BooleanLiteral(b) => JsonExpr::Literal(serde_json::json!(b.value)),
            Expression::NullLiteral(_) => JsonExpr::Literal(serde_json::Value::Null),
            Expression::ObjectExpression(o) => {
                let mut properties = Vec::new();
                for p in &o.properties {
                    let ObjectPropertyKind::ObjectProperty(p) = p else {
                        bail!("native object spread")
                    };
                    ensure!(
                        !p.computed && !p.method && p.kind == PropertyKind::Init,
                        "native object property"
                    );
                    let key = match &p.key {
                        PropertyKey::StaticIdentifier(k) => k.name.to_string(),
                        PropertyKey::StringLiteral(k) => k.value.to_string(),
                        _ => bail!("native property key"),
                    };
                    ensure!(key != "__proto__", "native prototype property");
                    properties.push((key, self.json(&p.value, array_from)?));
                }
                JsonExpr::Object(properties)
            }
            Expression::ArrayExpression(a) => {
                let mut items = Vec::new();
                for item in &a.elements {
                    items.push(
                        self.json(
                            item.as_expression()
                                .ok_or_else(|| anyhow::anyhow!("native array spread/hole"))?,
                            array_from,
                        )?,
                    );
                }
                JsonExpr::Array(items)
            }
            Expression::TemplateLiteral(t) => {
                ensure!(
                    t.quasis.iter().all(|q| !q.lone_surrogates),
                    "native template surrogate"
                );
                ensure!(
                    t.expressions.iter().all(|e| matches!(
                        e,
                        Expression::Identifier(_) | Expression::NumericLiteral(_)
                    )),
                    "native template requires a numeric local or literal"
                );
                let parts = t
                    .quasis
                    .iter()
                    .map(|q| {
                        q.value
                            .cooked
                            .as_ref()
                            .map(|s| s.to_string())
                            .ok_or_else(|| anyhow::anyhow!("native template escape"))
                    })
                    .collect::<Result<Vec<_>>>()?;
                JsonExpr::Template(
                    parts,
                    t.expressions
                        .iter()
                        .map(|e| self.expr(e))
                        .collect::<Result<Vec<_>>>()?,
                )
            }
            Expression::CallExpression(c) => {
                let Expression::StaticMemberExpression(m) = &c.callee else {
                    bail!("native JSON call")
                };
                let Expression::Identifier(object) = &m.object else {
                    bail!("native JSON call receiver")
                };
                ensure!(
                    object.name == "Array"
                        && m.property.name == "from"
                        && c.arguments.len() == 2
                        && !c.optional
                        && !m.optional,
                    "native JSON call"
                );
                let Some(Expression::ObjectExpression(length)) = c.arguments[0].as_expression()
                else {
                    bail!("native Array.from length")
                };
                ensure!(length.properties.len() == 1, "native Array.from source");
                let ObjectPropertyKind::ObjectProperty(p) = &length.properties[0] else {
                    bail!("native Array.from source")
                };
                ensure!(
                    matches!(&p.key,PropertyKey::StaticIdentifier(k) if k.name=="length")
                        && !p.computed
                        && !p.method
                        && p.kind == PropertyKind::Init,
                    "native Array.from length"
                );
                let length = self.expr(&p.value)?;
                let Some(Expression::ArrowFunctionExpression(map)) = c.arguments[1].as_expression()
                else {
                    bail!("native Array.from mapper")
                };
                ensure!(
                    !map.r#async && map.params.items.len() == 2 && map.params.rest.is_none(),
                    "native mapper signature"
                );
                let BindingPattern::BindingIdentifier(unused) = &map.params.items[0].pattern else {
                    bail!("native mapper binding")
                };
                let BindingPattern::BindingIdentifier(index) = &map.params.items[1].pattern else {
                    bail!("native mapper binding")
                };
                ensure!(
                    map.params.items.iter().all(|p| p.initializer.is_none()),
                    "native mapper defaults"
                );
                ensure!(
                    unused.name != index.name
                        && !self.names.contains_key(index.name.as_str())
                        && !self.names.contains_key(unused.name.as_str()),
                    "native mapper shadowing"
                );
                let slot = self.next;
                self.next += 1;
                self.names.insert(index.name.to_string(), slot);
                let value = self.json(
                    map.body
                        .as_expression()
                        .ok_or_else(|| anyhow::anyhow!("native mapper requires expression body"))?,
                    array_from,
                )?;
                self.names.remove(index.name.as_str());
                *array_from = true;
                JsonExpr::Range(length, slot, Box::new(value))
            }
            _ => {
                ensure!(!self.is_boolean(e), "native JSON boolean expression");
                JsonExpr::Number(self.expr(e)?)
            }
        })
    }
}
pub fn json_plan(source: &str) -> Result<JsonPlan> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, source, SourceType::mjs()).parse();
    ensure!(parsed.diagnostics.is_empty(), "native JSON syntax");
    let Some(Statement::FunctionDeclaration(f)) = parsed.program.body.first() else {
        bail!("native JSON function")
    };
    ensure!(
        !f.r#async && !f.generator && f.params.rest.is_none(),
        "native JSON function shape"
    );
    let mut lower = Lower {
        names: HashMap::new(),
        next: 0,
        immutable: Default::default(),
        boolean: Default::default(),
        builtins: Default::default(),
    };
    let mut params = Vec::new();
    for p in &f.params.items {
        ensure!(p.initializer.is_none(), "native JSON defaults");
        let BindingPattern::BindingIdentifier(id) = &p.pattern else {
            bail!("native JSON binding")
        };
        ensure!(
            !lower.names.contains_key(id.name.as_str()),
            "native JSON duplicate parameter"
        );
        lower.names.insert(id.name.to_string(), lower.next);
        lower.next += 1;
        params.push(id.name.to_string());
    }
    let body = f
        .body
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("native JSON body"))?;
    ensure!(
        body.statements.len() == 1 && body.directives.is_empty(),
        "native JSON requires return expression"
    );
    let Statement::ReturnStatement(r) = &body.statements[0] else {
        bail!("native JSON return")
    };
    let mut array_from = false;
    let value = lower.json(
        r.argument
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("native JSON return value"))?,
        &mut array_from,
    )?;
    ensure!(
        params.len() <= 16 && lower.next <= 256,
        "native JSON limits"
    );
    Ok(JsonPlan {
        params,
        locals: lower.next,
        value,
        array_from,
        builtins: {
            let mut names: Vec<String> = lower.builtins.borrow().iter().cloned().collect();
            names.sort();
            names
        },
    })
}
struct JsonRun {
    deadline: Instant,
    nodes: usize,
    bytes: usize,
    max: usize,
}
impl JsonRun {
    fn step(&mut self, bytes: usize) -> Result<()> {
        ensure!(
            Instant::now() < self.deadline,
            "native execution deadline exceeded"
        );
        ensure!(self.nodes > 0, "native JSON node limit exceeded");
        self.nodes -= 1;
        self.bytes = self
            .bytes
            .checked_add(bytes)
            .ok_or_else(|| anyhow::anyhow!("native JSON size overflow"))?;
        ensure!(self.bytes <= self.max, "native JSON body limit exceeded");
        Ok(())
    }
    fn number(&mut self, e: &Expr, locals: &[f64], depth: usize) -> Result<f64> {
        self.step(0)?;
        ensure!(depth < 64, "native JSON expression depth");
        Ok(match e {
            Expr::Number(n) => *n,
            Expr::Local(i) => *locals
                .get(*i)
                .ok_or_else(|| anyhow::anyhow!("native JSON local"))?,
            Expr::Unary(op, e) => {
                let n = self.number(e, locals, depth + 1)?;
                match op.as_str() {
                    "+" => n,
                    "-" => -n,
                    "!" => {
                        if n == 0.0 || n.is_nan() {
                            1.0
                        } else {
                            0.0
                        }
                    }
                    "~" => (!int32(n)) as f64,
                    "sqrt" => n.sqrt(),
                    "floor" => n.floor(),
                    "ceil" => n.ceil(),
                    "trunc" => n.trunc(),
                    "abs" => n.abs(),
                    _ => bail!("native JSON unary operator"),
                }
            }
            Expr::Binary(op, a, b) => {
                let a = self.number(a, locals, depth + 1)?;
                let b = self.number(b, locals, depth + 1)?;
                match op.as_str() {
                    "+" => a + b,
                    "-" => a - b,
                    "*" => a * b,
                    "/" => a / b,
                    "%" => a % b,
                    "|" => bitwise(a, b, 0),
                    "&" => bitwise(a, b, 1),
                    "^" => bitwise(a, b, 2),
                    "<<" => bitwise(a, b, 3),
                    ">>" => bitwise(a, b, 4),
                    ">>>" => bitwise(a, b, 5),
                    "imul" => bitwise(a, b, 6),
                    "<" => f64::from(a < b),
                    "<=" => f64::from(a <= b),
                    ">" => f64::from(a > b),
                    ">=" => f64::from(a >= b),
                    "==" | "===" => f64::from(a == b),
                    "!=" | "!==" => f64::from(a != b),
                    _ => bail!("native JSON numeric operator"),
                }
            }
        })
    }
    fn value(
        &mut self,
        e: &JsonExpr,
        locals: &mut [f64],
        depth: usize,
    ) -> Result<serde_json::Value> {
        self.step(1)?;
        ensure!(depth < 64, "native JSON nesting limit");
        Ok(match e {
            JsonExpr::Literal(v) => {
                ensure!(
                    v.is_null() || v.is_boolean() || v.is_string(),
                    "invalid native JSON literal"
                );
                self.step(v.as_str().map(str::len).unwrap_or(0))?;
                v.clone()
            }
            JsonExpr::Number(e) => serde_json::json!(self.number(e, locals, 0)?),
            JsonExpr::Object(fields) => {
                let mut object = serde_json::Map::new();
                for (key, value) in fields {
                    self.step(key.len())?;
                    object.insert(key.clone(), self.value(value, locals, depth + 1)?);
                }
                serde_json::Value::Object(object)
            }
            JsonExpr::Array(items) => serde_json::Value::Array(
                items
                    .iter()
                    .map(|item| self.value(item, locals, depth + 1))
                    .collect::<Result<_>>()?,
            ),
            JsonExpr::Template(parts, exprs) => {
                ensure!(parts.len() == exprs.len() + 1, "invalid native template");
                let mut s = String::new();
                for (i, part) in parts.iter().enumerate() {
                    self.step(part.len())?;
                    s.push_str(part);
                    if let Some(e) = exprs.get(i) {
                        let n = self.number(e, locals, 0)?;
                        ensure!(
                            n.is_finite() && n.fract() == 0.0 && n.abs() < 1e21,
                            "native template numeric format unsupported"
                        );
                        let text = if n == 0.0 {
                            "0".into()
                        } else {
                            format!("{n:.0}")
                        };
                        self.step(text.len())?;
                        s.push_str(&text);
                    }
                }
                serde_json::Value::String(s)
            }
            JsonExpr::Range(length, index, value) => {
                let n = self.number(length, locals, 0)?;
                let len = if n.is_nan() || n <= 0.0 {
                    0
                } else {
                    ensure!(
                        n.is_finite() && n <= self.max as f64,
                        "native JSON array limit exceeded"
                    );
                    n.floor() as usize
                };
                ensure!(*index < locals.len(), "native JSON mapper local");
                let mut out = Vec::new();
                for i in 0..len {
                    locals[*index] = i as f64;
                    out.push(self.value(value, locals, depth + 1)?);
                }
                serde_json::Value::Array(out)
            }
        })
    }
}
impl JsonPlan {
    pub fn run(&self, args: &[f64], deadline: Instant, max: usize) -> Result<String> {
        ensure!(
            self.params.len() <= 16
                && self.locals <= 256
                && self.locals >= self.params.len()
                && args.len() == self.params.len(),
            "invalid native JSON limits"
        );
        let mut locals = vec![f64::NAN; self.locals];
        locals[..args.len()].copy_from_slice(args);
        let mut run = JsonRun {
            deadline,
            nodes: max.min(1_000_000),
            bytes: 0,
            max,
        };
        let value = run.value(&self.value, &mut locals, 0)?;
        let result = serde_json::to_string(&value)?;
        ensure!(result.len() <= max, "native JSON body limit exceeded");
        Ok(result)
    }
}
