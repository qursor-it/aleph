#define _POSIX_C_SOURCE 200809L

/*
 * Tlön, the Aleph runtime (v0.1).  See tlon/tlon.h and ADR-0024.
 *
 * The runtime mirrors compiler/src/interp.rs byte for byte.  It is C99 plus
 * libc and pthreads; generated code never frees arena memory.
 */
#include "tlon.h"

#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <locale.h>
#include <math.h>
#include <pthread.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <unistd.h>

/* ------------------------------------------------------------------ */
/* Global state                                                        */
/* ------------------------------------------------------------------ */

static const char *g_file = "";
static const AlUse *g_uses;
static size_t g_use_count;
static int g_argc;
static const char *const *g_argv;

static size_t g_eval_depth;
static size_t g_call_depth;

/* Native stack guard for the big main thread (set in main_thread). */
#define AL_STACK_GUARD (1024u * 1024u)
static uintptr_t g_stack_base;
static bool g_stack_base_set;

static const AlLoc g_default_loc = {0, 0, 0};
static const AlLoc *g_locs[AL_MAX_EVAL_DEPTH + 1];
static size_t g_loc_depth;

void aleph_set_file(const char *file) { g_file = file ? file : ""; }

void aleph_set_uses(const AlUse *uses, size_t count) {
    g_uses = uses;
    g_use_count = count;
}

void aleph_set_args(int argc, const char *const *argv) {
    g_argc = argc;
    g_argv = argv;
}

/* ------------------------------------------------------------------ */
/* Traps                                                               */
/* ------------------------------------------------------------------ */

static const AlLoc *current_loc(void) {
    if (g_loc_depth > 0 && g_locs[g_loc_depth - 1] != NULL) {
        return g_locs[g_loc_depth - 1];
    }
    return &g_default_loc;
}

static void write_json_string(FILE *out, const char *text) {
    const unsigned char *p = (const unsigned char *)text;
    fputc('"', out);
    while (*p != 0) {
        unsigned char c = *p++;
        switch (c) {
        case '"':
            fputs("\\\"", out);
            break;
        case '\\':
            fputs("\\\\", out);
            break;
        case '\b':
            fputs("\\b", out);
            break;
        case '\f':
            fputs("\\f", out);
            break;
        case '\n':
            fputs("\\n", out);
            break;
        case '\r':
            fputs("\\r", out);
            break;
        case '\t':
            fputs("\\t", out);
            break;
        default:
            if (c < 0x20) {
                fprintf(out, "\\u%04x", (unsigned int)c);
            } else {
                fputc((int)c, out);
            }
            break;
        }
    }
    fputc('"', out);
}

void aleph_trap_at(const char *code, const char *msg, const AlLoc *loc) {
    const AlLoc *where = loc ? loc : current_loc();
    fflush(stdout);
    fputs("{\"code\":", stderr);
    write_json_string(stderr, code);
    fputs(",\"severity\":\"error\",\"file\":", stderr);
    write_json_string(stderr, g_file);
    fprintf(stderr, ",\"line\":%u,\"col\":%u,\"node\":\"n%u\",\"msg\":",
            (unsigned int)where->line, (unsigned int)where->col,
            (unsigned int)where->node);
    write_json_string(stderr, msg);
    fputs("}\n", stderr);
    fflush(stderr);
    exit(70);
}

void aleph_trap(const char *code, const char *msg) {
    aleph_trap_at(code, msg, current_loc());
}

static void out_of_memory(void) AL_NORETURN;
static void out_of_memory(void) {
    aleph_trap("R005", "allocation limit exceeded");
}

/* ------------------------------------------------------------------ */
/* Arena                                                               */
/* ------------------------------------------------------------------ */

typedef struct AlChunk {
    struct AlChunk *next;
    size_t used;
    size_t cap;
    unsigned char *data;
} AlChunk;

#define AL_CHUNK_MIN (64u * 1024u)
#define AL_ALIGN 16u

static AlChunk *g_chunks;

void *aleph_alloc(size_t size) {
    AlChunk *chunk;
    void *result;
    if (size == 0) {
        size = 1;
    }
    if (size > SIZE_MAX - (AL_ALIGN - 1)) {
        out_of_memory();
    }
    size = (size + (AL_ALIGN - 1)) & ~(size_t)(AL_ALIGN - 1);
    chunk = g_chunks;
    if (chunk == NULL || chunk->cap - chunk->used < size) {
        size_t cap = size > AL_CHUNK_MIN ? size : AL_CHUNK_MIN;
        size_t header = sizeof(AlChunk) + (AL_ALIGN - 1);
        unsigned char *base;
        if (cap < size || cap > SIZE_MAX - header) {
            out_of_memory();
        }
        chunk = (AlChunk *)malloc(header + cap);
        if (chunk == NULL) {
            out_of_memory();
        }
        base = (unsigned char *)chunk + sizeof(AlChunk);
        chunk->next = g_chunks;
        chunk->used = 0;
        chunk->cap = cap;
        chunk->data = (unsigned char *)(((uintptr_t)base + (AL_ALIGN - 1)) &
                                        ~(uintptr_t)(AL_ALIGN - 1));
        g_chunks = chunk;
    }
    result = chunk->data + chunk->used;
    chunk->used += size;
    return result;
}

/* ------------------------------------------------------------------ */
/* Depth accounting                                                    */
/* ------------------------------------------------------------------ */

void aleph_eval_enter(const AlLoc *loc) {
    if (g_eval_depth >= AL_MAX_EVAL_DEPTH) {
        aleph_trap_at("R004", "stack overflow", loc);
    }
    g_eval_depth += 1;
    if (g_loc_depth < AL_MAX_EVAL_DEPTH + 1) {
        g_locs[g_loc_depth++] = loc;
    }
}

void aleph_eval_leave(void) {
    if (g_eval_depth > 0) {
        g_eval_depth -= 1;
    }
    if (g_loc_depth > 0) {
        g_loc_depth -= 1;
    }
}

static void native_stack_guard(const AlLoc *loc) {
    uintptr_t here;
    uintptr_t used;
    if (!g_stack_base_set) {
        return;
    }
    here = (uintptr_t)&here;
    used = here <= g_stack_base ? g_stack_base - here : here - g_stack_base;
    if (used > (uintptr_t)(AL_MAIN_STACK_SIZE - AL_STACK_GUARD)) {
        aleph_trap_at("R004", "stack overflow", loc);
    }
}

void aleph_call_enter(const AlLoc *loc) {
    native_stack_guard(loc);
    if (g_call_depth >= AL_MAX_CALL_DEPTH || g_eval_depth >= AL_MAX_EVAL_DEPTH) {
        aleph_trap_at("R004", "stack overflow", loc);
    }
    g_call_depth += 1;
    g_eval_depth += 1;
}

void aleph_call_leave(void) {
    if (g_call_depth > 0) {
        g_call_depth -= 1;
    }
    if (g_eval_depth > 0) {
        g_eval_depth -= 1;
    }
}

/* ------------------------------------------------------------------ */
/* Value constructors                                                  */
/* ------------------------------------------------------------------ */

AlValue aleph_unit(void) {
    AlValue value;
    value.tag = AL_UNIT;
    value.u.i64 = 0;
    return value;
}

AlValue aleph_i64(int64_t number) {
    AlValue value;
    value.tag = AL_I64;
    value.u.i64 = number;
    return value;
}

AlValue aleph_f64(double number) {
    AlValue value;
    value.tag = AL_F64;
    value.u.f64 = number;
    return value;
}

AlValue aleph_f64_bits(uint64_t bits) {
    double number;
    memcpy(&number, &bits, sizeof(number));
    return aleph_f64(number);
}

