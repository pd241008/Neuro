use std::collections::HashMap;

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

pub struct SymbolTable {
    scopes: Vec<HashMap<String, Symbol>>,
    current_offset: usize,
}

impl NeuroType {
    pub fn from_proto_kind(kind: i32) -> Self {
        match kind {
            0 => NeuroType::Int,
            1 => NeuroType::Float,
            2 => NeuroType::Bool,
            3 => NeuroType::String,
            4 => NeuroType::Void,
            _ => NeuroType::Custom("unknown".to_string()),
        }
    }
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            current_offset: 0,
        }
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
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
}
