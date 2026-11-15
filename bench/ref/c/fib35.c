/* Heavy reference for BENCH_SCALE=1: print the 35th Fibonacci number. */
#include <stdio.h>

static long long fib(long long n) {
    if (n < 2) {
        return n;
    }
    return fib(n - 1) + fib(n - 2);
}

int main(void) {
    printf("%lld\n", fib(35));
    return 0;
}
