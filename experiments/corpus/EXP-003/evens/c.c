/* Reference for bench/tasks.md `evens`: even numbers from 0 to 20. */
#include <stdio.h>

int main(void) {
    int n;
    int first = 1;

    for (n = 0; n <= 20; n += 2) {
        if (!first) {
            putchar(' ');
        }
        printf("%d", n);
        first = 0;
    }
    putchar('\n');
    return 0;
}
