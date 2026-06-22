pub mod neuro {
    pub mod ast {
        include!(concat!(env!("OUT_DIR"), "/neuro.ast.rs"));
    }
}

pub use neuro::ast::*;
