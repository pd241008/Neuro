#ifndef SYMBOL_TABLE_H
#define SYMBOL_TABLE_H

typedef struct {
    char* name;
    int offset;
} Symbol;

int get_symbol_offset(char* name);

#endif
