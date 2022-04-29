#include <unistd.h>

int fn(int a, int b, int c) {
    return a + b / c;
}

int fib(int a) {
    if (a < 2) return a;

    return fib(a-1) + fib(a-2);
}

int main() {
    int y = fib(5);
    int z = fib(10);

    return 0;
}
