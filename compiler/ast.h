#ifndef AST_H
#define AST_H

// The different types of grammatical structures in our code
typedef enum {
    NODE_PROGRAM,
    NODE_STMTS,
    NODE_VAR,
    NODE_NUM,
    NODE_ASSIGN,
    NODE_MULTIPLY,
    NODE_IF,
    NODE_SCANF,
    NODE_PRINTF
} NodeType;

// The Tree Node Structure
typedef struct ASTNode {
    NodeType type;
    char* str_val;                 // Holds variable names or raw numbers ("income", "0.05")
    
    struct ASTNode* left;          // Used for sequences or left side of math
    struct ASTNode* right;         // Used for right side of math/assignments
    
    struct ASTNode* condition;     // For IF statements
    struct ASTNode* true_branch;   
    struct ASTNode* false_branch;  
} ASTNode;

// Function prototypes
ASTNode* make_var_node(char* name);
ASTNode* make_num_node(char* val);
ASTNode* make_math_node(NodeType type, ASTNode* left, ASTNode* right);
ASTNode* make_assign_node(char* var_name, ASTNode* expr);
ASTNode* make_if_node(ASTNode* cond_left, ASTNode* cond_right, ASTNode* true_b, ASTNode* false_b);
ASTNode* make_scanf_node(char* var_name);
ASTNode* make_printf_node(char* var_name);
ASTNode* make_stmts_node(ASTNode* left, ASTNode* right);

#endif