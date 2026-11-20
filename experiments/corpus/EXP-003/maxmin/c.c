/* Reference for bench/tasks.md `maxmin`: max and min of integer arguments. */
#include <stdio.h>
#include <stdlib.h>

int main(int argc, char **argv) {
    long long maximum = 0;
    long long minimum = 0;
    int any = 0;
    int index;

    for (index = 1; index < argc; index += 1) {
        long long value = strtoll(argv[index], NULL, 10);
        if (!any) {
            maximum = value;
            minimum = value;
            any = 1;
        } else {
            if (value > maximum) {
                maximum = value;
            }
            if (value < minimum) {
                minimum = value;
            }
        }
    }
    if (!any) {
        fputs("no numbers\n", stderr);
        return 1;
    }
    printf("max %lld min %lld\n", maximum, minimum);
    return 0;
}
