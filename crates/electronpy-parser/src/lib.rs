use anyhow::{Context, Result};
use electronpy_ast::Module;

/// Parse JSON-serialized Python AST into ElectronPy AST
pub fn parse_python_ast(json: &str) -> Result<Module> {
    let module: Module =
        serde_json::from_str(json).context("failed to deserialize Python AST to ElectronPy AST")?;

    Ok(module)
}
