with open("src/native.rs", "r") as f:
    content = f.read()

expr_i64 = """    fn expr_i64(&mut self, e: &Expr, depth: usize) -> Result<ir::Value> {
        let v = self.expr(e, depth)?;
        Ok(self.b.ins().fcvt_to_sint_sat(types::I64, v))
    }
"""

content = content.replace("    fn expr_i32(", expr_i64 + "    fn expr_i32(")
with open("src/native.rs", "w") as f:
    f.write(content)
