/* Reference for bench/tasks.md `palindrome`: yes/no per argument, ASCII case. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int palindrome(const char *text) {
    size_t left = 0;
    size_t right = strlen(text);

    while (right > 0) {
        unsigned char a = (unsigned char)text[left];
        unsigned char b = (unsigned char)text[right - 1];
        if (a >= 'A' && a <= 'Z') {
            a = (unsigned char)(a + ('a' - 'A'));
        }
        if (b >= 'A' && b <= 'Z') {
            b = (unsigned char)(b + ('a' - 'A'));
        }
        if (a != b) {
            return 0;
        }
        left += 1;
        right -= 1;
    }
    return 1;
}

int main(int argc, char **argv) {
    int index;

    for (index = 1; index < argc; index += 1) {
        puts(palindrome(argv[index]) ? "yes" : "no");
    }
    return 0;
}
