use std::collections::HashMap;
use crate::error::NeuroError;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VariableState { 
    Valid,
    Moved,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BorrowType {
    Shared,
    Exclusive,
}

struct Borrow {
    borrower: String,
    borrow_type: BorrowType,
    lifetime_end: usize,
}

pub struct BorrowChecker {
    variable_states: Vec<HashMap<String, VariableState>>,
    active_borrows: HashMap<String, Vec<Borrow>>,
    current_line: usize,
}

impl BorrowChecker {
    pub fn new() -> Self {
        Self {
            variable_states: vec![HashMap::new()],
            active_borrows: HashMap::new(),
            current_line: 0,
        }
    }

    pub fn push_scope(&mut self) {
        self.variable_states.push(HashMap::new());
    }

    pub fn pop_scope(&mut self) {
        if self.variable_states.len() > 1 {
            self.variable_states.pop();
        }
    }

    pub fn expire_borrow(&mut self) {
        for (_owner, borrows) in self.active_borrows.iter_mut() {
            borrows.retain(|b| b.lifetime_end > self.current_line);
        }
    }

    fn lookup_state(&self, var: &str) -> Option<&VariableState> {
        for scope in self.variable_states.iter().rev() {
            if let Some(state) = scope.get(var) {
                return Some(state);
            }
        }
        None
    }

    pub fn check_read(&self, var: &str) -> Result<(), NeuroError> {
        if self.lookup_state(var) == Some(&VariableState::Moved) {
            return Err(NeuroError::analysis(format!("Use of moved value `{}`", var)));
        }

        if let Some(borrows) = self.active_borrows.get(var) {
            if borrows.iter().any(|b| b.borrow_type == BorrowType::Exclusive) {
                return Err(NeuroError::analysis(format!("Cannot read `{}` while mutably borrowed", var)));
            }
        }

        Ok(())
    }

    pub fn check_write(&self, var: &str) -> Result<(), NeuroError> {
        if let Some(borrows) = self.active_borrows.get(var) {
            if !borrows.is_empty() {
                return Err(NeuroError::analysis(format!("Cannot write to `{}` while it is borrowed", var)));
            }
        }

        Ok(())
    }

    pub fn declare_variable(&mut self, var: String) {
        if let Some(current) = self.variable_states.last_mut() {
            current.insert(var, VariableState::Valid);
        }
    }

    pub fn set_valid(&mut self, var: &str) {
        for scope in self.variable_states.iter_mut().rev() {
            if let Some(state) = scope.get_mut(var) {
                *state = VariableState::Valid;
                return;
            }
        }
    }

    pub fn move_variable(&mut self, var: &str) -> Result<(), NeuroError> {
        if let Some(borrows) = self.active_borrows.get(var) {
            if !borrows.is_empty() {
                return Err(NeuroError::analysis(format!("Cannot move `{}` because it is borrowed", var)));
            }
        }

        for scope in self.variable_states.iter_mut().rev() {
            if let Some(state) = scope.get_mut(var) {
                *state = VariableState::Moved;
                return Ok(());
            }
        }
        Ok(())
    }

    pub fn create_borrow(&mut self, owner: &str, borrower: String, borrow_type: BorrowType, duration: usize) -> Result<(), NeuroError> {
        match borrow_type {
            BorrowType::Shared => self.check_read(owner)?,
            BorrowType::Exclusive => self.check_write(owner)?,
        }

        let new_borrow = Borrow {
            borrower,
            borrow_type,
            lifetime_end: self.current_line + duration,
        };

        self.active_borrows.entry(owner.to_string()).or_default().push(new_borrow);
        Ok(())
    }
}
