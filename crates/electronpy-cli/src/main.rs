use anyhow::{Context, Result};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use electronpy_analysis::{analyze_module, lower_module};
use electronpy_codegen_rust::RustCodegen;
use electronpy_core::{
    export_python_ast as core_export_python_ast, find_python_command as core_find_python_command,
    CompilePipeline,
};
use electronpy_optimizer::Optimizer;
use electronpy_parser::parse_python_ast;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_help();
        std::process::exit(1);
    }

    match args[1].as_str() {
        "compile" | "c" => compile_mode(&args[2..]),
        "build" | "b" => build_mode(&args[2..]),
        "run" => run_mode(&args[2..]),
        "init" => init_mode(&args[2..]),
        "clean" => clean_mode(&args[2..]),
        "export" => export_mode(&args[2..]),
        "doctor" => doctor_mode(&args[2..]),
        "version" | "--version" | "-V" => {
            println!("ElectronPy 0.0.1");
            Ok(())
        }
        "analyze" | "profile" => analyze_mode(&args[2..]),
        "benchmark" | "bench" => benchmark_mode(&args[2..]),
        "--help" | "-h" | "help" => {
            print_help();
            Ok(())
        }
        _ => compile_mode(&args[1..]),
    }
}

fn compile_mode(args: &[String]) -> Result<()> {
    let mut output_file = "output.rs".to_string();
    let mut format = "rust".to_string();
    let mut source_only = false;
    let mut iter = args.iter();
    let input_file = iter
        .next()
        .ok_or_else(|| anyhow::anyhow!("usage: electronpy <file.py> [output.rs]"))?;

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--format" | "-f" => {
                format = iter.next().cloned().unwrap_or_else(|| "rust".to_string());
            }
            "--output" | "-o" => {
                output_file = iter.next().cloned().unwrap_or_else(|| output_file.clone());
            }
            "--source-only" => {
                source_only = true;
            }
            _ => {
                if output_file == "output.rs" {
                    output_file = arg.clone();
                }
            }
        }
    }

    let working_dir = env::current_dir()?;
    let input_path = secure_input_path(input_file, &working_dir)?;
    let output_path = secure_output_path(&output_file, &working_dir)?;
    let rust_source = transpile_python_to_rust(&input_path)?;

    println!("=== ElectronPy Compiler ===\n");
    println!("Input: {}", input_path.display());
    println!("  AST Visitor: re-enabled via ast.NodeVisitor");
    println!("  [1/6] Exporting Python AST...");
    println!("  [2/6] Parsing AST...");
    println!("  [3/6] Type checking and lowering to IR...");
    println!("  [4/6] Running optimizations...");
    println!("  [5/6] Generating Rust code...");
    println!("  [6/6] Writing output...");

    write_rust_output(&output_path, &rust_source)?;

    println!("\n=== Compilation Successful ===\n");
    println!("Output: {}", output_path.display());
    println!("Mode: source-only pipeline (Rust toolchain not required)");

    if !source_only
        && (format == "exe" || output_path.extension().and_then(|s| s.to_str()) == Some("exe"))
    {
        compile_rust_binary(&output_path, &rust_source)?;
    } else {
        println!("\n=== Generated Rust Code ===\n{}", rust_source);
    }

    Ok(())
}

fn build_mode(args: &[String]) -> Result<()> {
    let mut output_file = "electronpy_app.exe".to_string();
    let mut source_only = false;
    let mut iter = args.iter();
    let input_file = iter
        .next()
        .ok_or_else(|| anyhow::anyhow!("usage: electronpy build <file.py> [output.exe]"))?;
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--output" | "-o" => {
                output_file = iter.next().cloned().unwrap_or_else(|| output_file.clone());
            }
            "--source-only" => {
                source_only = true;
            }
            _ => {
                if output_file == "electronpy_app.exe" {
                    output_file = arg.clone();
                }
            }
        }
    }

    let working_dir = env::current_dir()?;
    let input_path = secure_input_path(input_file, &working_dir)?;
    let output_path = secure_output_path(&output_file, &working_dir)?;
    let rust_source = transpile_python_to_rust(&input_path)?;
    let rust_output = output_path.with_extension("rs");

    write_rust_output(&rust_output, &rust_source)?;

    if source_only {
        println!("Source-only build complete: {}", rust_output.display());
        println!("Rust is not required for source generation. Use `electronpy build ... --source-only` to emit Rust without native EXE compilation.");
        return Ok(());
    }

    match compile_rust_binary(&output_path, &rust_source) {
        Ok(()) => {
            println!("Build complete: {}", output_path.display());
            Ok(())
        }
        Err(err) => {
            eprintln!(
                "Rust native build unavailable; source generation succeeded at {}",
                rust_output.display()
            );
            eprintln!("Install Rust or use `electronpy compile <file.py> --source-only` to keep source mode working without a Rust toolchain.");
            Err(err)
        }
    }
}

