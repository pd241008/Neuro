use shared_ast::{
    Program, Function, Statement, Expression, VariableDeclaration,
    statement, expression, literal,
    r#type::Kind,
    binary_operation,
    unary_operation,
};
use crate::symbol_table::{SymbolTable, NeuroType, FunctionSignature};
use crate::borrow_check::BorrowChecker;
use crate::error::NeuroError;

pub fn analyze_ast(program: &Program) -> Result<(), NeuroError> {
    let mut ctx = AnalysisContext::new();
    ctx.visit_program(program)
}

struct AnalysisContext {
    symbol_table: SymbolTable,
    borrow_checker: BorrowChecker,
    current_return_type: Option<NeuroType>,
}

impl AnalysisContext {
    fn new() -> Self {
        Self {
            symbol_table: SymbolTable::new(),
            borrow_checker: BorrowChecker::new(),
            current_return_type: None,
        }
    }

    fn visit_program(&mut self, program: &Program) -> Result<(), NeuroError> {
        for function in &program.functions {
            let mut parameters = Vec::new();
            for param in &function.parameters {
                let param_kind = param.r#type.as_ref().map_or(Kind::Custom as i32, |t| t.kind);
                parameters.push(NeuroType::from_proto_kind(param_kind));
            }
            let return_kind = function.return_type.as_ref().map_or(Kind::Void as i32, |t| t.kind);
            let return_type = NeuroType::from_proto_kind(return_kind);
            
            let sig = FunctionSignature { parameters, return_type };
            self.symbol_table.insert_function(&function.name, sig)
                .map_err(|e| NeuroError::analysis(e))?;
        }

        for function in &program.functions {
            self.visit_function(function)?;
        }
        Ok(())
    }

    fn visit_function(&mut self, function: &Function) -> Result<(), NeuroError> {
        self.symbol_table.push_scope();
        self.borrow_checker.push_scope();

        for param in &function.parameters {
            let param_kind = param.r#type.as_ref().map_or(Kind::Custom as i32, |t| t.kind);
            let param_type = NeuroType::from_proto_kind(param_kind);
            if self.symbol_table.lookup_current_scope(&param.name).is_some() {
                return Err(NeuroError::analysis(format!("Duplicate parameter `{}` in function `{}`", param.name, function.name)));
            }
            self.symbol_table.insert(&param.name, param_type, false);
            self.symbol_table.mark_initialized(&param.name)
                .map_err(|e| NeuroError::analysis(format!("In function `{}`: {}", function.name, e)))?;
            self.borrow_checker.declare_variable(param.name.clone());
        }

        let declared_return = function.return_type.as_ref()
            .map(|t| NeuroType::from_proto_kind(t.kind));
        self.current_return_type = declared_return;

        for stmt in &function.body {
            self.visit_statement(stmt)?;
        }

        self.borrow_checker.pop_scope();
        self.borrow_checker.expire_borrow();
        self.symbol_table.pop_scope()
            .map_err(|e| NeuroError::analysis(format!("In function `{}`: {}", function.name, e)))?;
        self.current_return_type = None;
        Ok(())
    }

