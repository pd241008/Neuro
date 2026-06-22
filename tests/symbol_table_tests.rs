use analyzer::symbol_table::{SymbolTable, NeuroType};

#[test]
fn insert_and_lookup() {
    let mut st = SymbolTable::new();
    st.insert("x", NeuroType::Int, false);
    assert!(st.lookup("x").is_some());
    assert_eq!(st.lookup("x").unwrap().name, "x");
}

#[test]
fn scope_hiding() {
    let mut st = SymbolTable::new();
    st.insert("x", NeuroType::Int, false);
    st.push_scope();
    st.insert("x", NeuroType::Float, false);
    assert_eq!(st.lookup("x").unwrap().type_, NeuroType::Float);
    let _ = st.pop_scope();
    assert_eq!(st.lookup("x").unwrap().type_, NeuroType::Int);
}

#[test]
fn lookup_unknown() {
    let st = SymbolTable::new();
    assert!(st.lookup("nonexistent").is_none());
}

#[test]
fn mark_initialized_tracking() {
    let mut st = SymbolTable::new();
    st.insert("x", NeuroType::Int, false);
    assert!(!st.is_initialized("x").unwrap());
    st.mark_initialized("x").unwrap();
    assert!(st.is_initialized("x").unwrap());
}

#[test]
fn offset_increments() {
    let mut st = SymbolTable::new();
    st.insert("a", NeuroType::Int, false);
    st.insert("b", NeuroType::Float, false);
    assert_eq!(st.lookup("a").unwrap().offset, 0);
    assert_eq!(st.lookup("b").unwrap().offset, 1);
}

#[test]
fn scope_levels() {
    let mut st = SymbolTable::new();
    assert_eq!(st.scope_level(), 0);
    st.push_scope();
    assert_eq!(st.scope_level(), 1);
    st.push_scope();
    assert_eq!(st.scope_level(), 2);
    let _ = st.pop_scope();
    assert_eq!(st.scope_level(), 1);
}
