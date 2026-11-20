/* Reference for bench/tasks.md `reverse`: reverse the words of each argument. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void reverse_argument(const char *argument) {
    size_t length = strlen(argument);
    char *copy = malloc(length + 1);
    char **tokens = NULL;
    size_t capacity = 0;
    size_t count = 0;
    char *cursor;
    char *end;

    if (copy == NULL) {
        perror("malloc");
        exit(1);
    }
    memcpy(copy, argument, length + 1);

    cursor = copy;
    while (*cursor == ' ') {
        cursor += 1;
    }
    end = cursor + strlen(cursor);
    while (end > cursor && end[-1] == ' ') {
        end -= 1;
    }
    *end = '\0';

    while (*cursor != '\0') {
        if (count == capacity) {
            size_t grown = capacity == 0 ? 16 : capacity * 2;
            char **next = realloc(tokens, grown * sizeof(*tokens));
            if (next == NULL) {
                perror("realloc");
                free(tokens);
                free(copy);
                exit(1);
            }
            tokens = next;
            capacity = grown;
        }
        tokens[count] = cursor;
        count += 1;
        while (*cursor != '\0' && *cursor != ' ') {
            cursor += 1;
        }
        if (*cursor == ' ') {
            *cursor = '\0';
            cursor += 1;
        }
    }

    while (count > 0) {
        count -= 1;
        fputs(tokens[count], stdout);
        if (count > 0) {
            putchar(' ');
        }
    }
    putchar('\n');
    free(tokens);
    free(copy);
}

int main(int argc, char **argv) {
    int index;

    for (index = 1; index < argc; index += 1) {
        reverse_argument(argv[index]);
    }
    return 0;
}
