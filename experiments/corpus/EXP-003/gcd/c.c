/* Reference for bench/tasks.md `gcd`: gcd and lcm of two integer arguments. */
#include <stdio.h>
#include <stdlib.h>

static long long gcd(long long a, long long b) {
    while (b != 0) {
        long long remainder = a % b;
        a = b;
        b = remainder;
    }
    return a < 0 ? -a : a;
}

int main(int argc, char **argv) {
    long long a;
    long long b;
    long long g;
    long long l;

    if (argc < 3) {
        fputs("invalid int\n", stderr);
        return 1;
    }
    a = strtoll(argv[1], NULL, 10);
    b = strtoll(argv[2], NULL, 10);
    g = gcd(a, b);
    l = (a == 0 || b == 0) ? 0 : a / g * b;
    printf("gcd %lld lcm %lld\n", g, l);
    return 0;
}
