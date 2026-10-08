with open("src/native.rs", "r") as f:
    content = f.read()

old = """            Expr::Binary(op, a, b) if matches!(op.as_str(), "+" | "-") => {
                let (amin, amax) = self.runtime_integer_range(a)?;
                let (bmin, bmax) = self.runtime_integer_range(b)?;
                let range = if op == "+" {
                    (amin.checked_add(bmin)?, amax.checked_add(bmax)?)
                } else {
                    (amin.checked_sub(bmax)?, amax.checked_sub(bmin)?)
                };
                const MAX_SAFE: i64 = 9_007_199_254_740_991;
                (range.0 >= -MAX_SAFE && range.1 <= MAX_SAFE).then_some(range)
            }"""

replacement = """            Expr::Binary(op, a, b) if matches!(op.as_str(), "+" | "-" | "*" | "%") => {
                let (amin, amax) = self.runtime_integer_range(a)?;
                let (bmin, bmax) = self.runtime_integer_range(b)?;
                let range = if op == "+" {
                    (amin.checked_add(bmin)?, amax.checked_add(bmax)?)
                } else if op == "-" {
                    (amin.checked_sub(bmax)?, amax.checked_sub(bmin)?)
                } else if op == "*" {
                    let mut vals = [
                        amin.checked_mul(bmin)?,
                        amin.checked_mul(bmax)?,
                        amax.checked_mul(bmin)?,
                        amax.checked_mul(bmax)?,
                    ];
                    vals.sort();
                    (vals[0], vals[3])
                } else {
                    // %
                    // Worst case for a % b is (-b_max, b_max)
                    let bound = bmin.abs().max(bmax.abs());
                    (-bound, bound)
                };
                const MAX_SAFE: i64 = 9_007_199_254_740_991;
                (range.0 >= -MAX_SAFE && range.1 <= MAX_SAFE).then_some(range)
            }"""

if old in content:
    content = content.replace(old, replacement)
    with open("src/native.rs", "w") as f:
        f.write(content)
    print("Patched!")
else:
    print("Old not found")
