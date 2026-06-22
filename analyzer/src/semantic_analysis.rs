use shared_ast::{
    Program, Function, Statement, Expression, VariableDeclaration,
    statement, expression, literal,
};
use crate::symbol_table::{SymbolTable, NeuroType};

pub fn analyze_ast(program: &Program) -> Result<(), String> {
    let mut ctx = AnalysisContext::new();
    ctx.visit_program(program)
}

struct AnalysisContext {
    symbol_table: SymbolTable,
    current_return_type: Option<NeuroType>,
}

impl AnalysisContext {
    fn new() -> Self {
        Self {
            symbol_table: SymbolTable::new(),
            current_return_type: None,
        }
    }

    fn visit_program(&mut self, program: &Program) -> Result<(), String> {
        for function in &program.functions {
            self.visit_function(function)?;
        }
        Ok(())
    }

    fn visit_function(&mut self, function: &Function) -> Result<(), String> {
        self.symbol_table.push_scope();

        for param in &function.parameters {
            let param_kind = param.r#type.as_ref().map_or(5, |t| t.kind);
            let param_type = NeuroType::from_proto_kind(param_kind);
            self.symbol_table.insert(&param.name, param_type, false);
            self.symbol_table.mark_initialized(&param.name)
                .map_err(|e| format!("In function `{}`: {}", function.name, e))?;
        }

        let declared_return = function.return_type.as_ref()
            .map(|t| NeuroType::from_proto_kind(t.kind));
        self.current_return_type = declared_return;

        for stmt in &function.body {
            self.visit_statement(stmt)?;
        }

        self.symbol_table.pop_scope();
        self.current_return_type = None;
        Ok(())
    }

    fn visit_statement(&mut self, stmt: &Statement) -> Result<(), String> {
        match &stmt.stmt_kind {
            Some(statement::StmtKind::Declaration(decl)) => {
                self.visit_declaration(decl)
            }
            Some(statement::StmtKind::Assignment(assign)) => {
                self.visit_assignment(assign)
            }
            Some(statement::StmtKind::IfStmt(if_stmt)) => {
                self.visit_if(if_stmt)
            }
            Some(statement::StmtKind::WhileStmt(while_stmt)) => {
                self.visit_while(while_stmt)
            }
            Some(statement::StmtKind::ReturnStmt(ret)) => {
                self.visit_return(ret)
            }
            Some(statement::StmtKind::ExpressionStmt(expr)) => {
                self.resolve_expression(expr)?;
                Ok(())
            }
            None => Ok(()),
        }
    }

    fn visit_declaration(&mut self, decl: &VariableDeclaration) -> Result<(), String> {
        let declared_kind = decl.r#type.as_ref().map_or(5, |t| t.kind);
        let declared_type = NeuroType::from_proto_kind(declared_kind);

        if let Some(initializer) = &decl.initializer {
            let init_type = self.resolve_expression(initializer)?;
            check_type_match(&declared_type, &init_type,
                &format!("Variable `{}`", decl.name))?;
            self.symbol_table.insert(&decl.name, declared_type, false);
            self.symbol_table.mark_initialized(&decl.name)
                .map_err(|e| format!("Variable `{}`: {}", decl.name, e))?;
        } else {
            self.symbol_table.insert(&decl.name, declared_type, false);
        }

        Ok(())
    }

    fn visit_assignment(&mut self, assign: &shared_ast::Assignment) -> Result<(), String> {
        let target_type = self.symbol_table.lookup(&assign.target_name)
            .ok_or_else(|| format!("Undefined variable `{}`", assign.target_name))
            .map(|s| {
                if !s.is_mutable {
                    return Err(format!("Cannot assign to immutable variable `{}`", assign.target_name));
                }
                Ok(s.type_.clone())
            })?;

        let target_type = target_type?;

        if let Some(value) = &assign.value {
            let value_type = self.resolve_expression(value)?;
            check_type_match(&target_type, &value_type,
                &format!("Assignment to `{}`", assign.target_name))?;
        }

        Ok(())
    }

    fn visit_if(&mut self, if_stmt: &shared_ast::IfStatement) -> Result<(), String> {
        if let Some(condition) = &if_stmt.condition {
            let cond_type = self.resolve_expression(condition)?;
            if cond_type != NeuroType::Bool {
                return Err("If condition must be a boolean expression".to_string());
            }
        }

        self.symbol_table.push_scope();
        for stmt in &if_stmt.true_branch {
            self.visit_statement(stmt)?;
        }
        self.symbol_table.pop_scope();

        self.symbol_table.push_scope();
        for stmt in &if_stmt.false_branch {
            self.visit_statement(stmt)?;
        }
        self.symbol_table.pop_scope();

        Ok(())
    }

    fn visit_while(&mut self, while_stmt: &shared_ast::WhileStatement) -> Result<(), String> {
        if let Some(condition) = &while_stmt.condition {
            let cond_type = self.resolve_expression(condition)?;
            if cond_type != NeuroType::Bool {
                return Err("While condition must be a boolean expression".to_string());
            }
        }

        self.symbol_table.push_scope();
        for stmt in &while_stmt.body {
            self.visit_statement(stmt)?;
        }
        self.symbol_table.pop_scope();

        Ok(())
    }

