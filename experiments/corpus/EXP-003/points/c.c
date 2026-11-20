/* Reference for bench/tasks.md `points`: polygon perimeter. */
#include <math.h>
#include <stdio.h>

int main(void) {
    double xs[3] = {0.0, 3.0, 3.0};
    double ys[3] = {0.0, 0.0, 4.0};
    double total = 0.0;
    int index;

    for (index = 0; index < 3; index += 1) {
        int next = (index + 1) % 3;
        double dx = xs[index] - xs[next];
        double dy = ys[index] - ys[next];
        total += sqrt(dx * dx + dy * dy);
    }
    printf("%.1f\n", total);
    return 0;
}
