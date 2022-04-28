#include <unistd.h>

int fn(int a, int b, int c) {
    return a + b / c;
}

int main() {
    sleep(2);
    int x = fn(1, 2, 3);
    sleep(5);
    int y = fn(1, x, 3) + x;
}