fn run_mode(args: &[String]) -> Result<()> {
    let input_file = args
        .first()
        .ok_or_else(|| anyhow::anyhow!("usage: electronpy run <file.py> [args...]"))?;
    let working_dir = env::current_dir()?;
    let input_path = secure_input_path(input_file, &working_dir)?;
    let rust_source = transpile_python_to_rust(&input_path)?;
    let temp_dir = working_dir.join(".build-tmp");
    fs::create_dir_all(&temp_dir).ok();
    let exe_path = temp_dir.join(format!(
        "{}{}",
        input_path.file_stem().unwrap_or_default().to_string_lossy(),
        if cfg!(windows) { ".exe" } else { "" }
    ));
    fs::write(temp_dir.join("runtime_run.rs"), &rust_source)?;
    compile_rust_binary(&exe_path, &rust_source)?;
    let mut cmd = Command::new(&exe_path);
    for arg in &args[1..] {
        cmd.arg(arg);
    }
    let status = cmd
        .status()
        .with_context(|| format!("failed to execute {}", exe_path.display()))?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}

fn doctor_mode(_args: &[String]) -> Result<()> {
    println!("ElectronPy doctor");
    println!(
        "Python: {}",
        find_python_command()
            .map(|p| p)
            .unwrap_or_else(|_| "not found".to_string())
    );
    println!(
        "Rust toolchain: {}",
        find_rustc_command()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "not found".to_string())
    );
    println!("Auto-bootstrap: available via rustup when a Rust toolchain is missing");
    println!("Safe project root: {}", env::current_dir()?.display());
    Ok(())
}

fn transpile_python_to_rust(input_path: &Path) -> Result<String> {
    CompilePipeline::transpile_file(input_path)
}

fn write_rust_output(output_path: &Path, rust_source: &str) -> Result<()> {
    CompilePipeline::write_rust_output(output_path, rust_source)
}

fn export_python_ast(source: &str) -> Result<String> {
    core_export_python_ast(source)
}

fn find_python_command() -> Result<String> {
    core_find_python_command()
}

fn compile_rust_binary(output_path: &Path, rust_source: &str) -> Result<()> {
    let rustc = ensure_rust_toolchain()?;
    let rust_file = output_path.with_extension("rs");
    fs::write(&rust_file, rust_source)
        .with_context(|| format!("failed to write {}", rust_file.display()))?;
    let status = Command::new(&rustc)
        .arg("-O")
        .arg("-C")
        .arg("target-cpu=native")
        .arg("-o")
        .arg(output_path)
        .arg(&rust_file)
        .status()
        .with_context(|| format!("failed to compile Rust target {}", output_path.display()))?;
    if !status.success() {
        return Err(anyhow::anyhow!(
            "native Rust compilation failed for {}",
            output_path.display()
        ));
    }
    Ok(())
}

fn ensure_rust_toolchain() -> Result<PathBuf> {
    if let Some(path) = find_rustc_command() {
        return Ok(path);
    }

    for candidate in [PathBuf::from("rustup"), PathBuf::from("rustup.exe")] {
        let tool = if candidate.is_absolute() {
            candidate
        } else {
            which_simple(&candidate.to_string_lossy())?.unwrap_or(candidate)
        };
        if !tool.exists() {
            continue;
        }
        let install = Command::new(&tool)
            .args(["toolchain", "install", "stable", "--profile", "minimal"])
            .status()
            .with_context(|| {
                format!(
                    "failed to bootstrap Rust toolchain using {}",
                    tool.display()
                )
            })?;
        if !install.success() {
            continue;
        }
        if let Some(path) = find_rustc_command() {
            return Ok(path);
        }
    }

    Err(anyhow::anyhow!(
        "Rust is not installed and could not be auto-bootstrapped. Source-only mode is still available via `electronpy compile <file.py> output.rs`."
    ))
}

