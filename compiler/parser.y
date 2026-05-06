%{
#include <stdio.h>
#include <stdlib.h>
#include "ast.h"

extern int yylex();
void yyerror(const char *s);

ASTNode* root_node;
%}

%union { 
    char* str; 
    struct ASTNode* node; 
}

%token <str> ID NUM STRING
%token INT FLOAT IF ELSE RETURN PRINTF SCANF LEQ

%type <node> stmts stmt expr

%%
program:
    INT ID '(' ')' '{' decls stmts '}' { 
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
    | ID '=' expr ';' {
        $$ = make_assign_node($1, $3);
    }
    | IF '(' expr LEQ expr ')' stmt {
        $$ = make_if_node($3, $5, $7, NULL);
    }
    | IF '(' expr LEQ expr ')' stmt ELSE stmt {
        $$ = make_if_node($3, $5, $7, $9);
    }
    | '{' stmts '}' {
        $$ = $2;
    }
    | RETURN expr ';' {
        $$ = NULL; 
    }
    ;

expr:
    ID { $$ = make_var_node($1); }
    | NUM { $$ = make_num_node($1); }
    | expr '*' expr { $$ = make_math_node(NODE_MULTIPLY, $1, $3); }
    ;
%%

void yyerror(const char *s) { 
    fprintf(stderr, "Syntax Error: %s\n", s); 
}