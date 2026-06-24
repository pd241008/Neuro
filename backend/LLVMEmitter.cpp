#include "NeuroBackend.h"
#include "build/ast.pb.h"
#include <iostream>
#include <cassert>

using namespace neuro::ast;

LLVMEmitter::LLVMEmitter(std::ostream& output) : out_(output) {}

void LLVMEmitter::emitLine(const std::string& line) {
    for (int i = 0; i < indent_; ++i) out_ << "  ";
    out_ << line << "\n";
}

std::string LLVMEmitter::newRegister() {
    return "%r" + std::to_string(regCounter_++);
}

std::string LLVMEmitter::mangleName(const std::string& name) {
    return "@_" + name;
}

std::string LLVMEmitter::typeToLLVM(const Type& type) {
    return typeToLLVM(type.kind());
}

std::string LLVMEmitter::typeToLLVM(int kind) {
    switch (kind) {
        case Type_Kind_INT:    return "i32";
        case Type_Kind_FLOAT:  return "double";
        case Type_Kind_BOOL:   return "i1";
        case Type_Kind_STRING: return "i8*";
        case Type_Kind_VOID:   return "void";
        default:               return "i32";
    }
}

bool LLVMEmitter::emitProgram(const VerifiedProgram& verified) {
    if (!verified.has_program()) {
        error_ = "VerifiedProgram missing Program";
        return false;
    }

    const Program& prog = verified.program();

    out_ << "; NEURO Compiler — LLVM IR Output\n";
    out_ << "; Module: " << prog.name() << "\n";
    out_ << "target triple = \"x86_64-pc-linux-gnu\"\n";
    out_ << "\n";

    // Declare external I/O functions used by the runtime
    out_ << "declare void @neuro_print(i8*)\n";
    out_ << "declare void @neuro_println(i8*)\n";
    out_ << "declare i32 @neuro_read()\n";
    out_ << "\n";

    for (const auto& func : prog.functions()) {
        emitFunction(func);
        out_ << "\n";
    }

    return true;
}

void LLVMEmitter::emitFunction(const Function& func) {
    std::string retType = typeToLLVM(func.return_type());
    std::string mangled = mangleName(func.name());

    // Build parameter list
    std::vector<std::string> paramTypes;
    for (const auto& param : func.parameters()) {
        paramTypes.push_back(typeToLLVM(param.type()));
    }

    out_ << "define " << retType << " " << mangled << "(";
    for (size_t i = 0; i < paramTypes.size(); ++i) {
        if (i > 0) out_ << ", ";
        out_ << paramTypes[i];
    }
    out_ << ") {\n";

    indent_ = 1;
    regCounter_ = 0;

    // Entry block
    std::ostringstream entry;
    entry << "entry:\n";

    // Allocate space for parameters and store them
    for (size_t i = 0; i < func.parameters().size(); ++i) {
        const auto& param = func.parameters()[i];
        std::string llvmType = typeToLLVM(param.type());
        entry << "  %" << param.name() << "_ptr = alloca " << llvmType << "\n";
        entry << "  store " << llvmType << " %" << i << ", ptr %" << param.name() << "_ptr\n";
    }

    // Emit body statements
    std::ostringstream body;
    for (const auto& stmt : func.body()) {
        emitStatement(stmt, body);
    }

    // If function is void and no return was emitted, add implicit ret
    if (func.return_type().kind() == Type_Kind_VOID) {
        body << "  ret void\n";
    }

    out_ << entry.str() << body.str();

    indent_ = 0;
    out_ << "}\n";
}

