#include <stdio.h>
#include <string.h>
#include "codegen.h"
#include "symbol_table.h"

int label_counter = 0;

void generate_assembly(ASTNode* node, FILE* out) {
    if (!node) return;

    switch(node->type) {
        case NODE_STMTS:
            generate_assembly(node->left, out);
            generate_assembly(node->right, out);
            break;

        case NODE_SCANF:
            fprintf(out, "  ; input variable: %s\n", node->str_val);
            fprintf(out, "  lea rdi, .LC_SCAN_FMT[rip]\n");
            fprintf(out, "  lea rsi, [rbp%d]\n", get_symbol_offset(node->str_val));
            fprintf(out, "  mov eax, 0\n");
            fprintf(out, "  call scanf\n\n");
            break;

        case NODE_PRINTF:
            fprintf(out, "  ; output variable: %s\n", node->str_val);
            fprintf(out, "  lea rdi, .LC_PRINT_FMT[rip]\n");
            fprintf(out, "  cvtss2sd xmm0, DWORD PTR [rbp%d]\n", get_symbol_offset(node->str_val));
            fprintf(out, "  mov eax, 1\n");
            fprintf(out, "  call printf\n\n");
            break;

        case NODE_ASSIGN:
            fprintf(out, "  ; %s =\n", node->str_val);
            generate_assembly(node->right, out); // Result in xmm0
            fprintf(out, "  movss DWORD PTR [rbp%d], xmm0\n\n", get_symbol_offset(node->str_val));
            break;

        case NODE_VAR:
            fprintf(out, "  movss xmm0, DWORD PTR [rbp%d]\n", get_symbol_offset(node->str_val));
            break;

        case NODE_NUM:
            // Handled by parent or as constants
            break;

        case NODE_MULTIPLY:
            generate_assembly(node->left, out); // Left side in xmm0
            fprintf(out, "  mulss xmm0, DWORD PTR .LC_CONST_%s[rip]\n", node->right->str_val);
            break;

        case NODE_IF: {
            label_counter++;
            int current_lbl = label_counter;
            int end_lbl = current_lbl + 1;
            label_counter++;

            fprintf(out, "  ; if condition\n");
            generate_assembly(node->left, out); 
            fprintf(out, "  ucomiss xmm0, DWORD PTR .LC_CONST_%s[rip]\n", node->right->str_val);
            
            fprintf(out, "  ja .L_FALSE_%d\n", current_lbl); 
            
            generate_assembly(node->true_branch, out);
            fprintf(out, "  jmp .L_END_%d\n\n", end_lbl);
            
            fprintf(out, ".L_FALSE_%d:\n", current_lbl);
            if (node->false_branch) generate_assembly(node->false_branch, out);
            
            fprintf(out, ".L_END_%d:\n", end_lbl);
            break;
        }
        default: break;
    }
}
