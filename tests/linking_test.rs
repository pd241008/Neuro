use analyzer::audit_ast;
use shared_ast;
use prost::Message;
use std::process::Command;
use std::path::PathBuf;
use std::fs;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn project_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.parent().unwrap().to_path_buf()
}

fn unique_path(suffix: &str) -> PathBuf {
    let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    PathBuf::from(format!("/tmp/neuro_link_{}_{}", n, suffix))
}

fn make_type(kind: i32) -> Option<shared_ast::Type> {
    Some(shared_ast::Type { kind, custom_name: String::new() })
}

fn make_lit_int(val: i64) -> shared_ast::Expression {
    shared_ast::Expression {
        location: None, resolved_type: None,
        expr_kind: Some(shared_ast::expression::ExprKind::Literal(shared_ast::Literal {
            value: Some(shared_ast::literal::Value::IntVal(val)),
        })),
    }
}

fn make_fn(name: &str, params: Vec<shared_ast::Parameter>, ret_kind: i32, body: Vec<shared_ast::Statement>) -> shared_ast::Function {
    shared_ast::Function {
        name: name.to_string(), parameters: params,
        return_type: make_type(ret_kind), body, location: None,
    }
}

fn make_return(val: Option<shared_ast::Expression>) -> shared_ast::Statement {
    shared_ast::Statement {
        location: None,
        stmt_kind: Some(shared_ast::statement::StmtKind::ReturnStmt(shared_ast::ReturnStatement { value: val })),
    }
}

fn build_runtime(output_dir: &PathBuf) -> Result<PathBuf, String> {
    let runtime_dir = project_root().join("runtime");
    let runtime_a = output_dir.join("libneuro_runtime.a");
    let io_o = output_dir.join("runtime_io.o");
    let mem_o = output_dir.join("runtime_memory.o");
    let cc = "gcc";

    let compile = |src: &str, dst: &PathBuf| -> Result<(), String> {
        let out = Command::new(cc)
            .args(["-c", src, "-o", &dst.to_string_lossy()])
            .current_dir(&runtime_dir)
            .output()
            .map_err(|e| format!("Failed to compile {}: {}", src, e))?;
        if !out.status.success() {
            return Err(format!("Compile error ({}): {}", src, String::from_utf8_lossy(&out.stderr)));
        }
        Ok(())
    };

    compile("io.c", &io_o)?;
    compile("memory.c", &mem_o)?;

    let ar_out = Command::new("ar")
        .args(["rcs", &runtime_a.to_string_lossy(), &io_o.to_string_lossy(), &mem_o.to_string_lossy()])
        .output()
        .map_err(|e| format!("ar failed: {}", e))?;
    if !ar_out.status.success() {
        return Err(format!("ar error: {}", String::from_utf8_lossy(&ar_out.stderr)));
    }

    Ok(runtime_a)
}

#[test]
fn test_end_to_end_linking() {
    // Skip if clang or backend binary not available
    if which::which("clang").is_err() {
        eprintln!("Skipping linking test: clang not found");
        return;
    }
    let backend_bin = project_root().join("backend/build/neuro_backend");
    if !backend_bin.exists() {
        eprintln!("Skipping linking test: backend binary not found");
        return;
    }

    // Build a simple program: int main() { return 42; }
    let prog = shared_ast::Program {
        name: "test_link".to_string(),
        functions: vec![
            make_fn("main", vec![], 0, vec![
                make_return(Some(make_lit_int(42))),
            ]),
        ],
    };

    // Step 1: Analyze
    let encoded = prog.encode_to_vec();
    let verified = audit_ast(&encoded).expect("audit_ast should succeed");

    let verified_path = unique_path("verified.ast");
    let ll_path = unique_path("output.ll");
    let obj_path = unique_path("output.o");
    let bin_path = unique_path("output.bin");
    let out_dir = PathBuf::from("/tmp");

    fs::write(&verified_path, &verified).expect("write verified ast");

    // Step 2: Backend → LLVM IR
    let be_out = Command::new(&backend_bin)
        .arg(verified_path.to_str().unwrap())
        .arg(ll_path.to_str().unwrap())
        .output()
        .expect("backend invocation");
    assert!(be_out.status.success(),
        "Backend failed: {}", String::from_utf8_lossy(&be_out.stderr));

    // Step 3: Build runtime
    let runtime_a = build_runtime(&out_dir).expect("build runtime");

    // Step 4: Compile .ll → .o
    let compile_out = Command::new("clang")
        .args(["-c", ll_path.to_str().unwrap(), "-o", obj_path.to_str().unwrap()])
        .output()
        .expect("clang compile");
    assert!(compile_out.status.success(),
        "clang -c failed: {}", String::from_utf8_lossy(&compile_out.stderr));

    // Step 5: Link .o + runtime.a → binary
    let link_out = Command::new("clang")
        .args(["-o", bin_path.to_str().unwrap(), obj_path.to_str().unwrap(), runtime_a.to_str().unwrap()])
        .output()
        .expect("clang link");
    assert!(link_out.status.success(),
        "clang link failed: {}", String::from_utf8_lossy(&link_out.stderr));

    // Step 6: Verify binary exists and is executable
    assert!(bin_path.exists(), "Binary was not created");
    assert!(bin_path.metadata().unwrap().len() > 0, "Binary is empty");

    // Step 7: Execute the binary and verify exit code matches return value
    let run_out = Command::new(&bin_path)
        .output()
        .expect("execute binary");
    assert_eq!(run_out.status.code(), Some(42),
        "Binary should exit with code 42, got {:?}", run_out.status.code());

    // Cleanup
    for p in &[verified_path, ll_path, obj_path, bin_path,
               out_dir.join("runtime_io.o"), out_dir.join("runtime_memory.o"),
               out_dir.join("libneuro_runtime.a")] {
        fs::remove_file(p).ok();
    }
}
