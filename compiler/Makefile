CC = gcc
LEX = flex
YACC = yacc
CFLAGS = -I../include

SRCS = src/main.c src/codegen.c src/symbol_table.c src/ast.c
OBJS = $(SRCS:.c=.o)

all: compiler

compiler: y.tab.c lex.yy.c $(OBJS)
	$(CC) $(CFLAGS) y.tab.c lex.yy.c $(OBJS) -o compiler

y.tab.c y.tab.h: parser.y
	$(YACC) -d parser.y

lex.yy.c: lexer.l y.tab.h
	$(LEX) lexer.l

src/%.o: src/%.c
	$(CC) $(CFLAGS) -c $< -o $@

clean:
	rm -f compiler y.tab.c y.tab.h lex.yy.c src/*.o