AlValue aleph_bool(bool flag) {
    AlValue value;
    value.tag = AL_BOOL;
    value.u.b = flag;
    return value;
}

AlValue aleph_str_n(const char *text, size_t len) {
    AlValue value;
    AlStr *string;
    char *data;
    if (len == SIZE_MAX) {
        out_of_memory();
    }
    string = (AlStr *)aleph_alloc(sizeof(AlStr));
    data = (char *)aleph_alloc(len + 1);
    if (len > 0) {
        if (text != NULL) {
            memcpy(data, text, len);
        } else {
            memset(data, 0, len);
        }
    }
    data[len] = '\0';
    string->len = len;
    string->data = data;
    value.tag = AL_STR;
    value.u.str = string;
    return value;
}

AlValue aleph_str(const char *text) {
    if (text == NULL) {
        return aleph_str_n("", 0);
    }
    return aleph_str_n(text, strlen(text));
}

AlValue aleph_list_new(size_t len) {
    AlValue value;
    AlList *list;
    size_t index;
    if (len > (SIZE_MAX - sizeof(AlList)) / sizeof(AlValue)) {
        out_of_memory();
    }
    list = (AlList *)aleph_alloc(sizeof(AlList) + len * sizeof(AlValue));
    list->len = len;
    for (index = 0; index < len; index += 1) {
        list->items[index] = aleph_unit();
    }
    value.tag = AL_LIST;
    value.u.list = list;
    return value;
}

AlValue aleph_list(const AlValue *items, size_t len) {
    AlValue value = aleph_list_new(len);
    if (len > 0 && items != NULL) {
        memcpy(value.u.list->items, items, len * sizeof(AlValue));
    }
    return value;
}

AlValue aleph_none(void) {
    AlValue value;
    value.tag = AL_NONE;
    value.u.i64 = 0;
    return value;
}

static AlValue boxed(AlTag tag, AlValue payload) {
    AlValue value;
    AlBox *box = (AlBox *)aleph_alloc(sizeof(AlBox));
    box->value = payload;
    value.tag = tag;
    value.u.box = box;
    return value;
}

AlValue aleph_some(AlValue value) { return boxed(AL_SOME, value); }
AlValue aleph_ok(AlValue value) { return boxed(AL_OK, value); }
AlValue aleph_err(AlValue value) { return boxed(AL_ERR, value); }

AlValue aleph_record(const AlRecordInfo *info, const AlValue *fields,
                     size_t count) {
    AlValue value;
    AlRecord *record;
    if (count > (SIZE_MAX - sizeof(AlRecord)) / sizeof(AlValue)) {
        out_of_memory();
    }
    record = (AlRecord *)aleph_alloc(sizeof(AlRecord) + count * sizeof(AlValue));
    record->info = info;
    record->n = count;
    if (count > 0) {
        if (fields != NULL) {
            memcpy(record->fields, fields, count * sizeof(AlValue));
        } else {
            memset(record->fields, 0, count * sizeof(AlValue));
        }
    }
    value.tag = AL_RECORD;
    value.u.rec = record;
    return value;
}

AlValue aleph_fn_kind(AlValue (*code)(AlValue *, size_t), const char *name,
                      AlFnKind kind) {
    AlValue value;
    AlFn *fn = (AlFn *)aleph_alloc(sizeof(AlFn));
    fn->code = code;
    fn->name = name;
    fn->kind = kind;
    value.tag = AL_FN;
    value.u.fn = fn;
    return value;
}

AlValue aleph_fn(AlValue (*code)(AlValue *, size_t), const char *name) {
    return aleph_fn_kind(code, name, AL_FN_USER);
}

AlValue aleph_fn_static(const AlFn *fn) {
    AlValue value;
    value.tag = AL_FN;
    value.u.fn = fn;
    return value;
}

/* ------------------------------------------------------------------ */
/* Checked i64 arithmetic                                              */
/* ------------------------------------------------------------------ */

static int64_t as_i64(AlValue value) { return value.u.i64; }

AlValue aleph_add_i64(AlValue left, AlValue right) {
    int64_t result;
    if (__builtin_add_overflow(as_i64(left), as_i64(right), &result)) {
        aleph_trap("R001", "integer overflow");
    }
    return aleph_i64(result);
}

AlValue aleph_sub_i64(AlValue left, AlValue right) {
    int64_t result;
    if (__builtin_sub_overflow(as_i64(left), as_i64(right), &result)) {
        aleph_trap("R001", "integer overflow");
    }
    return aleph_i64(result);
}

AlValue aleph_mul_i64(AlValue left, AlValue right) {
    int64_t result;
    if (__builtin_mul_overflow(as_i64(left), as_i64(right), &result)) {
        aleph_trap("R001", "integer overflow");
    }
    return aleph_i64(result);
}

AlValue aleph_div_i64(AlValue left, AlValue right) {
    int64_t a = as_i64(left);
    int64_t b = as_i64(right);
    if (b == 0) {
        aleph_trap("R002", "division by zero");
    }
    if (a == INT64_MIN && b == -1) {
        aleph_trap("R001", "integer overflow");
    }
    return aleph_i64(a / b);
}

AlValue aleph_mod_i64(AlValue left, AlValue right) {
    int64_t a = as_i64(left);
    int64_t b = as_i64(right);
    if (b == 0) {
        aleph_trap("R002", "division by zero");
    }
    if (a == INT64_MIN && b == -1) {
        aleph_trap("R001", "integer overflow");
    }
    return aleph_i64(a % b);
}

AlValue aleph_neg_i64(AlValue value) {
    int64_t number = as_i64(value);
    if (number == INT64_MIN) {
        aleph_trap("R001", "integer overflow");
    }
    return aleph_i64(-number);
}

/* ------------------------------------------------------------------ */
/* Equality and ordering                                               */
/* ------------------------------------------------------------------ */

bool aleph_values_equal(AlValue left, AlValue right) {
    size_t index;
    if (left.tag != right.tag) {
        return false;
    }
    switch (left.tag) {
    case AL_UNIT:
    case AL_NONE:
        return true;
    case AL_I64:
        return left.u.i64 == right.u.i64;
    case AL_F64:
        return left.u.f64 == right.u.f64;
    case AL_BOOL:
        return left.u.b == right.u.b;
    case AL_STR:
        return left.u.str->len == right.u.str->len &&
               memcmp(left.u.str->data, right.u.str->data, left.u.str->len) ==
                   0;
    case AL_LIST:
        if (left.u.list->len != right.u.list->len) {
            return false;
        }
        for (index = 0; index < left.u.list->len; index += 1) {
            if (!aleph_values_equal(left.u.list->items[index],
                                    right.u.list->items[index])) {
                return false;
            }
        }
        return true;
    case AL_SOME:
    case AL_OK:
    case AL_ERR:
        return aleph_values_equal(left.u.box->value, right.u.box->value);
    case AL_RECORD:
        if (left.u.rec->n != right.u.rec->n) {
            return false;
        }
        if (left.u.rec->info != NULL && right.u.rec->info != NULL) {
            if (strcmp(left.u.rec->info->name, right.u.rec->info->name) != 0) {
                return false;
            }
        }
        for (index = 0; index < left.u.rec->n; index += 1) {
            if (!aleph_values_equal(left.u.rec->fields[index],
                                    right.u.rec->fields[index])) {
                return false;
            }
        }
        return true;
    case AL_FN:
        if (left.u.fn->kind != right.u.fn->kind) {
            return false;
        }
        if (left.u.fn->name == NULL || right.u.fn->name == NULL) {
            return left.u.fn->name == right.u.fn->name &&
                   left.u.fn->code == right.u.fn->code;
        }
        return strcmp(left.u.fn->name, right.u.fn->name) == 0;
    }
    return false;
}

