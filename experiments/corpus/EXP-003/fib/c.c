/* Reference for bench/tasks.md `fib`: print the 30th Fibonacci number. */
#include <stdio.h>

static long long fib(int n) {
    if (n < 2) {
        return n;
    }
    return fib(n - 1) + fib(n - 2);
}

int main(void) {
    printf("%lld\n", fib(30));
    return 0;
}
