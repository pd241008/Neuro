CC = gcc
CFLAGS = -Wall -Wextra -O2

all: runtime.a

runtime.a: io.o memory.o
	ar rcs libneuro_runtime.a io.o memory.o

io.o: io.c
	$(CC) $(CFLAGS) -c io.c

memory.o: memory.c
	$(CC) $(CFLAGS) -c memory.c

clean:
	rm -f *.o *.a
