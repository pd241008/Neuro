fn main() {
    prost_build::compile_protos(&["ast.proto"], &["."]).unwrap();
}