static int compare_ordering(int ordering, int op) {
    /* op: 0 <, 1 <=, 2 >, 3 >= */
    switch (op) {
    case 0:
        return ordering < 0;
    case 1:
        return ordering <= 0;
    case 2:
        return ordering > 0;
    default:
        return ordering >= 0;
    }
}

static int double_order(double left, double right, bool *unordered) {
    if (isnan(left) || isnan(right)) {
        *unordered = true;
        return 0;
    }
    *unordered = false;
    if (left < right) {
        return -1;
    }
    if (left > right) {
        return 1;
    }
    return 0;
}

static bool compare_op(AlValue left, AlValue right, int op) {
    bool unordered;
    int ordering;
    if (left.tag != right.tag) {
        return false;
    }
    if (left.tag == AL_I64) {
        ordering = left.u.i64 < right.u.i64
                       ? -1
                       : (left.u.i64 > right.u.i64 ? 1 : 0);
        return compare_ordering(ordering, op) != 0;
    }
    if (left.tag == AL_F64) {
        ordering = double_order(left.u.f64, right.u.f64, &unordered);
        if (unordered) {
            return false;
        }
        return compare_ordering(ordering, op) != 0;
    }
    return false;
}

bool aleph_lt(AlValue left, AlValue right) { return compare_op(left, right, 0); }
bool aleph_le(AlValue left, AlValue right) { return compare_op(left, right, 1); }
bool aleph_gt(AlValue left, AlValue right) { return compare_op(left, right, 2); }
bool aleph_ge(AlValue left, AlValue right) { return compare_op(left, right, 3); }

/* ------------------------------------------------------------------ */
/* Buffers and show/literal                                            */
/* ------------------------------------------------------------------ */

typedef struct Buf {
    char *data;
    size_t len;
    size_t cap;
} Buf;

static void buf_reserve(Buf *buffer, size_t extra) {
    size_t need;
    size_t cap;
    char *data;
    if (extra > SIZE_MAX - buffer->len) {
        out_of_memory();
    }
    need = buffer->len + extra;
    if (need <= buffer->cap) {
        return;
    }
    cap = buffer->cap == 0 ? 64 : buffer->cap;
    while (cap < need) {
        if (cap > SIZE_MAX / 2) {
            out_of_memory();
        }
        cap *= 2;
    }
    data = (char *)realloc(buffer->data, cap);
    if (data == NULL) {
        out_of_memory();
    }
    buffer->data = data;
    buffer->cap = cap;
}

static void buf_putc(Buf *buffer, char byte) {
    buf_reserve(buffer, 1);
    buffer->data[buffer->len++] = byte;
}

static void buf_write(Buf *buffer, const char *text, size_t len) {
    if (len == 0) {
        return;
    }
    buf_reserve(buffer, len);
    memcpy(buffer->data + buffer->len, text, len);
    buffer->len += len;
}

static void buf_cstr(Buf *buffer, const char *text) {
    buf_write(buffer, text, strlen(text));
}

static void buf_finish(Buf *buffer) {
    buf_reserve(buffer, 1);
    buffer->data[buffer->len] = '\0';
}

/* Rust's `{:?}` for f64.
 *
 * The shortest round-tripping decimal is generated from a high-precision
 * `%e` expansion so that exact halfway cases round away from zero, matching
 * Rust's flt2dec rather than printf's ties-to-even. */
static void exact_digits(double magnitude, char *digits, int *exponent) {
    char exact[1024];
    const char *e;
    const char *p;
    size_t count = 0;

    snprintf(exact, sizeof(exact), "%.800e", magnitude);
    e = strchr(exact, 'e');
    if (e == NULL) {
        e = exact + strlen(exact);
    }
    for (p = exact; p < e; p += 1) {
        if (*p >= '0' && *p <= '9' && count < 900) {
            digits[count++] = *p;
        }
    }
    while (count < 900) {
        digits[count++] = '0';
    }
    *exponent = (*e != '\0') ? atoi(e + 1) : 0;
}

static void round_digits(const char *exact, size_t precision, char *out,
                         int *exponent) {
    size_t index;
    bool round_up = exact[precision] >= '5';

    memcpy(out, exact, precision);
    out[precision] = '\0';
    if (round_up) {
        index = precision;
        while (index > 0) {
            index -= 1;
            if (out[index] < '9') {
                out[index] += 1;
                break;
            }
            out[index] = '0';
        }
        if (index == 0 && out[0] == '0') {
            out[0] = '1';
            for (index = 1; index < precision; index += 1) {
                out[index] = '0';
            }
            *exponent += 1;
        }
    }
}

static void format_f64(char *out, double number) {
    char exact[1024];
    char candidate[32];
    char digits[32];
    char tmp[64];
    int exponent;
    int start_exponent;
    size_t precision;
    size_t chosen = 1;
    int negative;
    double magnitude;
    char *o = out;

    if (isnan(number)) {
        strcpy(out, "NaN");
        return;
    }
    if (isinf(number)) {
        strcpy(out, signbit(number) ? "-inf" : "inf");
        return;
    }
    if (number == 0.0) {
        strcpy(out, signbit(number) ? "-0.0" : "0.0");
        return;
    }
    negative = signbit(number) ? 1 : 0;
    magnitude = negative ? -number : number;

    exact_digits(magnitude, exact, &start_exponent);
    exponent = start_exponent;
    for (precision = 1; precision <= 17; precision += 1) {
        int candidate_exponent = start_exponent;
        round_digits(exact, precision, candidate, &candidate_exponent);
        if (precision == 1) {
            snprintf(tmp, sizeof(tmp), "%ce%d", candidate[0],
                     candidate_exponent);
        } else {
            snprintf(tmp, sizeof(tmp), "%c.%se%d", candidate[0],
                     candidate + 1, candidate_exponent);
        }
        if (strtod(tmp, NULL) == magnitude) {
            memcpy(digits, candidate, precision);
            digits[precision] = '\0';
            exponent = candidate_exponent;
            chosen = precision;
            break;
        }
        chosen = precision;
    }
    if (chosen == 17) {
        int candidate_exponent = start_exponent;
        round_digits(exact, 17, candidate, &candidate_exponent);
        memcpy(digits, candidate, 17);
        digits[17] = '\0';
        exponent = candidate_exponent;
    }
    precision = chosen;
    while (precision > 1 && digits[precision - 1] == '0') {
        precision -= 1;
    }

    if (negative) {
        *o++ = '-';
    }
    if (exponent < -4 || exponent >= 16) {
        size_t index;
        *o++ = digits[0];
        if (precision > 1) {
            *o++ = '.';
            for (index = 1; index < precision; index += 1) {
                *o++ = digits[index];
            }
        }
        *o++ = 'e';
        if (exponent < 0) {
            *o++ = '-';
            exponent = -exponent;
        }
        snprintf(tmp, sizeof(tmp), "%d", exponent);
        strcpy(o, tmp);
        return;
    }
    if (exponent >= 0) {
        size_t integer_digits = (size_t)exponent + 1;
        size_t index;
        if (integer_digits >= precision) {
            for (index = 0; index < precision; index += 1) {
                *o++ = digits[index];
            }
            for (index = precision; index < integer_digits; index += 1) {
                *o++ = '0';
            }
            *o++ = '.';
            *o++ = '0';
        } else {
            for (index = 0; index < integer_digits; index += 1) {
                *o++ = digits[index];
            }
            *o++ = '.';
            for (index = integer_digits; index < precision; index += 1) {
                *o++ = digits[index];
            }
        }
    } else {
        int index;
        *o++ = '0';
        *o++ = '.';
        for (index = 0; index < -exponent - 1; index += 1) {
            *o++ = '0';
        }
        for (size_t index = 0; index < precision; index += 1) {
            *o++ = digits[index];
        }
    }
    *o = '\0';
}

