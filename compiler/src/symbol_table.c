#include <string.h>
#include <stdlib.h>
#include "symbol_table.h"

Symbol symbol_table[100];
int symbol_count = 0;
int current_stack_offset = -4;

int get_symbol_offset(char* name) {
    for (int i = 0; i < symbol_count; i++) {
        if (strcmp(symbol_table[i].name, name) == 0) {
            return symbol_table[i].offset;
        }
    }
    // Create new symbol if not found
    symbol_table[symbol_count].name = strdup(name);
    symbol_table[symbol_count].offset = current_stack_offset;
    int recorded_offset = current_stack_offset;
    current_stack_offset -= 4; // Each float is 4 bytes
    symbol_count++;
    return recorded_offset;
}
