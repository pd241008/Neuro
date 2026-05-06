#include <stdio.h>
#include <stdlib.h>
#include "ast.h"
#include "codegen.h"

extern int yyparse();
extern ASTNode* root_node;
extern FILE* yyin;

int main(int argc, char **argv) {
    if (argc < 2) { 
        fprintf(stderr, "Usage: %s <source_file> [output_file]\n", argv[0]); 
        return 1; 
    }
    
    char* input_path = argv[1];
    char* output_path = (argc > 2) ? argv[2] : "output.s";

    yyin = fopen(input_path, "r");
    if (!yyin) { 
        perror("Could not open input file"); 
        return 1; 
    }

    printf("[1] Parsing: %s\n", input_path);
    if (yyparse() != 0 || !root_node) {
        fprintf(stderr, "Error: Parsing failed.\n");
        return 1;
    }
    fclose(yyin);

    printf("[2] Generating Code: %s\n", output_path);
    FILE* asm_out = fopen(output_path, "w");
    if (!asm_out) {
        perror("Could not open output file");
        return 1;
    }
    
    // Header
    fprintf(asm_out, ".intel_syntax noprefix\n");
    fprintf(asm_out, ".section .rodata\n");
    fprintf(asm_out, ".LC_SCAN_FMT:\n  .string \"%%f\"\n");
    fprintf(asm_out, ".LC_PRINT_FMT:\n  .string \"Result = %%f\\n\"\n");
    
    fprintf(asm_out, ".section .text\n");
    fprintf(asm_out, ".globl main\n");
    fprintf(asm_out, "main:\n");
    fprintf(asm_out, "  push rbp\n");
    fprintf(asm_out, "  mov rbp, rsp\n");
    fprintf(asm_out, "  sub rsp, 64 ; Static frame size for demo\n\n");
    
    generate_assembly(root_node, asm_out);
    
    fprintf(asm_out, "  mov eax, 0\n");
    fprintf(asm_out, "  leave\n");
    fprintf(asm_out, "  ret\n");
    fclose(asm_out);

    printf("[3] Complete.\n");
    return 0;
}
