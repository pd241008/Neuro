#ifndef NEURO_BACKEND_H
#define NEURO_BACKEND_H

#include <string>
#include <vector>
#include <memory>
#include <sstream>
#include "build/ast.pb.h"

class LLVMEmitter {
public:
    explicit LLVMEmitter(std::ostream& output);

    bool emitProgram(const neuro::ast::VerifiedProgram& verified);
    std::string getError() const { return error_; }

private:
    std::ostream& out_;
    std::string error_;
    int indent_{0};

    void emitLine(const std::string& line);
    void emitFunction(const neuro::ast::Function& func);
    void emitStatement(const neuro::ast::Statement& stmt, std::ostream& block);
    void emitExpression(const neuro::ast::Expression& expr, std::ostream& block, const std::string& resultReg);

    std::string typeToLLVM(const neuro::ast::Type& type);
    std::string typeToLLVM(int kind);
    std::string mangleName(const std::string& name);
    std::string newRegister();
    std::string emitGlobalString(const std::string& content, size_t& outLen);

    int regCounter_{0};
    int globalStringCounter_{0};
    std::vector<std::string> globalStrings_;
};

#endif
