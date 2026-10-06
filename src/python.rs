#[cfg(feature = "python")]
use anyhow::Context;
use anyhow::Result;
#[cfg(feature = "python")]
use std::ffi::CString;

#[cfg(feature = "python")]
use pyo3::prelude::*;
#[cfg(feature = "python")]
use pyo3::types::{PyList, PyTuple};

#[cfg(feature = "python")]
pub fn init_python() {
    Python::attach(|_py| {});
}

#[cfg(not(feature = "python"))]
pub fn init_python() {}

#[cfg(feature = "python")]
pub fn eval_python(code: &str) -> Result<String> {
    let c_code = CString::new(code).context("Invalid CString in python code")?;
    Python::attach(|py| -> Result<String> {
        let res = py.eval(&c_code, None, None).context("Python eval failed")?;
        Ok(res.to_string())
    })
}

#[cfg(not(feature = "python"))]
pub fn eval_python(_code: &str) -> Result<String> {
    anyhow::bail!(
        "Python support not enabled. Rebuild nio-js with: cargo build --release --features python"
    )
}

#[cfg(feature = "python")]
pub fn call_python_fn(
    module_code_or_name: &str,
    func_name: &str,
    args_json: &str,
) -> Result<String> {
    Python::attach(|py| -> Result<String> {
        let json_mod = py.import("json")?;
        let py_args = json_mod.call_method1("loads", (args_json,))?;

        let module = if module_code_or_name.contains('\n') || module_code_or_name.contains(';') {
            let c_code = CString::new(module_code_or_name)?;
            let c_file = CString::new("inline_module.py")?;
            let c_name = CString::new("inline_module")?;
            PyModule::from_code(py, &c_code, &c_file, &c_name)?
        } else if std::path::Path::new(module_code_or_name).exists() {
            let code = std::fs::read_to_string(module_code_or_name)?;
            let c_code = CString::new(code)?;
            let c_file = CString::new(module_code_or_name)?;
            let c_name = CString::new("user_module")?;
            PyModule::from_code(py, &c_code, &c_file, &c_name)?
        } else {
            let c_name = CString::new(module_code_or_name)?;
            py.import(&c_name)?
        };

        let func = module.getattr(func_name)?;
        let result = if let Ok(list) = py_args.cast::<PyList>() {
            let tuple = PyTuple::new(py, list.iter())?;
            func.call1(&tuple)?
        } else {
            func.call1((py_args,))?
        };

        if let Ok(s) = result.extract::<String>() {
            Ok(s)
        } else {
            let serialized = json_mod.call_method1("dumps", (&result,))?;
            Ok(serialized.extract::<String>()?)
        }
    })
}

#[cfg(not(feature = "python"))]
pub fn call_python_fn(_module: &str, _func: &str, _args_json: &str) -> Result<String> {
    anyhow::bail!(
        "Python support not enabled. Rebuild nio-js with: cargo build --release --features python"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_python_stub_or_eval() {
        init_python();
        #[cfg(feature = "python")]
        {
            let res = eval_python("2 + 3").unwrap();
            assert_eq!(res, "5");

            let py_code = "def add(a, b):\n    return a + b\n";
            let sum_res = call_python_fn(py_code, "add", "[10, 20]").unwrap();
            assert_eq!(sum_res, "30");
        }
    }
}