static void escape_literal(Buf *buffer, const AlStr *text) {
    size_t index;
    buf_putc(buffer, '"');
    for (index = 0; index < text->len; index += 1) {
        char byte = text->data[index];
        switch (byte) {
        case '\n':
            buf_cstr(buffer, "\\n");
            break;
        case '\t':
            buf_cstr(buffer, "\\t");
            break;
        case '"':
            buf_cstr(buffer, "\\\"");
            break;
        case '\\':
            buf_cstr(buffer, "\\\\");
            break;
        default:
            buf_putc(buffer, byte);
            break;
        }
    }
    buf_putc(buffer, '"');
}

static void show_into(Buf *buffer, AlValue value);
static void literal_into(Buf *buffer, AlValue value);

static void show_into(Buf *buffer, AlValue value) {
    char number[64];
    size_t index;
    switch (value.tag) {
    case AL_UNIT:
        buf_cstr(buffer, "nil");
        break;
    case AL_I64:
        snprintf(number, sizeof(number), "%lld", (long long)value.u.i64);
        buf_cstr(buffer, number);
        break;
    case AL_F64:
        format_f64(number, value.u.f64);
        buf_cstr(buffer, number);
        break;
    case AL_BOOL:
        buf_cstr(buffer, value.u.b ? "true" : "false");
        break;
    case AL_STR:
        buf_write(buffer, value.u.str->data, value.u.str->len);
        break;
    case AL_LIST:
        buf_putc(buffer, '[');
        for (index = 0; index < value.u.list->len; index += 1) {
            if (index > 0) {
                buf_putc(buffer, ' ');
            }
            literal_into(buffer, value.u.list->items[index]);
        }
        buf_putc(buffer, ']');
        break;
    case AL_NONE:
        buf_cstr(buffer, "none");
        break;
    case AL_SOME:
        buf_cstr(buffer, "some ");
        literal_into(buffer, value.u.box->value);
        break;
    case AL_OK:
        buf_cstr(buffer, "ok ");
        literal_into(buffer, value.u.box->value);
        break;
    case AL_ERR:
        buf_cstr(buffer, "err ");
        literal_into(buffer, value.u.box->value);
        break;
    case AL_RECORD:
        if (value.u.rec->info != NULL) {
            buf_cstr(buffer, value.u.rec->info->name);
        }
        for (index = 0; index < value.u.rec->n; index += 1) {
            buf_putc(buffer, ' ');
            literal_into(buffer, value.u.rec->fields[index]);
        }
        break;
    case AL_FN:
        buf_putc(buffer, '@');
        if (value.u.fn->name != NULL) {
            buf_cstr(buffer, value.u.fn->name);
        }
        break;
    }
}

static void literal_into(Buf *buffer, AlValue value) {
    if (value.tag == AL_STR) {
        escape_literal(buffer, value.u.str);
    } else {
        show_into(buffer, value);
    }
}

static AlValue take_buffer(Buf *buffer) {
    AlValue value = aleph_str_n(buffer->data, buffer->len);
    free(buffer->data);
    return value;
}

AlValue aleph_show(AlValue value) {
    Buf buffer = {NULL, 0, 0};
    show_into(&buffer, value);
    return take_buffer(&buffer);
}

AlValue aleph_literal(AlValue value) {
    Buf buffer = {NULL, 0, 0};
    literal_into(&buffer, value);
    return take_buffer(&buffer);
}

/* ------------------------------------------------------------------ */
/* List and string builtins                                            */
/* ------------------------------------------------------------------ */

AlValue aleph_b_len(AlValue xs) {
    return aleph_i64((int64_t)xs.u.list->len);
}

AlValue aleph_b_at(AlValue xs, AlValue index) {
    int64_t i = index.u.i64;
    if (i < 0 || (uint64_t)i >= (uint64_t)xs.u.list->len) {
        return aleph_none();
    }
    return aleph_some(xs.u.list->items[(size_t)i]);
}

AlValue aleph_b_push(AlValue xs, AlValue value) {
    AlValue result = aleph_list_new(xs.u.list->len + 1);
    if (xs.u.list->len > 0) {
        memcpy(result.u.list->items, xs.u.list->items,
               xs.u.list->len * sizeof(AlValue));
    }
    result.u.list->items[xs.u.list->len] = value;
    return result;
}

static size_t clamp_index(int64_t value, size_t len) {
    if (value < 0) {
        return 0;
    }
    if ((uint64_t)value > (uint64_t)len) {
        return len;
    }
    return (size_t)value;
}

AlValue aleph_b_slice(AlValue xs, AlValue start, AlValue end) {
    size_t begin = clamp_index(start.u.i64, xs.u.list->len);
    size_t finish = clamp_index(end.u.i64, xs.u.list->len);
    if (begin >= finish) {
        return aleph_list_new(0);
    }
    return aleph_list(xs.u.list->items + begin, finish - begin);
}

AlValue aleph_b_rev(AlValue xs) {
    size_t index;
    size_t len = xs.u.list->len;
    AlValue result = aleph_list_new(len);
    for (index = 0; index < len; index += 1) {
        result.u.list->items[index] = xs.u.list->items[len - 1 - index];
    }
    return result;
}

static int f64_total_cmp(double left, double right) {
    uint64_t left_bits;
    uint64_t right_bits;
    int64_t left_order;
    int64_t right_order;
    memcpy(&left_bits, &left, sizeof(left_bits));
    memcpy(&right_bits, &right, sizeof(right_bits));
    left_order = (int64_t)left_bits;
    right_order = (int64_t)right_bits;
    if (left_order < 0) {
        left_order ^= INT64_MAX;
    }
    if (right_order < 0) {
        right_order ^= INT64_MAX;
    }
    if (left_order < right_order) {
        return -1;
    }
    if (left_order > right_order) {
        return 1;
    }
    return 0;
}

static int value_order(AlValue left, AlValue right) {
    size_t common;
    int compared;
    if (left.tag != right.tag) {
        return 0;
    }
    switch (left.tag) {
    case AL_I64:
        return left.u.i64 < right.u.i64
                   ? -1
                   : (left.u.i64 > right.u.i64 ? 1 : 0);
    case AL_F64:
        return f64_total_cmp(left.u.f64, right.u.f64);
    case AL_BOOL:
        return (int)left.u.b - (int)right.u.b;
    case AL_STR:
        common = left.u.str->len < right.u.str->len ? left.u.str->len
                                                    : right.u.str->len;
        compared = memcmp(left.u.str->data, right.u.str->data, common);
        if (compared != 0) {
            return compared < 0 ? -1 : 1;
        }
        if (left.u.str->len < right.u.str->len) {
            return -1;
        }
        if (left.u.str->len > right.u.str->len) {
            return 1;
        }
        return 0;
    default:
        return 0;
    }
}

static int qsort_value_order(const void *left, const void *right) {
    return value_order(*(const AlValue *)left, *(const AlValue *)right);
}

AlValue aleph_b_sort(AlValue xs) {
    AlValue result = aleph_list(xs.u.list->items, xs.u.list->len);
    if (result.u.list->len > 1) {
        qsort(result.u.list->items, result.u.list->len, sizeof(AlValue),
              qsort_value_order);
    }
    return result;
}