    fn visit_return(&mut self, ret: &shared_ast::ReturnStatement) -> Result<(), String> {
        let expected = self.current_return_type.clone();
        match (&ret.value, expected) {
            (None, Some(expected)) if expected != NeuroType::Void => {
                Err(format!("Function expects return type `{:?}`, but no value was returned", expected))
            }
            (None, _) => Ok(()),
            (Some(expr), None) => {
                let expr_type = self.resolve_expression(expr)?;
                Err(format!("Function has no return type annotation, but returned value of type `{:?}`", expr_type))
            }
            (Some(expr), Some(expected)) => {
                let expr_type = self.resolve_expression(expr)?;
                check_type_match(&expected, &expr_type, "Return value")?;
                Ok(())
            }
        }
    }

    fn resolve_expression(&mut self, expr: &Expression) -> Result<NeuroType, String> {
        match &expr.expr_kind {
            Some(expression::ExprKind::Literal(lit)) => Ok(self.resolve_literal(lit)),
            Some(expression::ExprKind::Variable(var)) => {
                let symbol = self.symbol_table.lookup(&var.name)
                    .ok_or_else(|| format!("Undefined variable `{}`", var.name))?;
                if !symbol.is_initialized {
                    return Err(format!("Variable `{}` may not be initialized", var.name));
                }
                Ok(symbol.type_.clone())
            }
            Some(expression::ExprKind::BinaryOp(bin_op)) => {
                self.resolve_binary_op(bin_op)
            }
            Some(expression::ExprKind::UnaryOp(un_op)) => {
                self.resolve_unary_op(un_op)
            }
            Some(expression::ExprKind::Call(call)) => {
                self.resolve_function_call(call)
            }
            None => Err("Empty expression".to_string()),
        }
    }

    fn resolve_literal(&self, lit: &shared_ast::Literal) -> NeuroType {
        match &lit.value {
            Some(literal::Value::IntVal(_)) => NeuroType::Int,
            Some(literal::Value::FloatVal(_)) => NeuroType::Float,
            Some(literal::Value::BoolVal(_)) => NeuroType::Bool,
            Some(literal::Value::StringVal(_)) => NeuroType::String,
            None => NeuroType::Void,
        }
    }

    fn resolve_binary_op(&mut self, bin_op: &shared_ast::BinaryOperation) -> Result<NeuroType, String> {
        let left = bin_op.left.as_deref()
            .ok_or_else(|| "Binary operation missing left operand".to_string())?;
        let right = bin_op.right.as_deref()
            .ok_or_else(|| "Binary operation missing right operand".to_string())?;

        let left_type = self.resolve_expression(left)?;
        let right_type = self.resolve_expression(right)?;

        match bin_op.op {
            0 | 1 | 2 | 3 => {
                check_type_match(&left_type, &right_type, "Binary arithmetic operation")?;
                match left_type {
                    NeuroType::Int | NeuroType::Float => Ok(left_type),
                    _ => Err(format!(
                        "Arithmetic operator requires Int or Float operands, got `{:?}` and `{:?}`",
                        left_type, right_type
                    )),
                }
            }
            4 | 5 => {
                check_type_match(&left_type, &right_type, "Binary equality comparison")?;
                Ok(NeuroType::Bool)
            }
            6 | 7 | 8 | 9 => {
                check_type_match(&left_type, &right_type, "Binary ordering comparison")?;
                match left_type {
                    NeuroType::Int | NeuroType::Float => Ok(NeuroType::Bool),
                    _ => Err(format!(
                        "Ordering operator requires Int or Float operands, got `{:?}`",
                        left_type
                    )),
                }
            }
            10 | 11 => {
                if left_type != NeuroType::Bool {
                    return Err(format!(
                        "Logical operator requires Bool operands, got `{:?}`",
                        left_type
                    ));
                }
                if right_type != NeuroType::Bool {
                    return Err(format!(
                        "Logical operator requires Bool operands, got `{:?}`",
                        right_type
                    ));
                }
                Ok(NeuroType::Bool)
            }
            _ => Err(format!("Unknown binary operator: {}", bin_op.op)),
        }
    }

    fn resolve_unary_op(&mut self, un_op: &shared_ast::UnaryOperation) -> Result<NeuroType, String> {
        let operand = un_op.operand.as_deref()
            .ok_or_else(|| "Unary operation missing operand".to_string())?;
        let operand_type = self.resolve_expression(operand)?;

        match un_op.op {
            0 => {
                match operand_type {
                    NeuroType::Int | NeuroType::Float => Ok(operand_type),
                    _ => Err(format!(
                        "Negation operator requires Int or Float operand, got `{:?}`",
                        operand_type
                    )),
                }
            }
            1 => {
                if operand_type != NeuroType::Bool {
                    return Err(format!(
                        "Logical NOT requires Bool operand, got `{:?}`",
                        operand_type
                    ));
                }
                Ok(NeuroType::Bool)
            }
            _ => Err(format!("Unknown unary operator: {}", un_op.op)),
        }
    }

    fn resolve_function_call(&mut self, call: &shared_ast::FunctionCall) -> Result<NeuroType, String> {
        for arg in &call.arguments {
            self.resolve_expression(arg)?;
        }
        Ok(NeuroType::Int)
    }
}

fn check_type_match(expected: &NeuroType, actual: &NeuroType, context: &str) -> Result<(), String> {
    if expected != actual {
        return Err(format!(
            "{}: type mismatch — expected `{:?}`, got `{:?}`",
            context, expected, actual
        ));
    }
    Ok(())
}