fn sanitize_package_name(name: &str) -> String {
    let mut sanitized: String = name
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| match c {
            'a'..='z' | '0'..='9' => c,
            '-' | '_' | '.' => '_',
            _ => '_',
        })
        .collect();

    while sanitized.starts_with('_') {
        sanitized.remove(0);
    }
    while sanitized.ends_with('_') {
        sanitized.pop();
    }
    if sanitized.is_empty() {
        return "electronpy_project".to_string();
    }
    sanitized
}

fn init_mode(args: &[String]) -> Result<()> {
    let project_name = args
        .first()
        .cloned()
        .unwrap_or_else(|| "electronpy-app".to_string());
    let project_dir = PathBuf::from(&project_name);
    if project_dir.exists() {
        return Err(anyhow::anyhow!(
            "project directory already exists: {}",
            project_dir.display()
        ));
    }
    let src_dir = project_dir.join("src");
    fs::create_dir_all(&src_dir)
        .with_context(|| format!("failed to create {}", src_dir.display()))?;

    let package_name = sanitize_package_name(&project_name);
    let cargo_toml = format!(
        "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n",
        package_name
    );
    fs::write(project_dir.join("Cargo.toml"), cargo_toml)
        .with_context(|| format!("failed to write Cargo.toml in {}", project_dir.display()))?;

    let sample = "print(\"Hello from ElectronPy!\")\n";
    fs::write(src_dir.join("main.py"), sample)
        .with_context(|| format!("failed to write sample program in {}", src_dir.display()))?;

    fs::write(
        project_dir.join("README.md"),
        "# ElectronPy project\n\nRun:\n\n```bash\nelectronpy build main.py\n```\n",
    )
    .with_context(|| format!("failed to write README in {}", project_dir.display()))?;

    println!(
        "Initialized ElectronPy project at {}",
        project_dir.display()
    );
    println!("Next steps:");
    println!("  electronpy build {}\\main.py", project_dir.display());
    Ok(())
}

fn clean_mode(_args: &[String]) -> Result<()> {
    let root = env::current_dir()?;
    for rel in [
        ".build-tmp",
        ".electronpy-cache",
        "tmp_build",
        "out",
        "electronpy-export",
    ] {
        let target = root.join(rel);
        if target.exists() {
            if target.is_dir() {
                fs::remove_dir_all(&target)
                    .with_context(|| format!("failed to remove {}", target.display()))?;
            } else {
                fs::remove_file(&target)
                    .with_context(|| format!("failed to remove {}", target.display()))?;
            }
            println!("Removed {}", target.display());
        }
    }
    println!("Workspace cleaned.");
    Ok(())
}

fn export_mode(args: &[String]) -> Result<()> {
    let input_file = args
        .first()
        .ok_or_else(|| anyhow::anyhow!("usage: electronpy export <file.py> [output-dir]"))?;
    let output_dir = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "electronpy-export".to_string());
    let root = env::current_dir()?;
    let input_path = secure_input_path(input_file, &root)?;
    let output_path = secure_output_path(&output_dir, &root)?;

    let rust_source = transpile_python_to_rust(&input_path)?;
    let src_dir = output_path.join("src");
    fs::create_dir_all(&src_dir)
        .with_context(|| format!("failed to create {}", src_dir.display()))?;

    let project_name = sanitize_package_name(
        output_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .as_ref(),
    );
    let cargo_toml = format!(
        "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n",
        project_name
    );
    fs::write(output_path.join("Cargo.toml"), cargo_toml)
        .with_context(|| format!("failed to write Cargo.toml in {}", output_path.display()))?;
    fs::write(src_dir.join("main.rs"), &rust_source)
        .with_context(|| format!("failed to write Rust source in {}", src_dir.display()))?;
    fs::write(
        output_path.join("README.md"),
        "# Exported ElectronPy project\n\nThis project was generated by `electronpy export`.\n",
    )
    .with_context(|| format!("failed to write README in {}", output_path.display()))?;

    println!("Exported ElectronPy project to {}", output_path.display());
    println!("Build with: cargo build --release");
    Ok(())
}

