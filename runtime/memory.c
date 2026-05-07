#include <stdlib.h>

void* neuro_alloc(size_t size) {
    return malloc(size);
}

void neuro_free(void* ptr) {
    free(ptr);
}