AlValue aleph_b_range(AlValue start, AlValue end) {
    int64_t first = start.u.i64;
    int64_t last = end.u.i64;
    int64_t count;
    int64_t index;
    AlValue result;
    if (first >= last) {
        return aleph_list_new(0);
    }
    if (__builtin_sub_overflow(last, first, &count) ||
        count > AL_MAX_RANGE_LEN) {
        aleph_trap("R005", "allocation limit exceeded");
    }
    result = aleph_list_new((size_t)count);
    for (index = 0; index < count; index += 1) {
        result.u.list->items[(size_t)index] = aleph_i64(first + index);
    }
    return result;
}

AlValue aleph_b_sum(AlValue xs) {
    size_t index;
    int64_t total = 0;
    for (index = 0; index < xs.u.list->len; index += 1) {
        if (__builtin_add_overflow(total, xs.u.list->items[index].u.i64,
                                   &total)) {
            aleph_trap("R001", "integer overflow");
        }
    }
    return aleph_i64(total);
}

AlValue aleph_b_map(AlValue function, AlValue xs) {
    const AlFn *fn = function.u.fn;
    const AlList *list = xs.u.list;
    AlValue result = aleph_list_new(list->len);
    size_t index;
    for (index = 0; index < list->len; index += 1) {
        AlValue args[1];
        args[0] = list->items[index];
        result.u.list->items[index] = fn->code(args, 1);
    }
    return result;
}

AlValue aleph_b_filter(AlValue function, AlValue xs) {
    const AlFn *fn = function.u.fn;
    const AlList *list = xs.u.list;
    AlValue result = aleph_list_new(list->len);
    size_t index;
    size_t kept = 0;
    for (index = 0; index < list->len; index += 1) {
        AlValue args[1];
        AlValue keep;
        args[0] = list->items[index];
        keep = fn->code(args, 1);
        if (keep.u.b) {
            result.u.list->items[kept] = list->items[index];
            kept += 1;
        }
    }
    result.u.list->len = kept;
    return result;
}

AlValue aleph_b_fold(AlValue function, AlValue init, AlValue xs) {
    const AlFn *fn = function.u.fn;
    const AlList *list = xs.u.list;
    AlValue accumulator = init;
    size_t index;
    for (index = 0; index < list->len; index += 1) {
        AlValue args[2];
        args[0] = accumulator;
        args[1] = list->items[index];
        accumulator = fn->code(args, 2);
    }
    return accumulator;
}

AlValue aleph_b_cat(AlValue left, AlValue right) {
    size_t left_len = left.u.str->len;
    size_t right_len = right.u.str->len;
    char *data;
    AlStr *string;
    AlValue value;
    if (left_len > SIZE_MAX - right_len ||
        left_len + right_len == SIZE_MAX) {
        out_of_memory();
    }
    data = (char *)aleph_alloc(left_len + right_len + 1);
    memcpy(data, left.u.str->data, left_len);
    memcpy(data + left_len, right.u.str->data, right_len);
    data[left_len + right_len] = '\0';
    string = (AlStr *)aleph_alloc(sizeof(AlStr));
    string->len = left_len + right_len;
    string->data = data;
    value.tag = AL_STR;
    value.u.str = string;
    return value;
}

AlValue aleph_b_count(AlValue text) {
    return aleph_i64((int64_t)text.u.str->len);
}

static size_t utf8_decode(const unsigned char *text, size_t len,
                          uint32_t *codepoint) {
    unsigned char first;
    if (len == 0) {
        return 0;
    }
    first = text[0];
    if (first < 0x80) {
        *codepoint = first;
        return 1;
    }
    if (first >= 0xC2 && first <= 0xDF) {
        if (len < 2 || (text[1] & 0xC0) != 0x80) {
            return 0;
        }
        *codepoint = ((uint32_t)(first & 0x1F) << 6) | (text[1] & 0x3F);
        return 2;
    }
    if (first >= 0xE0 && first <= 0xEF) {
        if (len < 3 || (text[1] & 0xC0) != 0x80 ||
            (text[2] & 0xC0) != 0x80) {
            return 0;
        }
        if (first == 0xE0 && text[1] < 0xA0) {
            return 0;
        }
        if (first == 0xED && text[1] >= 0xA0) {
            return 0;
        }
        *codepoint = ((uint32_t)(first & 0x0F) << 12) |
                     ((uint32_t)(text[1] & 0x3F) << 6) | (text[2] & 0x3F);
        return 3;
    }
    if (first >= 0xF0 && first <= 0xF4) {
        if (len < 4 || (text[1] & 0xC0) != 0x80 ||
            (text[2] & 0xC0) != 0x80 || (text[3] & 0xC0) != 0x80) {
            return 0;
        }
        if (first == 0xF0 && text[1] < 0x90) {
            return 0;
        }
        if (first == 0xF4 && text[1] >= 0x90) {
            return 0;
        }
        *codepoint = ((uint32_t)(first & 0x07) << 18) |
                     ((uint32_t)(text[1] & 0x3F) << 12) |
                     ((uint32_t)(text[2] & 0x3F) << 6) | (text[3] & 0x3F);
        return 4;
    }
    return 0;
}

static bool utf8_valid(const unsigned char *text, size_t len) {
    size_t index = 0;
    while (index < len) {
        uint32_t codepoint;
        size_t width = utf8_decode(text + index, len - index, &codepoint);
        if (width == 0) {
            return false;
        }
        index += width;
    }
    return true;
}

static AlValue string_from_bytes(const char *data, size_t len) {
    return aleph_str_n(data, len);
}

static bool find_separator(const char *source, size_t source_len, size_t from,
                           const char *sep, size_t sep_len,
                           size_t *position) {
    size_t at = from;
    while (at + sep_len <= source_len) {
        if (memcmp(source + at, sep, sep_len) == 0) {
            *position = at;
            return true;
        }
        at += 1;
    }
    *position = source_len;
    return false;
}

AlValue aleph_b_split(AlValue text, AlValue separator) {
    const char *source = text.u.str->data;
    size_t source_len = text.u.str->len;
    const char *sep = separator.u.str->data;
    size_t sep_len = separator.u.str->len;
    AlValue result;
    size_t count = 0;
    size_t index;

    if (sep_len == 0) {
        index = 0;
        while (index < source_len) {
            uint32_t codepoint;
            size_t width =
                utf8_decode((const unsigned char *)source + index,
                            source_len - index, &codepoint);
            if (width == 0) {
                width = 1;
            }
            index += width;
            count += 1;
        }
        result = aleph_list_new(count);
        index = 0;
        count = 0;
        while (index < source_len) {
            uint32_t codepoint;
            size_t width =
                utf8_decode((const unsigned char *)source + index,
                            source_len - index, &codepoint);
            if (width == 0) {
                width = 1;
            }
            result.u.list->items[count++] =
                string_from_bytes(source + index, width);
            index += width;
        }
        return result;
    }

    index = 0;
    for (;;) {
        size_t position;
        bool found =
            find_separator(source, source_len, index, sep, sep_len, &position);
        count += 1;
        if (!found) {
            break;
        }
        index = position + sep_len;
    }
    result = aleph_list_new(count);
    index = 0;
    count = 0;
    for (;;) {
        size_t position;
        bool found =
            find_separator(source, source_len, index, sep, sep_len, &position);
        result.u.list->items[count++] =
            string_from_bytes(source + index, position - index);
        if (!found) {
            break;
        }
        index = position + sep_len;
    }
    return result;
}