fn analyze_mode(args: &[String]) -> Result<()> {
    let input_file = args
        .first()
        .ok_or_else(|| anyhow::anyhow!("usage: electronpy analyze <file.py>"))?;

    let source =
        fs::read_to_string(input_file).with_context(|| format!("failed to read {}", input_file))?;

    let ast_json = export_python_ast(&source)?;
    let module = parse_python_ast(&ast_json)?;
    let stats = analyze_module(&module);

    println!("Input: {}", input_file);
    println!("{}", stats);

    Ok(())
}

fn benchmark_mode(args: &[String]) -> Result<()> {
    let input_file = args
        .first()
        .ok_or_else(|| anyhow::anyhow!("usage: electronpy benchmark <file.py> [reference.rs]"))?;
    let reference_file = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "examples/simple.rs".to_string());

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..");
    let python_script = repo_root.join("benchmarks").join("run_benchmarks.py");
    if !python_script.exists() {
        return Err(anyhow::anyhow!(
            "could not find benchmarks/run_benchmarks.py"
        ));
    }

    let input_path = secure_input_path(input_file, &repo_root)?;
    let reference_path = secure_input_path(&reference_file, &repo_root)?;

    let _ = ensure_rust_toolchain().ok();
    let mut cmd = Command::new(find_python_command()?);
    cmd.arg(&python_script)
        .arg(&input_path)
        .arg(&reference_path)
        .arg(format!("Benchmark: {}", input_path.display()));

    let output = cmd
        .current_dir(&repo_root)
        .output()
        .with_context(|| format!("failed to run benchmark for {}", input_path.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("{}", stderr);
        std::process::exit(output.status.code().unwrap_or(1));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    print!("{}", stdout);
    Ok(())
}

fn print_help() {
    eprintln!("ElectronPy - Python to Rust compiler");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  electronpy <file.py> [output.rs]");
    eprintln!("  electronpy compile <file.py> [output.rs]");
    eprintln!("  electronpy build <file.py> [output.exe]");
    eprintln!("  electronpy run <file.py> [args...]");
    eprintln!("  electronpy analyze <file.py>");
    eprintln!("  electronpy benchmark <file.py> [reference.rs]");
    eprintln!("  electronpy doctor");
    eprintln!("  electronpy --help");
    eprintln!();
    eprintln!("Native output notes:");
    eprintln!(
        "  - `compile` emits Rust source and can optionally build an EXE via `--format exe`."
    );
    eprintln!("  - `build` directly emits a native executable when a Rust toolchain is available.");
    eprintln!("  - `run` transpiles, builds, and executes the program in one command.");
    eprintln!("  - Rust remains optional for source emission; developers can still use `electronpy compile` without final native build.");
    eprintln!();
    eprintln!("Supported subset:");
    eprintln!("  - integers, floats, bools, strings, None");
    eprintln!("  - arithmetic and comparisons");
    eprintln!("  - print(), if/else, while, for range(...) loops");
    eprintln!("  - simple function definitions and typed annotations");
    eprintln!();
    eprintln!("Example:");
    eprintln!("  def add(a: int, b: int) -> int:");
    eprintln!("      return a + b");
    eprintln!();
    eprintln!("Note: ElectronPy targets a statically analyzable subset of Python, not full Python compatibility.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn branching_workload_keeps_then_branch_assignment() {
        let source = fs::read_to_string("../../benchmarks/workloads/branching.py").unwrap();
        let ast_json = export_python_ast(&source).unwrap();
        let module = parse_python_ast(&ast_json).unwrap();
        let ir = lower_module(&module).unwrap();
        let ir = Optimizer::optimize(&ir).unwrap();
        let rust = RustCodegen::generate(&ir).unwrap();

        assert!(
            rust.contains("result = 1_i64;"),
            "missing then-branch assignment:\n{}",
            rust
        );
        assert!(
            rust.contains("result = 0_i64;"),
            "missing else-branch assignment:\n{}",
            rust
        );
    }
}

fn find_rustc_command() -> Option<PathBuf> {
    let candidates = [
        env::var("RUSTC").ok().map(PathBuf::from),
        Some(PathBuf::from("rustc")),
        Some(PathBuf::from("rustc.exe")),
        Some(PathBuf::from("rustup")),
        Some(PathBuf::from("rustup.exe")),
    ];
    for candidate in candidates.into_iter().flatten() {
        let path = if candidate.is_absolute() {
            candidate
        } else {
            which_simple(&candidate.to_string_lossy())
                .ok()
                .flatten()
                .unwrap_or(candidate)
        };
        if !path.exists() {
            continue;
        }
        let canonical = path.canonicalize().unwrap_or(path.clone());
        let file_name = canonical
            .file_name()
            .map(|s| s.to_string_lossy().to_ascii_lowercase());

        if matches!(file_name.as_deref(), Some("rustup.exe") | Some("rustup")) {
            if let Ok(output) = Command::new(&canonical).arg("which").arg("rustc").output() {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !stdout.is_empty() {
                        let resolved = PathBuf::from(stdout.lines().next().unwrap_or_default());
                        if resolved.exists() {
                            return Some(resolved.canonicalize().unwrap_or(resolved));
                        }
                    }
                }
            }
            let sibling = canonical.with_file_name("rustc.exe");
            if sibling.exists() {
                return Some(sibling.canonicalize().unwrap_or(sibling));
            }
        }

        if matches!(file_name.as_deref(), Some("rustc.exe") | Some("rustc")) {
            return Some(canonical);
        }
    }
    None
}