void LLVMEmitter::emitStatement(const Statement& stmt, std::ostream& block) {
    switch (stmt.stmt_kind_case()) {
        case Statement::kDeclaration: {
            const auto& decl = stmt.declaration();
            std::string llvmType = typeToLLVM(decl.type());
            block << "  %" << decl.name() << " = alloca " << llvmType << "\n";
            if (decl.has_initializer()) {
                std::string reg = newRegister();
                emitExpression(decl.initializer(), block, reg);
                block << "  store " << llvmType << " " << reg << ", ptr %" << decl.name() << "\n";
            }
            break;
        }
        case Statement::kAssignment: {
            const auto& assign = stmt.assignment();
            std::string reg = newRegister();
            emitExpression(assign.value(), block, reg);
            // We need the type from the variable; look up via the enriched AST
            // For now use the expression's resolved type
            if (assign.value().has_resolved_type()) {
                std::string llvmType = typeToLLVM(assign.value().resolved_type());
                block << "  store " << llvmType << " " << reg << ", ptr %" << assign.target_name() << "\n";
            } else {
                block << "  store " << reg << ", ptr %" << assign.target_name() << "\n";
            }
            break;
        }
        case Statement::kReturnStmt: {
            const auto& ret = stmt.return_stmt();
            if (ret.has_value()) {
                std::string reg = newRegister();
                emitExpression(ret.value(), block, reg);
                block << "  ret " << typeToLLVM(ret.value().resolved_type()) << " " << reg << "\n";
            } else {
                block << "  ret void\n";
            }
            break;
        }
        case Statement::kIfStmt: {
            const auto& ifStmt = stmt.if_stmt();
            std::string condReg = newRegister();
            emitExpression(ifStmt.condition(), block, condReg);

            std::string trueLabel = "if_true_" + std::to_string(regCounter_++);
            std::string falseLabel = "if_false_" + std::to_string(regCounter_++);
            std::string mergeLabel = "if_merge_" + std::to_string(regCounter_++);

            block << "  br i1 " << condReg << ", label %" << trueLabel << ", label %" << falseLabel << "\n";

            block << trueLabel << ":\n";
            for (const auto& s : ifStmt.true_branch()) {
                emitStatement(s, block);
            }
            block << "  br label %" << mergeLabel << "\n";

            block << falseLabel << ":\n";
            for (const auto& s : ifStmt.false_branch()) {
                emitStatement(s, block);
            }
            block << "  br label %" << mergeLabel << "\n";

            block << mergeLabel << ":\n";
            break;
        }
        case Statement::kWhileStmt: {
            const auto& whileStmt = stmt.while_stmt();
            std::string condLabel = "while_cond_" + std::to_string(regCounter_++);
            std::string bodyLabel = "while_body_" + std::to_string(regCounter_++);
            std::string endLabel = "while_end_" + std::to_string(regCounter_++);

            block << "  br label %" << condLabel << "\n";
            block << condLabel << ":\n";
            std::string condReg = newRegister();
            emitExpression(whileStmt.condition(), block, condReg);
            block << "  br i1 " << condReg << ", label %" << bodyLabel << ", label %" << endLabel << "\n";

            block << bodyLabel << ":\n";
            for (const auto& s : whileStmt.body()) {
                emitStatement(s, block);
            }
            block << "  br label %" << condLabel << "\n";

            block << endLabel << ":\n";
            break;
        }
        case Statement::kExpressionStmt: {
            std::string reg = newRegister();
            emitExpression(stmt.expression_stmt(), block, reg);
            break;
        }
        default:
            break;
    }
}