AlValue aleph_b_join(AlValue parts, AlValue separator) {
    size_t index;
    size_t total = 0;
    size_t sep_len = separator.u.str->len;
    char *data;
    char *position;
    AlValue value;

    for (index = 0; index < parts.u.list->len; index += 1) {
        size_t part_len = parts.u.list->items[index].u.str->len;
        if (part_len > SIZE_MAX - total) {
            out_of_memory();
        }
        total += part_len;
        if (index + 1 < parts.u.list->len) {
            if (sep_len > SIZE_MAX - total) {
                out_of_memory();
            }
            total += sep_len;
        }
    }
    if (total == SIZE_MAX) {
        out_of_memory();
    }
    data = (char *)aleph_alloc(total + 1);
    position = data;
    for (index = 0; index < parts.u.list->len; index += 1) {
        const AlStr *part = parts.u.list->items[index].u.str;
        memcpy(position, part->data, part->len);
        position += part->len;
        if (index + 1 < parts.u.list->len) {
            memcpy(position, separator.u.str->data, sep_len);
            position += sep_len;
        }
    }
    *position = '\0';
    {
        AlStr *string = (AlStr *)aleph_alloc(sizeof(AlStr));
        string->len = total;
        string->data = data;
        value.tag = AL_STR;
        value.u.str = string;
    }
    return value;
}

static bool unicode_whitespace(uint32_t codepoint) {
    if (codepoint >= 0x0009 && codepoint <= 0x000D) {
        return true;
    }
    if (codepoint == 0x0020 || codepoint == 0x0085 || codepoint == 0x00A0 ||
        codepoint == 0x1680) {
        return true;
    }
    if (codepoint >= 0x2000 && codepoint <= 0x200A) {
        return true;
    }
    return codepoint == 0x2028 || codepoint == 0x2029 || codepoint == 0x202F ||
           codepoint == 0x205F || codepoint == 0x3000;
}

AlValue aleph_b_trim(AlValue text) {
    const unsigned char *source = (const unsigned char *)text.u.str->data;
    size_t len = text.u.str->len;
    size_t index = 0;
    size_t start = 0;
    size_t end = 0;
    bool seen = false;

    while (index < len) {
        uint32_t codepoint;
        size_t width = utf8_decode(source + index, len - index, &codepoint);
        if (width == 0) {
            width = 1;
            codepoint = source[index];
        }
        if (!unicode_whitespace(codepoint)) {
            if (!seen) {
                start = index;
                seen = true;
            }
            end = index + width;
        }
        index += width;
    }
    if (!seen) {
        return aleph_str_n("", 0);
    }
    return aleph_str_n(text.u.str->data + start, end - start);
}

static AlValue ascii_case(AlValue text, bool upper) {
    size_t index;
    char *data;
    if (text.u.str->len == SIZE_MAX) {
        out_of_memory();
    }
    data = (char *)aleph_alloc(text.u.str->len + 1);
    for (index = 0; index < text.u.str->len; index += 1) {
        unsigned char byte = (unsigned char)text.u.str->data[index];
        if (upper && byte >= 'a' && byte <= 'z') {
            byte = (unsigned char)(byte - 'a' + 'A');
        } else if (!upper && byte >= 'A' && byte <= 'Z') {
            byte = (unsigned char)(byte - 'A' + 'a');
        }
        data[index] = (char)byte;
    }
    data[text.u.str->len] = '\0';
    {
        AlValue value;
        value.tag = AL_STR;
        value.u.str = (AlStr *)aleph_alloc(sizeof(AlStr));
        ((AlStr *)value.u.str)->len = text.u.str->len;
        ((AlStr *)value.u.str)->data = data;
        return value;
    }
}

AlValue aleph_b_upper(AlValue text) { return ascii_case(text, true); }
AlValue aleph_b_lower(AlValue text) { return ascii_case(text, false); }

static const char *find_bytes(const char *haystack, size_t haystack_len,
                              const char *needle, size_t needle_len) {
    size_t index;
    if (needle_len == 0) {
        return haystack;
    }
    if (needle_len > haystack_len) {
        return NULL;
    }
    for (index = 0; index + needle_len <= haystack_len; index += 1) {
        if (memcmp(haystack + index, needle, needle_len) == 0) {
            return haystack + index;
        }
    }
    return NULL;
}

AlValue aleph_b_contains(AlValue text, AlValue needle) {
    return aleph_bool(find_bytes(text.u.str->data, text.u.str->len,
                                 needle.u.str->data, needle.u.str->len) != NULL);
}

AlValue aleph_b_show(AlValue value) { return aleph_show(value); }

/* ------------------------------------------------------------------ */
/* Parsing                                                             */
/* ------------------------------------------------------------------ */

static bool parse_int_value(const char *text, size_t len, int64_t *out) {
    size_t index = 0;
    bool negative = false;
    uint64_t magnitude = 0;
    uint64_t limit;

    if (len == 0) {
        return false;
    }
    if (text[0] == '-') {
        negative = true;
        index = 1;
    }
    if (index >= len) {
        return false;
    }
    limit = negative ? ((uint64_t)INT64_MAX + 1u) : (uint64_t)INT64_MAX;
    for (; index < len; index += 1) {
        unsigned char byte = (unsigned char)text[index];
        uint64_t digit;
        if (byte < '0' || byte > '9') {
            return false;
        }
        digit = (uint64_t)(byte - '0');
        if (magnitude > (limit - digit) / 10u) {
            return false;
        }
        magnitude = magnitude * 10u + digit;
    }
    if (negative) {
        if (magnitude == (uint64_t)INT64_MAX + 1u) {
            *out = INT64_MIN;
        } else {
            *out = -(int64_t)magnitude;
        }
    } else {
        *out = (int64_t)magnitude;
    }
    return true;
}

static bool parse_float_value(const char *text, size_t len, double *out) {
    size_t index = 0;
    size_t dot = SIZE_MAX;
    size_t digits = 0;
    char *copy;
    char *end = NULL;
    double value;

    if (len == 0) {
        return false;
    }
    if (text[0] == '-') {
        index = 1;
    }
    for (; index < len; index += 1) {
        unsigned char byte = (unsigned char)text[index];
        if (byte == '.') {
            if (dot != SIZE_MAX) {
                return false;
            }
            dot = index;
        } else if (byte >= '0' && byte <= '9') {
            digits += 1;
        } else {
            return false;
        }
    }
    if (dot == SIZE_MAX || dot == 0 || dot + 1 >= len ||
        (text[0] == '-' && dot == 1)) {
        return false;
    }
    if (digits != len - (text[0] == '-' ? 1u : 0u) - 1u) {
        return false;
    }
    if (len == SIZE_MAX) {
        out_of_memory();
    }
    copy = (char *)malloc(len + 1);
    if (copy == NULL) {
        out_of_memory();
    }
    memcpy(copy, text, len);
    copy[len] = '\0';
    value = strtod(copy, &end);
    if (end != copy + len) {
        free(copy);
        return false;
    }
    free(copy);
    *out = value;
    return true;
}

AlValue aleph_b_int(AlValue text) {
    int64_t number;
    if (!parse_int_value(text.u.str->data, text.u.str->len, &number)) {
        return aleph_err(aleph_str("invalid int"));
    }
    return aleph_ok(aleph_i64(number));
}

AlValue aleph_b_float(AlValue text) {
    double number;
    if (!parse_float_value(text.u.str->data, text.u.str->len, &number)) {
        return aleph_err(aleph_str("invalid float"));
    }
    return aleph_ok(aleph_f64(number));
}

/* ------------------------------------------------------------------ */
/* Effects                                                             */
/* ------------------------------------------------------------------ */

AlValue aleph_b_args(void) {
    size_t index;
    AlValue result = aleph_list_new((size_t)(g_argc > 0 ? g_argc : 0));
    for (index = 0; index < result.u.list->len; index += 1) {
        const char *argument = g_argv != NULL ? g_argv[index] : "";
        result.u.list->items[index] =
            aleph_str(argument != NULL ? argument : "");
    }
    return result;
}