fn which_simple(name: &str) -> Result<Option<PathBuf>> {
    let mut cmd = Command::new("where");
    if cfg!(unix) {
        cmd = Command::new("which");
    }
    let out = cmd.arg(name).output();
    let out = match out {
        Ok(output) => output,
        Err(_) => return Ok(None),
    };
    if !out.status.success() {
        return Ok(None);
    }
    let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if stdout.is_empty() {
        return Ok(None);
    }
    let first = stdout.lines().next().unwrap_or_default();
    if first.is_empty() {
        return Ok(None);
    }
    Ok(Some(PathBuf::from(first)))
}

fn secure_input_path(input: &str, root: &Path) -> Result<PathBuf> {
    let candidate = PathBuf::from(input);
    let absolute = if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    };
    let canonical = normalize_windows_path(absolute.canonicalize().unwrap_or(absolute.clone()));
    let root_norm = normalize_windows_path(root.canonicalize().unwrap_or(root.to_path_buf()));
    if !canonical.starts_with(&root_norm) {
        return Err(anyhow::anyhow!(
            "input path is outside the safe project root: {}",
            canonical.display()
        ));
    }
    Ok(canonical)
}

fn secure_output_path(output: &str, root: &Path) -> Result<PathBuf> {
    let candidate = PathBuf::from(output);
    let absolute = if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    };
    let parent = absolute.parent().unwrap_or(root);
    if !parent.exists() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create output directory {}", parent.display()))?;
    }
    let canonical_parent =
        normalize_windows_path(parent.canonicalize().unwrap_or(parent.to_path_buf()));
    let root_norm = normalize_windows_path(root.canonicalize().unwrap_or(root.to_path_buf()));
    if !canonical_parent.starts_with(&root_norm) {
        return Err(anyhow::anyhow!(
            "output directory is outside the safe project root: {}",
            canonical_parent.display()
        ));
    }
    Ok(absolute)
}

fn normalize_windows_path(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let text = path.to_string_lossy();
        let stripped = text
            .strip_prefix("\\\\?\\UNC\\")
            .or_else(|| text.strip_prefix("\\\\?\\"))
            .unwrap_or(&text);
        PathBuf::from(stripped)
    }
    #[cfg(not(windows))]
    {
        path
    }
}
