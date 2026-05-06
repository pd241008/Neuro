%{
#include "y.tab.h"
#include <string.h>
%}

%%
"int"       { return INT; }
"float"     { return FLOAT; }
"if"        { return IF; }
"else"      { return ELSE; }
"return"    { return RETURN; }
"printf"    { return PRINTF; }
"scanf"     { return SCANF; }
"<="        { return LEQ; }

[a-zA-Z_][a-zA-Z0-9_]* { yylval.str = strdup(yytext); return ID; }
[0-9]+(\.[0-9]+)?      { yylval.str = strdup(yytext); return NUM; }
\"[^\"]*\"             { yylval.str = strdup(yytext); return STRING; }

[ \t\n\r]+             { /* Ignore whitespace */ }
.                      { return yytext[0]; }
%%

int yywrap() { return 1; }