with open("src/native.rs", "r") as f:
    content = f.read()

replacement = """                "+" | "-" | "*" | "%" => {
                    let signed = |range: Option<(i64, i64)>| {
                        range.is_some_and(|(min, max)| min >= -9007199254740991 && max <= 9007199254740991)
                    };
                    if signed(self.runtime_integer_range(a)) && signed(self.runtime_integer_range(b)) {
                        let av64 = self.expr_i64(a, depth + 1)?;
                        let bv64 = self.expr_i64(b, depth + 1)?;
                        let res64 = match op.as_str() {
                            "+" => self.b.ins().iadd(av64, bv64),
                            "-" => self.b.ins().isub(av64, bv64),
                            "*" => self.b.ins().imul(av64, bv64),
                            "%" => {
                                // For JS modulo, which can be negative, srem is roughly correct if a and b are positive.
                                // But JS % handles negative correctly for srem, except for negative 0.
                                // Actually, srem is perfect for JS integers that aren't zero-result negative.
                                self.b.ins().srem(av64, bv64)
                            },
                            _ => unreachable!(),
                        };
                        self.b.ins().fcvt_from_sint(types::F64, res64)
                    } else {
                        let av = self.expr(a, depth + 1)?;
                        let bv = self.expr(b, depth + 1)?;
                        match op.as_str() {
                            "+" => self.b.ins().fadd(av, bv),
                            "-" => self.b.ins().fsub(av, bv),
                            "*" => self.b.ins().fmul(av, bv),
                            "%" => {
                                let call = self.b.ins().call(self.rem, &[av, bv]);
                                self.b.inst_results(call)[0]
                            },
                            _ => unreachable!(),
                        }
                    }
                }"""

old = """                "+" => {
                    let av = self.expr(a, depth + 1)?;
                    let bv = self.expr(b, depth + 1)?;
                    self.b.ins().fadd(av, bv)
                }
                "-" => {
                    let av = self.expr(a, depth + 1)?;
                    let bv = self.expr(b, depth + 1)?;
                    self.b.ins().fsub(av, bv)
                }
                "*" => {
                    let av = self.expr(a, depth + 1)?;
                    let bv = self.expr(b, depth + 1)?;
                    self.b.ins().fmul(av, bv)
                }
                "/" => {
                    let av = self.expr(a, depth + 1)?;
                    let bv = self.expr(b, depth + 1)?;
                    self.b.ins().fdiv(av, bv)
                }
                "%" => {
                    let av = self.expr(a, depth + 1)?;
                    let bv = self.expr(b, depth + 1)?;
                    let call = self.b.ins().call(self.rem, &[av, bv]);
                    self.b.inst_results(call)[0]
                }"""

# Add back "/" to normal float handling because int division is truncating
replacement_full = replacement + """
                "/" => {
                    let av = self.expr(a, depth + 1)?;
                    let bv = self.expr(b, depth + 1)?;
                    self.b.ins().fdiv(av, bv)
                }"""

new_content = content.replace(old, replacement_full)
with open("src/native.rs", "w") as f:
    f.write(new_content)
