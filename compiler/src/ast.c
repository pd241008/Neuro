#include <stdlib.h>
#include <string.h>
#include "ast.h"

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
