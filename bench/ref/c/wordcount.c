/* Reference for bench/tasks.md `wordcount`: count words in data/in.txt. */
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
    long long words = 0;
    int in_word = 0;

    for (cursor = text; *cursor != '\0'; cursor += 1) {
        if (*cursor == ' ' || *cursor == '\n') {
            in_word = 0;
        } else if (!in_word) {
            in_word = 1;
            words += 1;
        }
    }
    printf("%lld\n", words);
    free(text);
    return 0;
}
