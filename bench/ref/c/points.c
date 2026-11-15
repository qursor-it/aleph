/* Reference for bench/tasks.md `points`: polygon perimeter. */
#include <stdio.h>

/* Same 40-iteration Newton iteration as the Aleph program's `sqrt`. */
static double newton_sqrt(double x) {
    double guess;
    int i;

    if (x <= 0.0) {
        return 0.0;
    }
    guess = x;
    for (i = 0; i < 40; i += 1) {
        guess = (guess + x / guess) / 2.0;
    }
    return guess;
}

int main(void) {
    double xs[3] = {0.0, 3.0, 3.0};
    double ys[3] = {0.0, 0.0, 4.0};
    double total = 0.0;
    int index;

    for (index = 0; index < 3; index += 1) {
        int next = (index + 1) % 3;
        double dx = xs[index] - xs[next];
        double dy = ys[index] - ys[next];
        total += newton_sqrt(dx * dx + dy * dy);
    }
    printf("%.1f\n", total);
    return 0;
}
