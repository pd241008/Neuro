use std::collections::HashMap;
use shared_ast::r#type::Kind;

#[derive(Debug, Clone, PartialEq)]
pub enum NeuroType {
    Int,
    Float,
    Bool,
    String,
    Void,
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub type_: NeuroType,
    pub offset: usize,
    pub is_mutable: bool,
    pub is_initialized: bool,
    pub scope_level: usize,
}

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub parameters: Vec<NeuroType>,
    pub return_type: NeuroType,
}

pub struct SymbolTable {
    scopes: Vec<HashMap<String, Symbol>>,
    functions: HashMap<String, FunctionSignature>,
    current_offset: usize,
}

impl NeuroType {
    pub fn from_proto_kind(kind: i32) -> Self {
        match kind {
            k if k == Kind::Int as i32 => NeuroType::Int,
            k if k == Kind::Float as i32 => NeuroType::Float,
            k if k == Kind::Bool as i32 => NeuroType::Bool,
            k if k == Kind::String as i32 => NeuroType::String,
            k if k == Kind::Void as i32 => NeuroType::Void,
            _ => NeuroType::Custom("unknown".to_string()),
        }
    }

    pub fn to_proto_kind(&self) -> i32 {
        match self {
            NeuroType::Int => Kind::Int as i32,
            NeuroType::Float => Kind::Float as i32,
            NeuroType::Bool => Kind::Bool as i32,
            NeuroType::String => Kind::String as i32,
            NeuroType::Void => Kind::Void as i32,
            NeuroType::Custom(_) => Kind::Custom as i32,
        }
    }
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            functions: HashMap::new(),
            current_offset: 0,
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) -> Result<(), String> {
        if self.scopes.len() > 1 {
            self.scopes.pop();
            Ok(())
        } else {
            Err("Cannot pop the global scope".to_string())
        }
    }

    pub fn scope_level(&self) -> usize {
        self.scopes.len() - 1
    }

    pub fn insert(
        &mut self,
        name: &str,
        type_: NeuroType,
        is_mutable: bool,
    ) -> Option<&Symbol> {
        let offset = self.current_offset;
        self.current_offset += 1;

        let symbol = Symbol {
            name: name.to_string(),
            type_,
            offset,
            is_mutable,
            is_initialized: false,
            scope_level: self.scope_level(),
        };

        let current = self.scopes.last_mut()?;
        current.insert(name.to_string(), symbol);
        current.get(name)
    }

    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        for scope in self.scopes.iter().rev() {
            if let Some(symbol) = scope.get(name) {
                return Some(symbol);
            }
        }
        None
    }

    pub fn lookup_current_scope(&self, name: &str) -> Option<&Symbol> {
        self.scopes.last()?.get(name)
    }

    pub fn mark_initialized(&mut self, name: &str) -> Result<(), String> {
        for scope in self.scopes.iter_mut().rev() {
            if let Some(symbol) = scope.get_mut(name) {
                symbol.is_initialized = true;
                return Ok(());
            }
        }
        Err(format!("Variable `{}` not found", name))
    }

    pub fn is_initialized(&self, name: &str) -> Result<bool, String> {
        self.lookup(name)
            .map(|s| s.is_initialized)
            .ok_or_else(|| format!("Variable `{}` not found", name))
    }

    pub fn insert_function(&mut self, name: &str, sig: FunctionSignature) -> Result<(), String> {
        if self.functions.contains_key(name) {
            return Err(format!("Function `{}` already defined", name));
        }
        self.functions.insert(name.to_string(), sig);
        Ok(())
    }

    pub fn lookup_function(&self, name: &str) -> Option<&FunctionSignature> {
        self.functions.get(name)
    }
}
