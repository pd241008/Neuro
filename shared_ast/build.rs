fn main() {
    if std::process::Command::new("protoc")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| !s.success())
        .unwrap_or(true)
    {
        panic!("`protoc` not found in PATH. Install protobuf-compiler (e.g., `apt install protobuf-compiler` or `brew install protobuf`)");
    }
    prost_build::compile_protos(&["ast.proto"], &["."]).unwrap();
}
