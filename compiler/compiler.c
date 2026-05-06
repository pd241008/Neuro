#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include "ast.h"

// External references to the parser
extern int yyparse();
extern ASTNode* root_node;
extern FILE* yyin;

int label_counter = 0;

// --- 1. AST Construction Functions ---
ASTNode* create_node(NodeType type) {
    ASTNode* node = (ASTNode*)malloc(sizeof(ASTNode));
    node->type = type;
    node->str_val = NULL;
    node->left = node->right = node->condition = node->true_branch = node->false_branch = NULL;
    return node;
}
ASTNode* make_var_node(char* name) { ASTNode* n = create_node(NODE_VAR); n->str_val = strdup(name); return n; }
ASTNode* make_num_node(char* val) { ASTNode* n = create_node(NODE_NUM); n->str_val = strdup(val); return n; }
ASTNode* make_math_node(NodeType type, ASTNode* left, ASTNode* right) { ASTNode* n = create_node(type); n->left = left; n->right = right; return n; }
ASTNode* make_assign_node(char* var_name, ASTNode* expr) { ASTNode* n = create_node(NODE_ASSIGN); n->str_val = strdup(var_name); n->right = expr; return n; }
ASTNode* make_if_node(ASTNode* cond_l, ASTNode* cond_r, ASTNode* t_branch, ASTNode* f_branch) {
    ASTNode* n = create_node(NODE_IF); n->left = cond_l; n->right = cond_r; n->true_branch = t_branch; n->false_branch = f_branch; return n;
}
ASTNode* make_scanf_node(char* var_name) { ASTNode* n = create_node(NODE_SCANF); n->str_val = strdup(var_name); return n; }
ASTNode* make_printf_node(char* var_name) { ASTNode* n = create_node(NODE_PRINTF); n->str_val = strdup(var_name); return n; }
ASTNode* make_stmts_node(ASTNode* left, ASTNode* right) { ASTNode* n = create_node(NODE_STMTS); n->left = left; n->right = right; return n; }

// --- 2. The Code Generator (Backend) ---
void generate_assembly(ASTNode* node, FILE* out) {
    if (!node) return;

    switch(node->type) {
        case NODE_STMTS:
            generate_assembly(node->left, out);
            generate_assembly(node->right, out);
            break;

        case NODE_SCANF:
            fprintf(out, "  ; scanf(\"%%f\", &%s);\n  call scanf\n\n", node->str_val);
            break;

        case NODE_PRINTF:
            fprintf(out, "  ; printf(\"Tax = %%f\", %s);\n", node->str_val);
            fprintf(out, "  movss xmm0, DWORD PTR [rbp-8] ; Assuming tax is rbp-8\n");
            fprintf(out, "  call printf\n\n");
            break;

        case NODE_ASSIGN:
            fprintf(out, "  ; %s =\n", node->str_val);
            if(node->right->type == NODE_NUM && strcmp(node->right->str_val, "0") == 0) {
                // The Optimization!
                fprintf(out, "  pxor xmm0, xmm0\n");
            } else {
                generate_assembly(node->right, out); // Math result ends up in xmm0
            }
            // Hardcoding memory addresses for simplicity in this demo (income: rbp-4, tax: rbp-8)
            char* addr = (strcmp(node->str_val, "tax") == 0) ? "[rbp-8]" : "[rbp-4]";
            fprintf(out, "  movss DWORD PTR %s, xmm0\n\n", addr);
            break;

        case NODE_MULTIPLY:
            // e.g. income * 0.05
            fprintf(out, "  movss xmm0, DWORD PTR [rbp-4] ; Load income\n");
            fprintf(out, "  mulss xmm0, DWORD PTR .LC_CONST_%s[rip]\n", node->right->str_val);
            break;

        case NODE_IF: {
            label_counter++;
            int current_lbl = label_counter;
            int end_lbl = current_lbl + 1;
            label_counter++; // Reserve label for the END jump

            fprintf(out, "  ; if(%s <= %s)\n", node->left->str_val, node->right->str_val);
            fprintf(out, "  movss xmm0, DWORD PTR [rbp-4]\n");
            fprintf(out, "  ucomiss xmm0, DWORD PTR .LC_CONST_%s[rip]\n", node->right->str_val);
            
            // If condition fails (income > val), jump to FALSE branch
            fprintf(out, "  ja .L_FALSE_%d\n", current_lbl); 
            
            // TRUE Branch
            generate_assembly(node->true_branch, out);
            fprintf(out, "  jmp .L_END_%d\n\n", end_lbl);
            
            // FALSE Branch (else / else-if)
            fprintf(out, ".L_FALSE_%d:\n", current_lbl);
            if (node->false_branch) generate_assembly(node->false_branch, out);
            
            fprintf(out, ".L_END_%d:\n", end_lbl);
            break;
        }
        default: break;
    }
}

// --- 3. The Main Compiler Driver ---
int main(int argc, char **argv) {
    if (argc < 2) { printf("Usage: ./my_compiler <source_file.c>\n"); return 1; }
    
    yyin = fopen(argv[1], "r");
    if (!yyin) { printf("Could not open file %s\n", argv[1]); return 1; }

    printf("1. Running Front-End Parser (Building AST)...\n");
    yyparse(); // Builds the tree and stores it in root_node
    fclose(yyin);

    if (!root_node) { printf("Error: Failed to build AST.\n"); return 1; }
    
    printf("2. AST Built Successfully. Running Back-End Code Generator...\n");
    FILE* asm_out = fopen("tax_compiled.s", "w");
    
    fprintf(asm_out, "; --- Generated Assembly from AST ---\n");
    fprintf(asm_out, "main:\n");
    fprintf(asm_out, "  push rbp\n");
    fprintf(asm_out, "  mov rbp, rsp\n");
    fprintf(asm_out, "  sub rsp, 16\n\n");
    
    // Traverse the tree to generate logic
    generate_assembly(root_node, asm_out);
    
    fprintf(asm_out, "  ; return 0;\n");
    fprintf(asm_out, "  mov eax, 0\n");
    fprintf(asm_out, "  leave\n");
    fprintf(asm_out, "  ret\n");
    fclose(asm_out);

    printf("3. Compilation Complete! Assembly written to 'tax_compiled.s'\n");
    return 0;
}