    fn visit_statement(&mut self, stmt: &Statement) -> Result<(), NeuroError> {
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

    fn visit_declaration(&mut self, decl: &VariableDeclaration) -> Result<(), NeuroError> {
        let declared_kind = decl.r#type.as_ref().map_or(Kind::Custom as i32, |t| t.kind);
        let declared_type = NeuroType::from_proto_kind(declared_kind);

        if self.symbol_table.lookup_current_scope(&decl.name).is_some() {
            return Err(NeuroError::analysis(format!("Duplicate variable `{}` in the same scope", decl.name)));
        }

        if let Some(initializer) = &decl.initializer {
            let init_type = self.resolve_expression(initializer)?;
            check_type_match(&declared_type, &init_type,
                &format!("Variable `{}`", decl.name))?;
            self.symbol_table.insert(&decl.name, declared_type, decl.is_mutable);
            self.symbol_table.mark_initialized(&decl.name)
                .map_err(|e| NeuroError::analysis(format!("Variable `{}`: {}", decl.name, e)))?;
        } else {
            return Err(NeuroError::analysis(format!("Variable `{}` must be initialized", decl.name)));
        }

        self.borrow_checker.declare_variable(decl.name.clone());

        Ok(())
    }

    fn visit_assignment(&mut self, assign: &shared_ast::Assignment) -> Result<(), NeuroError> {
        {
            let symbol = self.symbol_table.lookup(&assign.target_name)
                .ok_or_else(|| NeuroError::analysis(format!("Undefined variable `{}`", assign.target_name)))?;

            if !symbol.is_mutable {
                return Err(NeuroError::analysis(format!("Cannot assign to immutable variable `{}`", assign.target_name)));
            }
        }

        self.borrow_checker.check_write(&assign.target_name)?;

        if let Some(value) = &assign.value {
            let value_type = self.resolve_expression(value)?;
            let target_type = self.symbol_table.lookup(&assign.target_name)
                .ok_or_else(|| NeuroError::analysis(format!("Undefined variable `{}`", assign.target_name)))?
                .type_.clone();
            check_type_match(&target_type, &value_type,
                &format!("Assignment to `{}`", assign.target_name))?;
        }

        self.symbol_table.mark_initialized(&assign.target_name)
            .map_err(|e| NeuroError::analysis(format!("Assignment: {}", e)))?;
        self.borrow_checker.set_valid(&assign.target_name);

        Ok(())
    }

    fn visit_if(&mut self, if_stmt: &shared_ast::IfStatement) -> Result<(), NeuroError> {
        if let Some(condition) = &if_stmt.condition {
            let cond_type = self.resolve_expression(condition)?;
            if cond_type != NeuroType::Bool {
                return Err(NeuroError::analysis("If condition must be a boolean expression"));
            }
        }

        self.symbol_table.push_scope();
        self.borrow_checker.push_scope();
        for stmt in &if_stmt.true_branch {
            self.visit_statement(stmt)?;
        }
        self.borrow_checker.pop_scope();
        self.symbol_table.pop_scope()?;

        self.symbol_table.push_scope();
        self.borrow_checker.push_scope();
        for stmt in &if_stmt.false_branch {
            self.visit_statement(stmt)?;
        }
        self.borrow_checker.pop_scope();
        self.symbol_table.pop_scope()?;

        Ok(())
    }

    fn visit_while(&mut self, while_stmt: &shared_ast::WhileStatement) -> Result<(), NeuroError> {
        if let Some(condition) = &while_stmt.condition {
            let cond_type = self.resolve_expression(condition)?;
            if cond_type != NeuroType::Bool {
                return Err(NeuroError::analysis("While condition must be a boolean expression"));
            }
        }

        self.symbol_table.push_scope();
        self.borrow_checker.push_scope();
        for stmt in &while_stmt.body {
            self.visit_statement(stmt)?;
        }
        self.borrow_checker.pop_scope();
        self.symbol_table.pop_scope()?;

        Ok(())
    }

    fn visit_return(&mut self, ret: &shared_ast::ReturnStatement) -> Result<(), NeuroError> {
        let expected = self.current_return_type.clone();
        match (&ret.value, expected) {
            (None, Some(expected)) if expected != NeuroType::Void => {
                Err(NeuroError::analysis(format!("Function expects return type `{:?}`, but no value was returned", expected)))
            }
            (None, _) => Ok(()),
            (Some(expr), None) => {
                let expr_type = self.resolve_expression(expr)?;
                Err(NeuroError::analysis(format!("Function has no return type annotation, but returned value of type `{:?}`", expr_type)))
            }
            (Some(expr), Some(expected)) => {
                let expr_type = self.resolve_expression(expr)?;
                check_type_match(&expected, &expr_type, "Return value")?;
                Ok(())
            }
        }
    }

    fn resolve_expression(&mut self, expr: &Expression) -> Result<NeuroType, NeuroError> {
        match &expr.expr_kind {
            Some(expression::ExprKind::Literal(lit)) => Ok(self.resolve_literal(lit)),
            Some(expression::ExprKind::Variable(var)) => {
                self.borrow_checker.check_read(&var.name)?;
                let symbol = self.symbol_table.lookup(&var.name)
                    .ok_or_else(|| NeuroError::analysis(format!("Undefined variable `{}`", var.name)))?;
                if !symbol.is_initialized {
                    return Err(NeuroError::analysis(format!("Variable `{}` may not be initialized", var.name)));
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
            None => Err(NeuroError::analysis("Empty expression")),
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

    fn resolve_binary_op(&mut self, bin_op: &shared_ast::BinaryOperation) -> Result<NeuroType, NeuroError> {
        let left = bin_op.left.as_deref()
            .ok_or_else(|| NeuroError::analysis("Binary operation missing left operand"))?;
        let right = bin_op.right.as_deref()
            .ok_or_else(|| NeuroError::analysis("Binary operation missing right operand"))?;

        let left_type = self.resolve_expression(left)?;
        let right_type = self.resolve_expression(right)?;

        let op = bin_op.op;
        use binary_operation::Operator;

        if op == Operator::Add as i32 || op == Operator::Sub as i32
            || op == Operator::Mul as i32 || op == Operator::Div as i32
        {
            check_type_match(&left_type, &right_type, "Binary arithmetic operation")?;
            match left_type {
                NeuroType::Int | NeuroType::Float => Ok(left_type),
                _ => Err(NeuroError::analysis(format!(
                    "Arithmetic operator requires Int or Float operands, got `{:?}` and `{:?}`",
                    left_type, right_type
                ))),
            }
        } else if op == Operator::Eq as i32 || op == Operator::Neq as i32 {
            check_type_match(&left_type, &right_type, "Binary equality comparison")?;
            Ok(NeuroType::Bool)
        } else if op == Operator::Lt as i32 || op == Operator::Gt as i32
            || op == Operator::Lte as i32 || op == Operator::Gte as i32
        {
            check_type_match(&left_type, &right_type, "Binary ordering comparison")?;
            match left_type {
                NeuroType::Int | NeuroType::Float => Ok(NeuroType::Bool),
                _ => Err(NeuroError::analysis(format!(
                    "Ordering operator requires Int or Float operands, got `{:?}`",
                    left_type
                ))),
            }
        } else if op == Operator::And as i32 || op == Operator::Or as i32 {
            if left_type != NeuroType::Bool {
                return Err(NeuroError::analysis(format!(
                    "Logical operator requires Bool operands, got `{:?}`",
                    left_type
                )));
            }
            if right_type != NeuroType::Bool {
                return Err(NeuroError::analysis(format!(
                    "Logical operator requires Bool operands, got `{:?}`",
                    right_type
                )));
            }
            Ok(NeuroType::Bool)
        } else {
            Err(NeuroError::analysis(format!("Unknown binary operator: {}", bin_op.op)))
        }
    }

    fn resolve_unary_op(&mut self, un_op: &shared_ast::UnaryOperation) -> Result<NeuroType, NeuroError> {
        let operand = un_op.operand.as_deref()
            .ok_or_else(|| NeuroError::analysis("Unary operation missing operand"))?;
        let operand_type = self.resolve_expression(operand)?;

        use unary_operation::Operator;

        if un_op.op == Operator::Neg as i32 {
            match operand_type {
                NeuroType::Int | NeuroType::Float => Ok(operand_type),
                _ => Err(NeuroError::analysis(format!(
                    "Negation operator requires Int or Float operand, got `{:?}`",
                    operand_type
                ))),
            }
        } else if un_op.op == Operator::Not as i32 {
            if operand_type != NeuroType::Bool {
                return Err(NeuroError::analysis(format!(
                    "Logical NOT requires Bool operand, got `{:?}`",
                    operand_type
                )));
            }
            Ok(NeuroType::Bool)
        } else {
            Err(NeuroError::analysis(format!("Unknown unary operator: {}", un_op.op)))
        }
    }

    fn resolve_function_call(&mut self, call: &shared_ast::FunctionCall) -> Result<NeuroType, NeuroError> {
        let sig = self.symbol_table.lookup_function(&call.function_name)
            .ok_or_else(|| NeuroError::analysis(format!("Undefined function `{}`", call.function_name)))?
            .clone();

        if sig.parameters.len() != call.arguments.len() {
            return Err(NeuroError::analysis(format!(
                "Function `{}` expects {} arguments, but got {}",
                call.function_name, sig.parameters.len(), call.arguments.len()
            )));
        }

        for (i, arg) in call.arguments.iter().enumerate() {
            let arg_type = self.resolve_expression(arg)?;
            check_type_match(&sig.parameters[i], &arg_type,
                &format!("Argument {} of `{}`", i + 1, call.function_name))?;

            if let Some(expression::ExprKind::Variable(var)) = &arg.expr_kind {
                self.borrow_checker.move_variable(&var.name)?;
            }
        }
        Ok(sig.return_type)
    }
}

fn check_type_match(expected: &NeuroType, actual: &NeuroType, context: &str) -> Result<(), NeuroError> {
    if expected != actual {
        return Err(NeuroError::analysis(format!(
            "{}: type mismatch — expected `{:?}`, got `{:?}`",
            context, expected, actual
        )));
    }
    Ok(())
}
