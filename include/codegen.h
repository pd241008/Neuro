#ifndef CODEGEN_H
#define CODEGEN_H

#include <stdio.h>
#include "ast.h"

void generate_assembly(ASTNode* node, FILE* out);

#endif
