/* Reference for bench/tasks.md `primes`: all primes below 100. */
#include <stdio.h>

static int is_prime(long long n) {
    long long d;

    if (n < 2) {
        return 0;
    }
    for (d = 2; d * d <= n; d += 1) {
        if (n % d == 0) {
            return 0;
        }
    }
    return 1;
}

int main(void) {
    long long n;

    for (n = 2; n < 100; n += 1) {
        if (is_prime(n)) {
            printf("%lld\n", n);
        }
    }
    return 0;
}
