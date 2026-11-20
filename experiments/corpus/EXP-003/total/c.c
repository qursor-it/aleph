/* Reference for bench/tasks.md `total`: sum the integer arguments. */
#include <stdio.h>
#include <stdlib.h>

int main(int argc, char **argv) {
    long long total = 0;
    int index;

    for (index = 1; index < argc; index += 1) {
        total += strtoll(argv[index], NULL, 10);
    }
    printf("%lld\n", total);
    return 0;
}
