%{
#include <stdio.h>
%}

%%
"int"|"float"|"if"|"else"|"return"  { printf("<KEYWORD, %s>\n", yytext); }
"printf"|"scanf"                    { printf("<BUILTIN, %s>\n", yytext); }
"<="                                { printf("<OP, LEQ>\n"); }
[a-zA-Z_][a-zA-Z0-9_]* { printf("<ID, %s>\n", yytext); }
[0-9]+(\.[0-9]+)?                   { printf("<NUM, %s>\n", yytext); }
\"[^\"]*\"                          { printf("<STRING, %s>\n", yytext); }
[ \t\n\r]+                          { /* Ignore whitespace */ }
.                                   { printf("<SYMBOL, %c>\n", yytext[0]); }
%%

int yywrap() { return 1; }

int main() {
    printf("--- RUNNING STANDALONE LEXICAL ANALYZER ---\n");
    yylex();
    return 0;
}