AlValue aleph_b_print(AlValue text) {
    if (text.u.str->len > 0) {
        fwrite(text.u.str->data, 1, text.u.str->len, stdout);
    }
    fputc('\n', stdout);
    return aleph_unit();
}

AlValue aleph_b_eprint(AlValue text) {
    fflush(stdout);
    if (text.u.str->len > 0) {
        fwrite(text.u.str->data, 1, text.u.str->len, stderr);
    }
    fputc('\n', stderr);
    fflush(stderr);
    return aleph_unit();
}

/* ------------------------------------------------------------------ */
/* Capability paths (ADR-0020/ADR-0021)                                */
/* ------------------------------------------------------------------ */

typedef struct Segment {
    const char *ptr;
    size_t len;
} Segment;

static bool resolve_lenient(const char *path, char **out) {
    size_t length;
    size_t index;
    size_t count = 0;
    bool absolute;
    Segment *segments;
    struct stat metadata;
    char *resolved;

    *out = NULL;
    if (path == NULL || path[0] == '\0') {
        return false;
    }
    length = strlen(path);
    absolute = path[0] == '/';
    if (length == SIZE_MAX ||
        length + 1 > SIZE_MAX / sizeof(Segment)) {
        out_of_memory();
    }
    segments = (Segment *)malloc((length + 1) * sizeof(Segment));
    if (segments == NULL) {
        out_of_memory();
    }

    index = 0;
    while (index < length) {
        size_t start;
        size_t size;
        while (index < length && path[index] == '/') {
            index += 1;
        }
        start = index;
        while (index < length && path[index] != '/') {
            index += 1;
        }
        size = index - start;
        if (size == 0) {
            break;
        }
        if (size == 1 && path[start] == '.') {
            continue;
        }
        if (size == 2 && path[start] == '.' && path[start + 1] == '.') {
            free(segments);
            return false;
        }
        segments[count].ptr = path + start;
        segments[count].len = size;
        count += 1;
    }

    if (lstat(path, &metadata) == 0) {
        resolved = realpath(path, NULL);
        free(segments);
        if (resolved == NULL) {
            return false;
        }
        *out = resolved;
        return true;
    }

    for (length = count; length > 0;) {
        Buf ancestor = {NULL, 0, 0};
        Buf result = {NULL, 0, 0};
        size_t position;
        length -= 1;

        if (absolute) {
            buf_putc(&ancestor, '/');
        }
        if (length == 0) {
            if (!absolute) {
                buf_putc(&ancestor, '.');
            }
        } else {
            for (index = 0; index < length; index += 1) {
                if (index > 0) {
                    buf_putc(&ancestor, '/');
                }
                buf_write(&ancestor, segments[index].ptr, segments[index].len);
            }
        }
        buf_finish(&ancestor);

        if (lstat(ancestor.data, &metadata) != 0) {
            free(ancestor.data);
            continue;
        }
        resolved = realpath(ancestor.data, NULL);
        free(ancestor.data);
        if (resolved == NULL) {
            free(segments);
            return false;
        }

        buf_cstr(&result, resolved);
        free(resolved);
        for (position = length; position < count; position += 1) {
            if (result.len == 0 || result.data[result.len - 1] != '/') {
                buf_putc(&result, '/');
            }
            buf_write(&result, segments[position].ptr, segments[position].len);
        }
        buf_finish(&result);
        free(segments);
        *out = result.data;
        return true;
    }

    free(segments);
    return false;
}

static bool path_has_nul(const char *path, size_t len) {
    return memchr(path, '\0', len) != NULL;
}

static bool path_has_parent_component(const char *path, size_t len) {
    size_t index = 0;
    while (index < len) {
        size_t start;
        size_t size;
        while (index < len && path[index] == '/') {
            index += 1;
        }
        start = index;
        while (index < len && path[index] != '/') {
            index += 1;
        }
        size = index - start;
        if (size == 2 && path[start] == '.' && path[start + 1] == '.') {
            return true;
        }
    }
    return false;
}

/*
 * Resolve `path[0..len)` like the interpreter, preserving embedded NUL bytes.
 *
 * A NUL cannot be passed to the operating system, so Rust's lstat fails for
 * such a path and its fallback canonicalizes the longest existing ancestor
 * before appending the remaining components verbatim. We do the same by
 * resolving the bytes before the component containing the first NUL.
 */
static bool resolve_lenient_bytes(const char *path, size_t len, char **out,
                                  size_t *out_len) {
    size_t first_nul;
    size_t component_start;
    size_t base_len;
    char *base;
    char *resolved;
    char *result;
    size_t resolved_len;
    size_t result_len;

    *out = NULL;
    *out_len = 0;
    if (path == NULL || len == 0) {
        return false;
    }
    if (!path_has_nul(path, len)) {
        char *copy;
        bool ok;
        if (len == SIZE_MAX) {
            out_of_memory();
        }
        copy = (char *)malloc(len + 1);
        if (copy == NULL) {
            out_of_memory();
        }
        memcpy(copy, path, len);
        copy[len] = '\0';
        ok = resolve_lenient(copy, &resolved);
        free(copy);
        if (!ok) {
            return false;
        }
        *out = resolved;
        *out_len = strlen(resolved);
        return true;
    }
    if (path_has_parent_component(path, len)) {
        return false;
    }

    first_nul = 0;
    while (path[first_nul] != '\0') {
        first_nul += 1;
    }
    component_start = first_nul;
    while (component_start > 0 && path[component_start - 1] != '/') {
        component_start -= 1;
    }

    base_len = component_start;
    if (base_len > SIZE_MAX - 2) {
        out_of_memory();
    }
    base = (char *)malloc(base_len + 2);
    if (base == NULL) {
        out_of_memory();
    }
    if (base_len == 0) {
        base[0] = '.';
        base[1] = '\0';
        base_len = 1;
    } else {
        memcpy(base, path, base_len);
        base[base_len] = '\0';
    }
    if (!resolve_lenient(base, &resolved)) {
        free(base);
        return false;
    }
    free(base);

    resolved_len = strlen(resolved);
    if (resolved_len > SIZE_MAX - (len - component_start) - 2) {
        free(resolved);
        out_of_memory();
    }
    result_len = resolved_len + 1 + (len - component_start);
    result = (char *)malloc(result_len + 1);
    if (result == NULL) {
        free(resolved);
        out_of_memory();
    }
    memcpy(result, resolved, resolved_len);
    free(resolved);
    result_len = resolved_len;
    if (result_len == 0 || result[result_len - 1] != '/') {
        result[result_len++] = '/';
    }
    memcpy(result + result_len, path + component_start,
           len - component_start);
    result_len += len - component_start;
    result[result_len] = '\0';
    *out = result;
    *out_len = result_len;
    return true;
}

static bool path_starts_with_bytes(const char *target, size_t target_len,
                                   const char *prefix, size_t prefix_len) {
    const char *t = target;
    const char *p = prefix;
    size_t tn = target_len;
    size_t pn = prefix_len;

    if (tn == 0 || pn == 0 || t[0] != '/' || p[0] != '/') {
        return tn == pn && (tn == 0 || memcmp(t, p, tn) == 0);
    }
    for (;;) {
        const char *t_end;
        const char *p_end;
        size_t t_len;
        size_t p_len;
        while (tn > 0 && *t == '/') {
            t += 1;
            tn -= 1;
        }
        while (pn > 0 && *p == '/') {
            p += 1;
            pn -= 1;
        }
        if (pn == 0) {
            return true;
        }
        if (tn == 0) {
            return false;
        }
        t_end = (const char *)memchr(t, '/', tn);
        p_end = (const char *)memchr(p, '/', pn);
        t_len = t_end != NULL ? (size_t)(t_end - t) : tn;
        p_len = p_end != NULL ? (size_t)(p_end - p) : pn;
        if (t_len != p_len || memcmp(t, p, t_len) != 0) {
            return false;
        }
        t += t_len;
        tn -= t_len;
        p += p_len;
        pn -= p_len;
    }
}

