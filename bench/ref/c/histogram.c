/* Reference for bench/tasks.md `histogram`: one bar per integer argument. */
#include <stdio.h>
#include <stdlib.h>

int main(int argc, char **argv) {
    int index;

    for (index = 1; index < argc; index += 1) {
        long long n = strtoll(argv[index], NULL, 10);
        long long k;
        printf("%lld ", n);
        for (k = 0; k < n; k += 1) {
            putchar('*');
        }
        putchar('\n');
    }
    return 0;
}
