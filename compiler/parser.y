%{
#include <stdio.h>
#include <stdlib.h>
#include "ast.h"

extern int yylex();
void yyerror(const char *s);

// This global pointer will hold the very top of our finished tree
ASTNode* root_node;
%}

/* We now tell Yacc that tokens can hold strings OR AST Nodes */
%union { 
    char* str; 
    struct ASTNode* node; 
}

%token <str> ID NUM STRING
%token INT FLOAT IF ELSE RETURN PRINTF SCANF LEQ

/* Define the return types of our grammatical rules */
%type <node> stmts stmt

%%
program:
    INT ID '(' ')' '{' decls stmts '}' { 
        // We reached the top of the program! Save the tree.
        root_node = $7; 
    }
    ;

decls:
    /* empty */
    | FLOAT id_list ';'
    ;

id_list:
    ID | id_list ',' ID
    ;

stmts:
    stmt { $$ = $1; }
    | stmts stmt { $$ = make_stmts_node($1, $2); }
    ;

stmt:
    SCANF '(' STRING ',' '&' ID ')' ';' {
        $$ = make_scanf_node($6);
    }
    | PRINTF '(' STRING ',' ID ')' ';' {
        $$ = make_printf_node($5);
    }
    | ID '=' NUM ';' {
        $$ = make_assign_node($1, make_num_node($3));
    }
    | ID '=' ID '*' NUM ';' {
        ASTNode* math = make_math_node(NODE_MULTIPLY, make_var_node($3), make_num_node($5));
        $$ = make_assign_node($1, math);
    }
    | IF '(' ID LEQ NUM ')' stmt {
        $$ = make_if_node(make_var_node($3), make_num_node($5), $7, NULL);
    }
    | IF '(' ID LEQ NUM ')' stmt ELSE stmt {
        $$ = make_if_node(make_var_node($3), make_num_node($5), $7, $9);
    }
    | '{' stmts '}' {
        $$ = $2;
    }
    | RETURN NUM ';' {
        $$ = NULL; // Ignored for simplicity in this demo
    }
    ;
%%

void yyerror(const char *s) { 
    fprintf(stderr, "Syntax Error: %s\n", s); 
}