void LLVMEmitter::emitExpression(const Expression& expr, std::ostream& block, const std::string& resultReg) {
    switch (expr.expr_kind_case()) {
        case Expression::kLiteral: {
            const auto& lit = expr.literal();
            std::string llvmType = expr.has_resolved_type() ? typeToLLVM(expr.resolved_type()) : "i32";
            switch (lit.value_case()) {
                case Literal::kIntVal:
                    block << "  " << resultReg << " = add " << llvmType << " 0, " << lit.int_val() << "\n";
                    break;
                case Literal::kFloatVal:
                    block << "  " << resultReg << " = fadd " << llvmType << " 0.0, " << lit.float_val() << "\n";
                    break;
                case Literal::kBoolVal:
                    block << "  " << resultReg << " = add i1 0, " << (lit.bool_val() ? "1" : "0") << "\n";
                    break;
                case Literal::kStringVal:
                    block << "  " << resultReg << " = getelementptr inbounds (["
                          << lit.string_val().size() + 1 << " x i8], ["
                          << lit.string_val().size() + 1 << " x i8]* @__str_" << resultReg.substr(1)
                          << ", i64 0, i64 0)\n";
                    break;
                default:
                    block << "  " << resultReg << " = add i32 0, 0\n";
                    break;
            }
            break;
        }
        case Expression::kVariable: {
            const auto& var = expr.variable();
            std::string llvmType = expr.has_resolved_type() ? typeToLLVM(expr.resolved_type()) : "i32";
            block << "  " << resultReg << " = load " << llvmType << ", ptr %" << var.name() << "\n";
            break;
        }
        case Expression::kBinaryOp: {
            const auto& binOp = expr.binary_op();
            std::string leftReg = newRegister();
            std::string rightReg = newRegister();
            emitExpression(binOp.left(), block, leftReg);
            emitExpression(binOp.right(), block, rightReg);

            std::string llvmType = expr.has_resolved_type() ? typeToLLVM(expr.resolved_type()) : "i32";
            bool isFloat = (expr.has_resolved_type() && expr.resolved_type().kind() == Type_Kind_FLOAT);
            bool isBool = (expr.has_resolved_type() && expr.resolved_type().kind() == Type_Kind_BOOL);

            switch (binOp.op()) {
                case BinaryOperation_Operator_ADD:
                    if (isFloat)
                        block << "  " << resultReg << " = fadd " << llvmType << " " << leftReg << ", " << rightReg << "\n";
                    else
                        block << "  " << resultReg << " = add " << llvmType << " " << leftReg << ", " << rightReg << "\n";
                    break;
                case BinaryOperation_Operator_SUB:
                    if (isFloat)
                        block << "  " << resultReg << " = fsub " << llvmType << " " << leftReg << ", " << rightReg << "\n";
                    else
                        block << "  " << resultReg << " = sub " << llvmType << " " << leftReg << ", " << rightReg << "\n";
                    break;
                case BinaryOperation_Operator_MUL:
                    if (isFloat)
                        block << "  " << resultReg << " = fmul " << llvmType << " " << leftReg << ", " << rightReg << "\n";
                    else
                        block << "  " << resultReg << " = mul " << llvmType << " " << leftReg << ", " << rightReg << "\n";
                    break;
                case BinaryOperation_Operator_DIV:
                    if (isFloat)
                        block << "  " << resultReg << " = fdiv " << llvmType << " " << leftReg << ", " << rightReg << "\n";
                    else
                        block << "  " << resultReg << " = sdiv " << llvmType << " " << leftReg << ", " << rightReg << "\n";
                    break;
                case BinaryOperation_Operator_EQ:
                    if (isFloat)
                        block << "  " << resultReg << " = fcmp oeq " << llvmType << " " << leftReg << ", " << rightReg << "\n";
                    else
                        block << "  " << resultReg << " = icmp eq " << llvmType << " " << leftReg << ", " << rightReg << "\n";
                    break;
                case BinaryOperation_Operator_NEQ:
                    if (isFloat)
                        block << "  " << resultReg << " = fcmp one " << llvmType << " " << leftReg << ", " << rightReg << "\n";
                    else
                        block << "  " << resultReg << " = icmp ne " << llvmType << " " << leftReg << ", " << rightReg << "\n";
                    break;
                case BinaryOperation_Operator_LT:
                    if (isFloat)
                        block << "  " << resultReg << " = fcmp olt " << llvmType << " " << leftReg << ", " << rightReg << "\n";
                    else
                        block << "  " << resultReg << " = icmp slt " << llvmType << " " << leftReg << ", " << rightReg << "\n";
                    break;
                case BinaryOperation_Operator_GT:
                    if (isFloat)
                        block << "  " << resultReg << " = fcmp ogt " << llvmType << " " << leftReg << ", " << rightReg << "\n";
                    else
                        block << "  " << resultReg << " = icmp sgt " << llvmType << " " << leftReg << ", " << rightReg << "\n";
                    break;
                case BinaryOperation_Operator_LTE:
                    if (isFloat)
                        block << "  " << resultReg << " = fcmp ole " << llvmType << " " << leftReg << ", " << rightReg << "\n";
                    else
                        block << "  " << resultReg << " = icmp sle " << llvmType << " " << leftReg << ", " << rightReg << "\n";
                    break;
                case BinaryOperation_Operator_GTE:
                    if (isFloat)
                        block << "  " << resultReg << " = fcmp oge " << llvmType << " " << leftReg << ", " << rightReg << "\n";
                    else
                        block << "  " << resultReg << " = icmp sge " << llvmType << " " << leftReg << ", " << rightReg << "\n";
                    break;
                case BinaryOperation_Operator_AND:
                    block << "  " << resultReg << " = and i1 " << leftReg << ", " << rightReg << "\n";
                    break;
                case BinaryOperation_Operator_OR:
                    block << "  " << resultReg << " = or i1 " << leftReg << ", " << rightReg << "\n";
                    break;
                default:
                    block << "  " << resultReg << " = add i32 0, 0\n";
                    break;
            }
            break;
        }
        case Expression::kUnaryOp: {
            const auto& unOp = expr.unary_op();
            std::string operandReg = newRegister();
            emitExpression(unOp.operand(), block, operandReg);

            std::string llvmType = expr.has_resolved_type() ? typeToLLVM(expr.resolved_type()) : "i32";
            bool isFloat = (expr.has_resolved_type() && expr.resolved_type().kind() == Type_Kind_FLOAT);

            switch (unOp.op()) {
                case UnaryOperation_Operator_NEG:
                    if (isFloat)
                        block << "  " << resultReg << " = fneg " << llvmType << " " << operandReg << "\n";
                    else
                        block << "  " << resultReg << " = sub " << llvmType << " 0, " << operandReg << "\n";
                    break;
                case UnaryOperation_Operator_NOT:
                    block << "  " << resultReg << " = xor i1 1, " << operandReg << "\n";
                    break;
                default:
                    block << "  " << resultReg << " = add i32 0, 0\n";
                    break;
            }
            break;
        }
        case Expression::kCall: {
            const auto& call = expr.call();
            std::string mangled = mangleName(call.function_name());

            // Build argument list
            block << "  " << resultReg << " = call " << typeToLLVM(expr.resolved_type()) << " " << mangled << "(";
            for (int i = 0; i < call.arguments_size(); ++i) {
                if (i > 0) block << ", ";
                std::string argReg = newRegister();
                emitExpression(call.arguments(i), block, argReg);
                // Need type for each argument - use the resolved type
                if (call.arguments(i).has_resolved_type()) {
                    block << typeToLLVM(call.arguments(i).resolved_type()) << " " << argReg;
                } else {
                    block << "i32 " << argReg;
                }
            }
            block << ")\n";
            break;
        }
        default:
            block << "  " << resultReg << " = add i32 0, 0\n";
            break;
    }
}
