use rquickjs::{Context, Runtime, Module};
fn main() {
    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();
    ctx.with(|ctx| {
        let m = Module::declare(ctx, "test", "const a = 1;").unwrap();
        let bytes = m.write_object(false).unwrap();
    });
}
