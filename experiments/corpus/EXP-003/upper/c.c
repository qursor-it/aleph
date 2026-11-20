/* Reference for bench/tasks.md `upper`: uppercase data/in.txt into out/up.txt. */
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
    FILE *output = fopen("out/up.txt", "wb");
    const char *cursor;

    if (output == NULL) {
        perror("out/up.txt");
        free(text);
        return 1;
    }
    for (cursor = text; *cursor != '\0'; cursor += 1) {
        unsigned char byte = (unsigned char)*cursor;
        if (byte >= 'a' && byte <= 'z') {
            byte = (unsigned char)(byte - ('a' - 'A'));
        }
        fputc(byte, output);
    }
    fclose(output);
    free(text);
    return 0;
}
