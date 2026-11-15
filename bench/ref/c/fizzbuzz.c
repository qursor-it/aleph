/* Reference for bench/tasks.md `fizzbuzz`: fizzbuzz from 1 to 30. */
#include <stdio.h>

int main(void) {
    int n;

    for (n = 1; n <= 30; n += 1) {
        if (n % 15 == 0) {
            puts("fizzbuzz");
        } else if (n % 3 == 0) {
            puts("fizz");
        } else if (n % 5 == 0) {
            puts("buzz");
        } else {
            printf("%d\n", n);
        }
    }
    return 0;
}
