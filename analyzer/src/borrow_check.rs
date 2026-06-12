use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
enum VariableState { 
    Valid,
    Moved,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum BorrowType {
    Shared,
    Exclusive,
}

struct Borrow {
    borrower: String,
    borrow_type: BorrowType,
    lifetime_end: usize,
}

struct BorrowChecker {
    variable_states: HashMap<String, VariableState>,
    active_borrows: HashMap<String, Vec<Borrow>>,
    current_line: usize,
}

impl BorrowChecker {
    fn new() -> Self {
        Self {
            variable_states: HashMap::new(),
            active_borrows: HashMap::new(),
            current_line: 0,
        }
    }

    fn expire_borrow(&mut self) {
        for (_owner, borrows) in self.active_borrows.iter_mut() {
            borrows.retain(|b| b.lifetime_end > self.current_line);
        }
    }

    fn check_read(&self, var: &str) -> Result<(), String> {
        if self.variable_states.get(var) == Some(&VariableState::Moved) {
            return Err(format!("Error: Use of moved value `{}`", var));
        }

        if let Some(borrows) = self.active_borrows.get(var) {
            if borrows.iter().any(|b| b.borrow_type == BorrowType::Exclusive) {
                return Err(format!("Error: Cannot read `{}` while mutably borrowed", var));
            }
        }

        Ok(())
    }

    fn check_write(&self, var: &str) -> Result<(), String> {
        if self.variable_states.get(var) == Some(&VariableState::Moved) {
            return Err(format!("Error: Use of moved value `{}`", var));
        }

        if let Some(borrows) = self.active_borrows.get(var) {
            if !borrows.is_empty() {
                return Err(format!("Error: Cannot write to `{}` while it is borrowed", var));
            }
        }

        Ok(())
    }

    fn declare_variable(&mut self, var: String) {
        self.variable_states.insert(var, VariableState::Valid);
    }

    fn move_variable(&mut self, var: &str) -> Result<(), String> {
        self.check_read(var)?;

        if let Some(borrows) = self.active_borrows.get(var) {
            if !borrows.is_empty() {
                return Err(format!("Error: Cannot move `{}` because it is borrowed", var));
            }
        }

        self.variable_states.insert(var.to_string(), VariableState::Moved);
        Ok(())
    }

    fn create_borrow(&mut self, owner: &str, borrower: String, borrow_type: BorrowType, duration: usize) -> Result<(), String> {
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
