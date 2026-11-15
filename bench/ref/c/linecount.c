/* Reference for bench/tasks.md `linecount`: non-empty lines in data/in.txt. */
#include <stdio.h>
#include <stdlib.h>

static char *read_file(const char *path) {
    FILE *file = fopen(path, "rb");
    long size;
    char *text;
    size_t got;

    if (file == NULL) {
        perror(path);
        exit(1);
    }
    if (fseek(file, 0, SEEK_END) != 0 || (size = ftell(file)) < 0) {
        perror(path);
        exit(1);
    }
    rewind(file);
    text = malloc((size_t)size + 1);
    if (text == NULL) {
        perror("malloc");
        exit(1);
    }
    got = fread(text, 1, (size_t)size, file);
    fclose(file);
    text[got] = '\0';
    return text;
}

int main(void) {
    char *text = read_file("data/in.txt");
    const char *cursor;
    long long lines = 0;
    int nonempty = 0;

    for (cursor = text; *cursor != '\0'; cursor += 1) {
        if (*cursor == '\n') {
            if (nonempty) {
                lines += 1;
            }
            nonempty = 0;
        } else if (*cursor != ' ' && *cursor != '\t' && *cursor != '\r') {
            nonempty = 1;
        }
    }
    if (nonempty) {
        lines += 1;
    }
    printf("%lld\n", lines);
    free(text);
    return 0;
}
