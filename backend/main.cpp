#include "NeuroBackend.h"
#include "build/ast.pb.h"
#include <fstream>
#include <iostream>
#include <string>

int main(int argc, char** argv) {
    if (argc != 3) {
        std::cerr << "Usage: neuro_backend <input_verified.ast> <output.ll>\n";
        return 1;
    }

    std::string inputPath = argv[1];
    std::string outputPath = argv[2];

    // Read input file
    std::ifstream input(inputPath, std::ios::binary);
    if (!input) {
        std::cerr << "Error: Cannot open input file: " << inputPath << "\n";
        return 1;
    }

    // Parse the VerifiedProgram protobuf
    neuro::ast::VerifiedProgram verified;
    if (!verified.ParseFromIstream(&input)) {
        std::cerr << "Error: Failed to parse VerifiedProgram from: " << inputPath << "\n";
        return 1;
    }

    // Generate LLVM IR
    std::ofstream output(outputPath);
    if (!output) {
        std::cerr << "Error: Cannot open output file: " << outputPath << "\n";
        return 1;
    }

    LLVMEmitter emitter(output);
    if (!emitter.emitProgram(verified)) {
        std::cerr << "Error: Backend emission failed: " << emitter.getError() << "\n";
        return 1;
    }

    return 0;
}
