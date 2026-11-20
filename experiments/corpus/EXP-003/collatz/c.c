/* Reference for bench/tasks.md `collatz`: sequence lengths for 1..20. */
#include <stdio.h>

static long long collatz_length(long long n) {
    long long length = 1;

    while (n != 1) {
        n = (n % 2 == 0) ? n / 2 : 3 * n + 1;
        length += 1;
    }
    return length;
}

int main(void) {
    int n;

    for (n = 1; n <= 20; n += 1) {
        printf("%lld\n", collatz_length(n));
    }
    return 0;
}
