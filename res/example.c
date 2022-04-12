int fn(int a, int b, int c) {
    return a + b / c;
}

int main() {
    int x = fn(1, 2, 3);
    int y = fn(1, x, 3) + x;
}