static bool path_allowed_bytes(AlCap cap, const char *path, size_t len) {
    char *target = NULL;
    size_t target_len = 0;
    size_t index;
    bool allowed = false;

    if (!resolve_lenient_bytes(path, len, &target, &target_len)) {
        return false;
    }
    for (index = 0; index < g_use_count; index += 1) {
        char *prefix = NULL;
        size_t prefix_len = 0;
        if (g_uses[index].cap != cap || g_uses[index].path == NULL) {
            continue;
        }
        if (resolve_lenient_bytes(g_uses[index].path,
                                  strlen(g_uses[index].path), &prefix,
                                  &prefix_len)) {
            if (path_starts_with_bytes(target, target_len, prefix, prefix_len)) {
                allowed = true;
            }
            free(prefix);
        }
    }
    free(target);
    return allowed;
}

bool aleph_path_allowed(AlCap cap, const char *path) {
    if (path == NULL) {
        return false;
    }
    return path_allowed_bytes(cap, path, strlen(path));
}

static char *string_to_cstr(const AlStr *string) {
    char *copy;
    if (string->len == SIZE_MAX) {
        out_of_memory();
    }
    copy = (char *)malloc(string->len + 1);
    if (copy == NULL) {
        out_of_memory();
    }
    memcpy(copy, string->data, string->len);
    copy[string->len] = '\0';
    return copy;
}

static AlValue io_error_value(int error) {
    const char *text;
    switch (error) {
    case ENOENT:
        text = "not found";
        break;
    case EACCES:
    case EPERM:
        text = "permission denied";
        break;
    case EEXIST:
        text = "already exists";
        break;
    case EINVAL:
        text = "invalid input";
        break;
    case EISDIR:
        text = "is a directory";
        break;
    case ENOTDIR:
        text = "not a directory";
        break;
    case ENOTEMPTY:
        text = "directory not empty";
        break;
    case EROFS:
        text = "read-only filesystem";
        break;
    case EAGAIN:
        text = "would block";
        break;
    case ENOMEM:
        text = "out of memory";
        break;
    default:
        text = "io error";
        break;
    }
    return aleph_err(aleph_str(text));
}

AlValue aleph_b_read(AlValue path_value) {
    char *path;
    char *buffer = NULL;
    size_t used = 0;
    size_t capacity = 0;
    int fd;
    AlValue result;

    if (!path_allowed_bytes(AL_CAP_FS_READ, path_value.u.str->data,
                            path_value.u.str->len)) {
        aleph_trap("R003", "capability violation");
    }
    if (memchr(path_value.u.str->data, '\0', path_value.u.str->len) != NULL) {
        return aleph_err(aleph_str("invalid input"));
    }
    path = string_to_cstr(path_value.u.str);
    fd = open(path, O_RDONLY);
    free(path);
    if (fd < 0) {
        return io_error_value(errno);
    }
    for (;;) {
        ssize_t count;
        if (used == capacity) {
            size_t next;
            char *grown;
            if (capacity == 0) {
                next = 8192;
            } else {
                if (capacity > SIZE_MAX / 2) {
                    free(buffer);
                    close(fd);
                    out_of_memory();
                }
                next = capacity * 2;
            }
            grown = (char *)realloc(buffer, next);
            if (grown == NULL) {
                free(buffer);
                close(fd);
                out_of_memory();
            }
            buffer = grown;
            capacity = next;
        }
        count = read(fd, buffer + used, capacity - used);
        if (count < 0) {
            int error = errno;
            if (error == EINTR) {
                continue;
            }
            free(buffer);
            close(fd);
            return io_error_value(error);
        }
        if (count == 0) {
            break;
        }
        used += (size_t)count;
    }
    close(fd);
    if (!utf8_valid((const unsigned char *)buffer, used)) {
        free(buffer);
        return aleph_err(aleph_str("invalid data"));
    }
    result = aleph_ok(aleph_str_n(buffer != NULL ? buffer : "", used));
    free(buffer);
    return result;
}

AlValue aleph_b_write(AlValue path_value, AlValue text) {
    char *path;
    const char *data = text.u.str->data;
    size_t len = text.u.str->len;
    size_t written = 0;
    int fd;

    if (!path_allowed_bytes(AL_CAP_FS_WRITE, path_value.u.str->data,
                            path_value.u.str->len)) {
        aleph_trap("R003", "capability violation");
    }
    if (memchr(path_value.u.str->data, '\0', path_value.u.str->len) != NULL) {
        return aleph_err(aleph_str("invalid input"));
    }
    path = string_to_cstr(path_value.u.str);
    fd = open(path, O_WRONLY | O_CREAT | O_TRUNC, 0666);
    free(path);
    if (fd < 0) {
        return io_error_value(errno);
    }
    while (written < len) {
        ssize_t count = write(fd, data + written, len - written);
        if (count < 0) {
            int error = errno;
            if (error == EINTR) {
                continue;
            }
            close(fd);
            return io_error_value(error);
        }
        if (count == 0) {
            close(fd);
            return aleph_err(aleph_str("io error"));
        }
        written += (size_t)count;
    }
    (void)close(fd);
    return aleph_ok(aleph_unit());
}

/* ------------------------------------------------------------------ */
/* Entry point                                                         */
/* ------------------------------------------------------------------ */

typedef struct MainContext {
    AlValue (*code)(AlValue *, size_t);
    AlValue value;
} MainContext;

static void *main_thread(void *argument) {
    MainContext *context = (MainContext *)argument;
    unsigned char stack_base;

    g_stack_base = (uintptr_t)&stack_base;
    g_stack_base_set = true;
    context->value = context->code(NULL, 0);
    return NULL;
}

static AL_NORETURN void main_thread_failure(void) {
    fputs("aleph: main thread failure\n", stderr);
    fflush(stderr);
    exit(70);
}

int aleph_main_run(AlValue (*main_code)(AlValue *, size_t), int argc,
                   const char *const *argv) {
    pthread_attr_t attributes;
    pthread_t thread;
    MainContext context;

    setlocale(LC_ALL, "C");
    setlocale(LC_NUMERIC, "C");
    signal(SIGPIPE, SIG_IGN);
    aleph_set_args(argc, argv);

    context.code = main_code;
    context.value = aleph_unit();
    if (pthread_attr_init(&attributes) != 0) {
        main_thread_failure();
    }
    if (pthread_attr_setstacksize(&attributes, (size_t)AL_MAIN_STACK_SIZE) !=
        0) {
        pthread_attr_destroy(&attributes);
        main_thread_failure();
    }
    if (pthread_create(&thread, &attributes, main_thread, &context) != 0) {
        pthread_attr_destroy(&attributes);
        main_thread_failure();
    }
    pthread_attr_destroy(&attributes);
    if (pthread_join(thread, NULL) != 0) {
        main_thread_failure();
    }

    fflush(stdout);
    fflush(stderr);
    if (context.value.tag == AL_ERR) {
        AlValue error = context.value.u.box->value;
        if (error.tag == AL_STR) {
            if (error.u.str->len > 0) {
                fwrite(error.u.str->data, 1, error.u.str->len, stderr);
            }
        } else {
            AlValue shown = aleph_show(error);
            if (shown.u.str->len > 0) {
                fwrite(shown.u.str->data, 1, shown.u.str->len, stderr);
            }
        }
        fputc('\n', stderr);
        fflush(stderr);
        return 1;
    }
    return 0;
}
