use std::io::Result;

fn main() -> Result<()> {
    prost_build::compile_protos(&["../shared_ast/ast.proto"], &["../shared_ast/"])?;
    Ok(())
}
