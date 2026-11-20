#define _XOPEN_SOURCE 700 /* POSIX 2008 plus realpath (not declared by _POSIX_C_SOURCE alone) */

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
#include <poll.h>
#include <pthread.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

/* ------------------------------------------------------------------ */
/* Global state                                                        */
/* ------------------------------------------------------------------ */

static const char *g_file = "";
static const AlUse *g_uses;
static size_t g_use_count;
static int g_argc;
static const char *const *g_argv;

/* Depth and location state; shared with the inline fast-path helpers in
 * tlon.h (ADR-0027). */
size_t aleph_g_eval_depth;
size_t aleph_g_call_depth;

/* Native stack guard for the big main thread (set in main_thread). */
uintptr_t aleph_g_stack_base;
bool aleph_g_stack_base_set;

static const AlLoc g_default_loc = {0, 0, 0};
const AlLoc *aleph_g_locs[AL_MAX_EVAL_DEPTH + 1];
size_t aleph_g_loc_depth;

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
    if (aleph_g_loc_depth > 0 && aleph_g_locs[aleph_g_loc_depth - 1] != NULL) {
        return aleph_g_locs[aleph_g_loc_depth - 1];
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

#define AL_CHUNK_MIN (64u * 1024u)
#define AL_ALIGN 16u
#define AL_ROUND(n) (((n) + (AL_ALIGN - 1)) & ~(size_t)(AL_ALIGN - 1))

AlChunk *aleph_g_chunks;
uint64_t aleph_g_live_base;
uint64_t aleph_g_freed;
/* One freed AL_CHUNK_MIN chunk kept for reuse, so a call that crosses a
   chunk boundary does not malloc/free on every iteration. */
static AlChunk *g_spare;

/* An unlinked chunk with room for `cap` bytes, or NULL.  Unless `exact`,
   it holds at least AL_CHUNK_MIN and may be the spare. */
static AlChunk *chunk_new(size_t cap, bool exact) {
    size_t header = sizeof(AlChunk) + (AL_ALIGN - 1);
    unsigned char *base;
    AlChunk *chunk;
    if (!exact && cap <= AL_CHUNK_MIN && g_spare != NULL) {
        chunk = g_spare;
        g_spare = NULL;
        chunk->used = 0;
        return chunk;
    }
    if (!exact && cap < AL_CHUNK_MIN) {
        cap = AL_CHUNK_MIN;
    }
    if (cap > SIZE_MAX - header) {
        return NULL;
    }
    chunk = (AlChunk *)malloc(header + cap);
    if (chunk == NULL) {
        return NULL;
    }
    base = (unsigned char *)chunk + sizeof(AlChunk);
    chunk->next = NULL;
    chunk->used = 0;
    chunk->cap = cap;
    chunk->data = (unsigned char *)(((uintptr_t)base + (AL_ALIGN - 1)) &
                                    ~(uintptr_t)(AL_ALIGN - 1));
    return chunk;
}

static void chunk_release(AlChunk *chunk) {
    if (g_spare == NULL && chunk->cap == AL_CHUNK_MIN) {
        g_spare = chunk;
    } else {
        free(chunk);
    }
}

static uint64_t live_bytes(void) {
    return aleph_g_live_base + (aleph_g_chunks != NULL ? aleph_g_chunks->used : 0);
}

static void push_head(AlChunk *chunk) {
    if (aleph_g_chunks != NULL) {
        aleph_g_live_base += aleph_g_chunks->used;
    }
    chunk->next = aleph_g_chunks;
    aleph_g_chunks = chunk;
}

void *aleph_alloc(size_t size) {
    AlChunk *chunk;
    void *result;
    if (size == 0) {
        size = 1;
    }
    if (size > SIZE_MAX - (AL_ALIGN - 1)) {
        out_of_memory();
    }
    size = AL_ROUND(size);
    chunk = aleph_g_chunks;
    if (chunk == NULL || chunk->cap - chunk->used < size) {
        chunk = chunk_new(size, false);
        if (chunk == NULL) {
            out_of_memory();
        }
        push_head(chunk);
    }
    result = chunk->data + chunk->used;
    chunk->used += size;
    return result;
}

size_t aleph_arena_bytes(void) {
    size_t total = 0;
    const AlChunk *chunk;
    for (chunk = aleph_g_chunks; chunk != NULL; chunk = chunk->next) {
        total += chunk->cap;
    }
    return total;
}

/* ------------------------------------------------------------------ */
/* Scoped regions (ADR-0039)                                           */
/* ------------------------------------------------------------------ */

void aleph_region_free_to(AlMark mark) {
    uint64_t before = live_bytes();
    while (aleph_g_chunks != mark.chunk && aleph_g_chunks != NULL) {
        AlChunk *chunk = aleph_g_chunks;
        aleph_g_chunks = chunk->next;
        chunk_release(chunk);
    }
    if (mark.chunk != NULL) {
        mark.chunk->used = mark.used;
    }
    aleph_g_live_base = mark.base;
    aleph_g_freed += before - live_bytes();
}

typedef struct {
    uintptr_t lo;
    uintptr_t hi;
} AlRange;

typedef struct {
    const void *old;
    void *copy;
} AlFwd;

/* State of one compaction: the region's chunk ranges (sorted), a
   forwarding table keyed by object address and an explicit work stack. */
typedef struct {
    AlRange *ranges;
    size_t nranges;
    AlFwd *table;
    size_t tcap;
    size_t tlen;
    void **stack;
    size_t slen;
    size_t scap;
    unsigned char *bump;
} AlCompact;

static int range_cmp(const void *a, const void *b) {
    uintptr_t x = ((const AlRange *)a)->lo;
    uintptr_t y = ((const AlRange *)b)->lo;
    return x < y ? -1 : x > y ? 1 : 0;
}

static bool in_region(const AlCompact *c, const void *ptr) {
    uintptr_t p = (uintptr_t)ptr;
    size_t lo = 0;
    size_t hi = c->nranges;
    while (lo < hi) {
        size_t mid = lo + (hi - lo) / 2;
        if (p < c->ranges[mid].lo) {
            hi = mid;
        } else if (p >= c->ranges[mid].hi) {
            lo = mid + 1;
        } else {
            return true;
        }
    }
    return false;
}

static size_t fwd_slot(const AlCompact *c, const void *old) {
    size_t mask = c->tcap - 1;
    size_t index = (size_t)(((uintptr_t)old >> 4) * (uintptr_t)0x9E3779B97F4A7C15ull) & mask;
    while (c->table[index].old != NULL && c->table[index].old != old) {
        index = (index + 1) & mask;
    }
    return index;
}

/* Inserts `old`; returns 1 if new, 0 if already present, -1 on failure. */
static int fwd_insert(AlCompact *c, const void *old) {
    size_t index;
    if (2 * (c->tlen + 1) > c->tcap) {
        size_t cap = c->tcap * 2;
        AlFwd *table = (AlFwd *)calloc(cap, sizeof(AlFwd));
        AlFwd *previous = c->table;
        size_t pcap = c->tcap;
        size_t i;
        if (table == NULL) {
            return -1;
        }
        c->table = table;
        c->tcap = cap;
        for (i = 0; i < pcap; i += 1) {
            if (previous[i].old != NULL) {
                c->table[fwd_slot(c, previous[i].old)] = previous[i];
            }
        }
        free(previous);
    }
    index = fwd_slot(c, old);
    if (c->table[index].old != NULL) {
        return 0;
    }
    c->table[index].old = old;
    c->table[index].copy = NULL;
    c->tlen += 1;
    return 1;
}

static bool push(AlCompact *c, void *item) {
    if (c->slen == c->scap) {
        size_t cap = c->scap * 2;
        void **stack = (void **)realloc(c->stack, cap * sizeof(void *));
        if (stack == NULL) {
            return false;
        }
        c->stack = stack;
        c->scap = cap;
    }
    c->stack[c->slen++] = item;
    return true;
}

/* Pushes the pointer-holding values of `items[0..n)`. */
static bool push_values(AlCompact *c, AlValue *items, size_t n) {
    size_t i;
    for (i = 0; i < n; i += 1) {
        if (aleph_value_root(items[i]) != NULL && !push(c, &items[i])) {
            return false;
        }
    }
    return true;
}

/* Size in bytes of the object a value points to, without the separate
   string data or captured array. */
static size_t object_size(AlValue value) {
    switch (value.tag) {
    case AL_STR:
        return AL_ROUND(sizeof(AlStr));
    case AL_LIST:
    case AL_DICT:
        return AL_ROUND(sizeof(AlList) + value.u.list->len * sizeof(AlValue));
    case AL_JSON:
        return AL_ROUND(sizeof(AlJson));
    case AL_RECORD:
        return AL_ROUND(sizeof(AlRecord) + value.u.rec->n * sizeof(AlValue));
    case AL_FN:
        return AL_ROUND(sizeof(AlFn));
    default:
        return AL_ROUND(sizeof(AlBox));
    }
}

/* Bytes the separate allocation of a value (string data, captured values)
   takes when it lies in the region, else 0. */
static size_t extra_size(const AlCompact *c, AlValue value) {
    if (value.tag == AL_STR && in_region(c, value.u.str->data)) {
        return AL_ROUND(value.u.str->len + 1);
    }
    if (value.tag == AL_FN && value.u.fn->ncaptured > 0 &&
        in_region(c, value.u.fn->captured)) {
        return AL_ROUND(value.u.fn->ncaptured * sizeof(AlValue));
    }
    return 0;
}

/* The values directly inside the object a value points to. */
static AlValue *children(AlValue value, size_t *n) {
    switch (value.tag) {
    case AL_LIST:
    case AL_DICT:
        *n = value.u.list->len;
        return value.u.list->items;
    case AL_SOME:
    case AL_OK:
    case AL_ERR:
    case AL_JSON:
        *n = 1;
        return &value.u.box->value;
    case AL_RECORD:
        *n = value.u.rec->n;
        return value.u.rec->fields;
    case AL_FN:
        *n = value.u.fn->ncaptured;
        return (AlValue *)value.u.fn->captured;
    default:
        *n = 0;
        return NULL;
    }
}

/* Bytes of the region reachable from `root`, each object counted once;
   SIZE_MAX if past `limit` or out of memory.  Iterative. */
static size_t region_reach(AlCompact *c, AlValue *root, size_t limit) {
    size_t total = 0;
    c->slen = 0;
    if (!push(c, root)) {
        return SIZE_MAX;
    }
    while (c->slen > 0) {
        AlValue value = *(AlValue *)c->stack[--c->slen];
        const void *object = aleph_value_root(value);
        size_t n;
        AlValue *items;
        int fresh;
        if (!in_region(c, object)) {
            continue;
        }
        fresh = fwd_insert(c, object);
        if (fresh < 0) {
            return SIZE_MAX;
        }
        if (fresh == 0) {
            continue;
        }
        total += object_size(value) + extra_size(c, value);
        if (total > limit) {
            return SIZE_MAX;
        }
        items = children(value, &n);
        if (!push_values(c, items, n)) {
            return SIZE_MAX;
        }
    }
    return total;
}

static void *bump(AlCompact *c, const void *from, size_t size) {
    void *to = c->bump;
    memcpy(to, from, size);
    c->bump += AL_ROUND(size);
    return to;
}

/* Copies one object into the new chunk and points the value at it. */
static void *copy_object(AlCompact *c, AlValue *slot) {
    size_t extra = extra_size(c, *slot);
    switch (slot->tag) {
    case AL_STR: {
        AlStr *str = (AlStr *)bump(c, slot->u.str, sizeof(AlStr));
        if (extra > 0) {
            /* Tlön strings own NUL-terminated data. */
            str->data = (const char *)bump(c, str->data, str->len + 1);
        }
        slot->u.str = str;
        return str;
    }
    case AL_LIST:
    case AL_DICT:
        slot->u.list = (AlList *)bump(c, slot->u.list, object_size(*slot));
        return slot->u.list;
    case AL_JSON:
        slot->u.box = (AlBox *)bump(c, slot->u.box, sizeof(AlJson));
        return slot->u.box;
    case AL_RECORD:
        slot->u.rec = (AlRecord *)bump(c, slot->u.rec, object_size(*slot));
        return slot->u.rec;
    case AL_FN: {
        AlFn *fn = (AlFn *)bump(c, slot->u.fn, sizeof(AlFn));
        if (extra > 0) {
            fn->captured = (const AlValue *)bump(
                c, fn->captured, fn->ncaptured * sizeof(AlValue));
        }
        slot->u.fn = fn;
        return fn;
    }
    default:
        slot->u.box = (AlBox *)bump(c, slot->u.box, sizeof(AlBox));
        return slot->u.box;
    }
}

/* Deep-copies the region objects reachable from `root` (all already in the
   forwarding table) into c->bump.  Iterative. */
static bool region_copy(AlCompact *c, AlValue *root) {
    c->slen = 0;
    if (!push(c, root)) {
        return false;
    }
    while (c->slen > 0) {
        AlValue *slot = (AlValue *)c->stack[--c->slen];
        const void *object = aleph_value_root(*slot);
        AlFwd *entry;
        size_t n;
        AlValue *items;
        if (!in_region(c, object)) {
            continue;
        }
        entry = &c->table[fwd_slot(c, object)];
        if (entry->copy != NULL) {
            switch (slot->tag) {
            case AL_STR:
                slot->u.str = (const AlStr *)entry->copy;
                break;
            case AL_LIST:
            case AL_DICT:
                slot->u.list = (AlList *)entry->copy;
                break;
            case AL_RECORD:
                slot->u.rec = (AlRecord *)entry->copy;
                break;
            case AL_FN:
                slot->u.fn = (const AlFn *)entry->copy;
                break;
            default:
                slot->u.box = (AlBox *)entry->copy;
                break;
            }
            continue;
        }
        entry->copy = copy_object(c, slot);
        items = children(*slot, &n);
        if (!push_values(c, items, n)) {
            return false;
        }
    }
    return true;
}

/* Copies the result out of a large region when it is small (rule 2);
   returns false, with the arena untouched, when it does not pay. */
static bool region_compact(AlMark mark, size_t total, AlValue *result) {
    AlCompact c;
    AlChunk *chunk;
    AlChunk *fresh = NULL;
    AlValue root = *result;
    size_t i = 0;
    size_t bytes;
    bool done = false;
    memset(&c, 0, sizeof(c));
    for (chunk = aleph_g_chunks; chunk != mark.chunk; chunk = chunk->next) {
        c.nranges += 1;
    }
    c.nranges += mark.chunk != NULL ? 1 : 0;
    c.ranges = (AlRange *)malloc(c.nranges * sizeof(AlRange));
    c.tcap = 1024;
    c.table = (AlFwd *)calloc(c.tcap, sizeof(AlFwd));
    c.scap = 256;
    c.stack = (void **)malloc(c.scap * sizeof(void *));
    if (c.ranges == NULL || c.table == NULL || c.stack == NULL) {
        goto out;
    }
    for (chunk = aleph_g_chunks; chunk != mark.chunk; chunk = chunk->next) {
        c.ranges[i].lo = (uintptr_t)chunk->data;
        c.ranges[i].hi = (uintptr_t)(chunk->data + chunk->used);
        i += 1;
    }
    if (mark.chunk != NULL) {
        c.ranges[i].lo = (uintptr_t)(mark.chunk->data + mark.used);
        c.ranges[i].hi = (uintptr_t)(mark.chunk->data + mark.chunk->used);
    }
    qsort(c.ranges, c.nranges, sizeof(AlRange), range_cmp);
    bytes = region_reach(&c, &root, total / 4);
    if (bytes == SIZE_MAX) {
        goto out;
    }
    /* Exact size: a 64 KiB chunk here would stay alive, mostly empty, under
       the caller's next marks.  The next allocation takes a new chunk. */
    fresh = chunk_new(bytes, true);
    if (fresh == NULL) {
        goto out;
    }
    c.bump = fresh->data;
    if (!region_copy(&c, &root)) {
        chunk_release(fresh);
        goto out;
    }
    fresh->used = (size_t)(c.bump - fresh->data);
    aleph_region_free_to(mark);
    push_head(fresh);
    *result = root;
    done = true;
out:
    free(c.ranges);
    free(c.table);
    free(c.stack);
    return done;
}

/* Whether `p` lies in the region opened by `mark`.  The usual case, a root
   in the newest chunk, needs no walk. */
static bool region_holds(AlMark mark, uintptr_t p) {
    const AlChunk *chunk = aleph_g_chunks;
    while (chunk != NULL) {
        uintptr_t lo = (uintptr_t)chunk->data;
        uintptr_t start = chunk == mark.chunk ? lo + mark.used : lo;
        if (p >= lo && p < lo + chunk->used) {
            return p >= start;
        }
        if (chunk == mark.chunk) {
            return false;
        }
        chunk = chunk->next;
    }
    return false;
}

/* Bytes scanned by compaction attempts that failed; rule 2 is skipped while
   they exceed a quarter of all bytes allocated, so the scanning of data a
   region leaves to its callers stays linear in allocation. */
static uint64_t g_scanned;

uint64_t aleph_region_scanned(void) { return g_scanned; }

AlValue aleph_region_leave(AlMark mark, AlValue result) {
    size_t total;
    if (!region_holds(mark, (uintptr_t)aleph_value_root(result))) {
        aleph_region_reset(mark);
        return result;
    }
    total = (size_t)(live_bytes() - (mark.base + mark.used));
    if (total >= AL_REGION_MIN &&
        g_scanned <= (live_bytes() + aleph_g_freed) / 4 &&
        !region_compact(mark, total, &result)) {
        g_scanned += total / 4;
    }
    return result;
}

/* ------------------------------------------------------------------ */
/* Depth accounting                                                    */
/* ------------------------------------------------------------------ */

void aleph_eval_enter(const AlLoc *loc) {
    if (aleph_g_eval_depth >= AL_MAX_EVAL_DEPTH) {
        aleph_trap_at("R004", "stack overflow", loc);
    }
    aleph_g_eval_depth += 1;
    if (aleph_g_loc_depth < AL_MAX_EVAL_DEPTH + 1) {
        aleph_g_locs[aleph_g_loc_depth++] = loc;
    }
}

void aleph_eval_leave(void) {
    if (aleph_g_eval_depth > 0) {
        aleph_g_eval_depth -= 1;
    }
    if (aleph_g_loc_depth > 0) {
        aleph_g_loc_depth -= 1;
    }
}

static void native_stack_guard(const AlLoc *loc) {
    uintptr_t here;
    uintptr_t used;
    if (!aleph_g_stack_base_set) {
        return;
    }
    here = (uintptr_t)&here;
    used = here <= aleph_g_stack_base ? aleph_g_stack_base - here : here - aleph_g_stack_base;
    if (used > (uintptr_t)(AL_MAIN_STACK_SIZE - AL_STACK_GUARD)) {
        aleph_trap_at("R004", "stack overflow", loc);
    }
}

void aleph_call_enter(const AlLoc *loc) {
    native_stack_guard(loc);
    if (aleph_g_call_depth >= AL_MAX_CALL_DEPTH || aleph_g_eval_depth >= AL_MAX_EVAL_DEPTH) {
        aleph_trap_at("R004", "stack overflow", loc);
    }
    aleph_g_call_depth += 1;
    aleph_g_eval_depth += 1;
}

void aleph_call_leave(void) {
    if (aleph_g_call_depth > 0) {
        aleph_g_call_depth -= 1;
    }
    if (aleph_g_eval_depth > 0) {
        aleph_g_eval_depth -= 1;
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

AlValue aleph_fnval_kind(AlFnCode code, const char *name, AlFnKind kind) {
    AlValue value;
    AlFn *fn = (AlFn *)aleph_alloc(sizeof(AlFn));
    fn->code = code;
    fn->name = name;
    fn->kind = kind;
    fn->captured = NULL;
    fn->ncaptured = 0;
    value.tag = AL_FN;
    value.u.fn = fn;
    return value;
}

AlValue aleph_fn(AlFnCode code, const char *name) {
    return aleph_fnval_kind(code, name, AL_FN_USER);
}

AlValue aleph_fnval_lam(AlFnCode code, const AlValue *captured, size_t count) {
    AlValue value = aleph_fnval_kind(code, "lam", AL_FN_LAM);
    AlFn *fn = (AlFn *)value.u.fn;
    if (count > 0) {
        AlValue *copy;
        if (count > SIZE_MAX / sizeof(AlValue)) {
            out_of_memory();
        }
        copy = (AlValue *)aleph_alloc(count * sizeof(AlValue));
        memcpy(copy, captured, count * sizeof(AlValue));
        fn->captured = copy;
        fn->ncaptured = count;
    }
    return value;
}

AlValue aleph_fnval_static(const AlFn *fn) {
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
    case AL_DICT:
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
    case AL_JSON:
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
        if (left.u.fn->kind == AL_FN_LAM) {
            return left.u.fn == right.u.fn;
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
static void json_into(Buf *buffer, AlValue payload);

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
    case AL_DICT:
        buf_cstr(buffer, "dict");
        for (index = 0; index < value.u.list->len; index += 1) {
            buf_putc(buffer, ' ');
            literal_into(buffer, value.u.list->items[index]);
        }
        break;
    case AL_JSON:
        json_into(buffer, value.u.box->value);
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

/* Sort order for f64: total order on non-NaN values (-0.0 < 0.0), and every
 * NaN, whatever its sign bit or payload, after +inf.  The sign of a NaN
 * produced by 0.0 / 0.0 differs between x86-64 and arm64, so it must not
 * affect the order (ADR-0024). */
static int f64_total_cmp(double left, double right) {
    uint64_t left_bits;
    uint64_t right_bits;
    int64_t left_order;
    int64_t right_order;
    if (isnan(left) || isnan(right)) {
        return (isnan(left) ? 1 : 0) - (isnan(right) ? 1 : 0);
    }
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

/* ------------------------------------------------------------------ */
/* Dict builtins (ADR-0032): sorted pairs, copy on update               */
/* ------------------------------------------------------------------ */

static AlValue as_dict(AlValue list) {
    list.tag = AL_DICT;
    return list;
}

AlValue aleph_dict_empty(void) {
    return as_dict(aleph_list_new(0));
}

/* Index of the pair holding `key`, or of the pair it would precede. */
static size_t dict_find(const AlList *pairs, AlValue key, bool *found) {
    size_t low = 0;
    size_t high = pairs->len / 2;
    while (low < high) {
        size_t mid = low + (high - low) / 2;
        int order = value_order(pairs->items[2 * mid], key);
        if (order == 0) {
            *found = true;
            return mid;
        }
        if (order < 0) {
            low = mid + 1;
        } else {
            high = mid;
        }
    }
    *found = false;
    return low;
}

AlValue aleph_b_put(AlValue dict, AlValue key, AlValue value) {
    const AlList *pairs = dict.u.list;
    bool found;
    size_t at = dict_find(pairs, key, &found);
    size_t len = pairs->len + (found ? 0 : 2);
    AlValue result = aleph_list_new(len);
    AlValue *items = result.u.list->items;
    if (pairs->len > 0) {
        memcpy(items, pairs->items, 2 * at * sizeof(AlValue));
    }
    items[2 * at] = key;
    items[2 * at + 1] = value;
    {
        size_t rest = found ? 2 * at + 2 : 2 * at;
        if (pairs->len > rest) {
            memcpy(items + 2 * at + 2, pairs->items + rest,
                   (pairs->len - rest) * sizeof(AlValue));
        }
    }
    return as_dict(result);
}

AlValue aleph_b_get(AlValue dict, AlValue key) {
    bool found;
    size_t at = dict_find(dict.u.list, key, &found);
    return found ? aleph_some(dict.u.list->items[2 * at + 1]) : aleph_none();
}

AlValue aleph_b_del(AlValue dict, AlValue key) {
    const AlList *pairs = dict.u.list;
    bool found;
    size_t at = dict_find(pairs, key, &found);
    AlValue result;
    if (!found) {
        return dict;
    }
    result = aleph_list_new(pairs->len - 2);
    if (at > 0) {
        memcpy(result.u.list->items, pairs->items, 2 * at * sizeof(AlValue));
    }
    if (pairs->len > 2 * at + 2) {
        memcpy(result.u.list->items + 2 * at, pairs->items + 2 * at + 2,
               (pairs->len - 2 * at - 2) * sizeof(AlValue));
    }
    return as_dict(result);
}

static AlValue dict_column(AlValue dict, size_t column) {
    size_t count = dict.u.list->len / 2;
    AlValue result = aleph_list_new(count);
    size_t index;
    for (index = 0; index < count; index += 1) {
        result.u.list->items[index] = dict.u.list->items[2 * index + column];
    }
    return result;
}

AlValue aleph_b_keys(AlValue dict) {
    return dict_column(dict, 0);
}

AlValue aleph_b_vals(AlValue dict) {
    return dict_column(dict, 1);
}

AlValue aleph_b_size(AlValue dict) {
    return aleph_i64((int64_t)(dict.u.list->len / 2));
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
        result.u.list->items[index] = fn->code(fn, args, 1);
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
        keep = fn->code(fn, args, 1);
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
        accumulator = fn->code(fn, args, 2);
    }
    return accumulator;
}

/* ADR-0036 ---------------------------------------------------------- */

AlValue aleph_b_abs(AlValue x) {
    if (x.tag == AL_F64) {
        return aleph_f64(fabs(x.u.f64));
    }
    if (x.u.i64 == INT64_MIN) {
        aleph_trap("R001", "integer overflow");
    }
    return aleph_i64(x.u.i64 < 0 ? -x.u.i64 : x.u.i64);
}

/* `min a b` is b when b < a, else a; `max a b` is b when b > a, else a.
   With NaN the comparison is false, so the first argument is kept. */
AlValue aleph_b_min(AlValue left, AlValue right) {
    if (left.tag == AL_F64) {
        return right.u.f64 < left.u.f64 ? right : left;
    }
    return right.u.i64 < left.u.i64 ? right : left;
}

AlValue aleph_b_max(AlValue left, AlValue right) {
    if (left.tag == AL_F64) {
        return right.u.f64 > left.u.f64 ? right : left;
    }
    return right.u.i64 > left.u.i64 ? right : left;
}

AlValue aleph_b_starts(AlValue text, AlValue prefix) {
    size_t len = prefix.u.str->len;
    return aleph_bool(len <= text.u.str->len &&
                      memcmp(text.u.str->data, prefix.u.str->data, len) == 0);
}

AlValue aleph_b_ends(AlValue text, AlValue suffix) {
    size_t len = suffix.u.str->len;
    size_t total = text.u.str->len;
    return aleph_bool(len <= total &&
                      memcmp(text.u.str->data + (total - len), suffix.u.str->data,
                             len) == 0);
}

AlValue aleph_b_uniq(AlValue xs) {
    AlValue result = aleph_b_sort(xs);
    AlList *list = result.u.list;
    size_t kept = 0;
    size_t index;
    for (index = 0; index < list->len; index += 1) {
        if (kept == 0 || value_order(list->items[kept - 1], list->items[index]) != 0) {
            list->items[kept] = list->items[index];
            kept += 1;
        }
    }
    list->len = kept;
    return result;
}

AlValue aleph_b_map2(AlValue function, AlValue left, AlValue right) {
    const AlFn *fn = function.u.fn;
    size_t len = left.u.list->len < right.u.list->len ? left.u.list->len
                                                      : right.u.list->len;
    AlValue result = aleph_list_new(len);
    size_t index;
    for (index = 0; index < len; index += 1) {
        AlValue args[2];
        args[0] = left.u.list->items[index];
        args[1] = right.u.list->items[index];
        result.u.list->items[index] = fn->code(fn, args, 2);
    }
    return result;
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

/* ADR-0035: scalar value of the first character; none when empty. */
AlValue aleph_b_ord(AlValue text) {
    uint32_t codepoint;
    if (utf8_decode((const unsigned char *)text.u.str->data, text.u.str->len,
                    &codepoint) == 0) {
        return aleph_none();
    }
    return aleph_some(aleph_i64((int64_t)codepoint));
}

/* ADR-0035: one-character string; none outside Unicode scalar values. */
AlValue aleph_b_chr(AlValue code) {
    int64_t n = code.u.i64;
    char bytes[4];
    size_t len;
    if (n < 0 || n > 0x10FFFF || (n >= 0xD800 && n <= 0xDFFF)) {
        return aleph_none();
    }
    if (n < 0x80) {
        bytes[0] = (char)n;
        len = 1;
    } else if (n < 0x800) {
        bytes[0] = (char)(0xc0 | (n >> 6));
        bytes[1] = (char)(0x80 | (n & 0x3f));
        len = 2;
    } else if (n < 0x10000) {
        bytes[0] = (char)(0xe0 | (n >> 12));
        bytes[1] = (char)(0x80 | ((n >> 6) & 0x3f));
        bytes[2] = (char)(0x80 | (n & 0x3f));
        len = 3;
    } else {
        bytes[0] = (char)(0xf0 | (n >> 18));
        bytes[1] = (char)(0x80 | ((n >> 12) & 0x3f));
        bytes[2] = (char)(0x80 | ((n >> 6) & 0x3f));
        bytes[3] = (char)(0x80 | (n & 0x3f));
        len = 4;
    }
    return aleph_some(aleph_str_n(bytes, len));
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

/* ------------------------------------------------------------------ */
/* JSON (ADR-0033): mirrors compiler/src/json.rs byte for byte          */
/* ------------------------------------------------------------------ */

#define JSON_MAX_DEPTH 256

static void json_str_into(Buf *buffer, const AlStr *text) {
    size_t index;
    char escape[8];
    buf_putc(buffer, '"');
    for (index = 0; index < text->len; index += 1) {
        unsigned char byte = (unsigned char)text->data[index];
        switch (byte) {
        case '"':
            buf_cstr(buffer, "\\\"");
            break;
        case '\\':
            buf_cstr(buffer, "\\\\");
            break;
        case '\b':
            buf_cstr(buffer, "\\b");
            break;
        case '\f':
            buf_cstr(buffer, "\\f");
            break;
        case '\n':
            buf_cstr(buffer, "\\n");
            break;
        case '\r':
            buf_cstr(buffer, "\\r");
            break;
        case '\t':
            buf_cstr(buffer, "\\t");
            break;
        default:
            if (byte < 0x20) {
                snprintf(escape, sizeof(escape), "\\u%04x", (unsigned)byte);
                buf_cstr(buffer, escape);
            } else {
                buf_putc(buffer, (char)byte);
            }
        }
    }
    buf_putc(buffer, '"');
}

static void json_into(Buf *buffer, AlValue payload) {
    size_t index;
    switch (payload.tag) {
    case AL_UNIT:
        buf_cstr(buffer, "null");
        break;
    case AL_BOOL:
        buf_cstr(buffer, payload.u.b ? "true" : "false");
        break;
    case AL_I64:
    case AL_F64:
        show_into(buffer, payload);
        break;
    case AL_STR:
        json_str_into(buffer, payload.u.str);
        break;
    case AL_LIST:
        buf_putc(buffer, '[');
        for (index = 0; index < payload.u.list->len; index += 1) {
            if (index > 0) {
                buf_putc(buffer, ',');
            }
            json_into(buffer, payload.u.list->items[index].u.box->value);
        }
        buf_putc(buffer, ']');
        break;
    case AL_DICT:
        buf_putc(buffer, '{');
        for (index = 0; index < payload.u.list->len; index += 2) {
            if (index > 0) {
                buf_putc(buffer, ',');
            }
            json_str_into(buffer, payload.u.list->items[index].u.str);
            buf_putc(buffer, ':');
            json_into(buffer, payload.u.list->items[index + 1].u.box->value);
        }
        buf_putc(buffer, '}');
        break;
    default:
        break;
    }
}

static uint32_t json_depth(AlValue json) {
    return ((const AlJson *)json.u.box)->depth;
}

/* Wrap a payload, computing its depth from its (already wrapped) children. */
static AlValue json_make(AlValue payload) {
    AlValue value;
    AlJson *json = (AlJson *)aleph_alloc(sizeof(AlJson));
    size_t index;
    uint32_t depth = 0;
    if (payload.tag == AL_LIST || payload.tag == AL_DICT) {
        size_t step = payload.tag == AL_DICT ? 2 : 1;
        size_t start = payload.tag == AL_DICT ? 1 : 0;
        for (index = start; index < payload.u.list->len; index += step) {
            uint32_t child = json_depth(payload.u.list->items[index]);
            if (child > depth) {
                depth = child;
            }
        }
        depth += 1;
    }
    json->box.value = payload;
    json->depth = depth;
    value.tag = AL_JSON;
    value.u.box = &json->box;
    return value;
}

AlValue aleph_b_json(AlValue value) {
    AlValue json;
    if (value.tag == AL_JSON) {
        return value;
    }
    if (value.tag == AL_F64 && !isfinite(value.u.f64)) {
        value = aleph_unit();
    }
    json = json_make(value);
    if (json_depth(json) > JSON_MAX_DEPTH) {
        aleph_trap("R005", "allocation limit exceeded");
    }
    return json;
}

typedef struct JsonParser {
    const unsigned char *data;
    size_t len;
    size_t pos;
    char error[96];
} JsonParser;

/* Records the first error; returns false so callers can `return json_fail`. */
static bool json_fail(JsonParser *parser, const char *reason, size_t at) {
    snprintf(parser->error, sizeof(parser->error), "json: %s at byte %zu",
             reason, at);
    return false;
}

static bool json_unexpected(JsonParser *parser) {
    if (parser->pos >= parser->len) {
        return json_fail(parser, "unexpected end", parser->len);
    }
    return json_fail(parser, "unexpected byte", parser->pos);
}

static int json_peek(const JsonParser *parser) {
    return parser->pos < parser->len ? parser->data[parser->pos] : -1;
}

static void json_skip_ws(JsonParser *parser) {
    int byte = json_peek(parser);
    while (byte == ' ' || byte == '\t' || byte == '\n' || byte == '\r') {
        parser->pos += 1;
        byte = json_peek(parser);
    }
}

static bool json_value(JsonParser *parser, size_t depth, AlValue *out);

/* A malloc'd growable array, so parsing leaves no garbage in the arena. */
typedef struct Values {
    AlValue *items;
    size_t len;
    size_t cap;
} Values;

static void values_insert(Values *values, size_t at, AlValue value) {
    if (values->len == values->cap) {
        size_t cap = values->cap == 0 ? 8 : values->cap * 2;
        AlValue *items;
        if (cap > SIZE_MAX / 2 / sizeof(AlValue)) {
            out_of_memory();
        }
        items = (AlValue *)realloc(values->items, cap * sizeof(AlValue));
        if (items == NULL) {
            out_of_memory();
        }
        values->items = items;
        values->cap = cap;
    }
    memmove(values->items + at + 1, values->items + at,
            (values->len - at) * sizeof(AlValue));
    values->items[at] = value;
    values->len += 1;
}

static AlValue values_take(Values *values) {
    AlValue list = aleph_list(values->items, values->len);
    free(values->items);
    return list;
}

static bool json_literal(JsonParser *parser, const char *word, AlValue payload,
                         AlValue *out) {
    size_t len = strlen(word);
    if (parser->len - parser->pos < len ||
        memcmp(parser->data + parser->pos, word, len) != 0) {
        return json_fail(parser, "unexpected byte", parser->pos);
    }
    parser->pos += len;
    *out = payload;
    return true;
}

static bool json_open(JsonParser *parser, size_t depth) {
    if (depth > JSON_MAX_DEPTH) {
        return json_fail(parser, "too deep", parser->pos);
    }
    parser->pos += 1;
    json_skip_ws(parser);
    return true;
}

/* After an element: *more = true on ',', false on `close`. */
static bool json_separator(JsonParser *parser, int close, bool *more) {
    int byte;
    json_skip_ws(parser);
    byte = json_peek(parser);
    if (byte == ',' || byte == close) {
        parser->pos += 1;
        *more = byte == ',';
        return true;
    }
    return json_unexpected(parser);
}

static bool json_list(JsonParser *parser, size_t depth, AlValue *out) {
    Values items = {NULL, 0, 0};
    AlValue item;
    bool more = true;
    if (!json_open(parser, depth)) {
        return false;
    }
    if (json_peek(parser) == ']') {
        parser->pos += 1;
        more = false;
    }
    while (more) {
        if (!json_value(parser, depth, &item) ||
            !json_separator(parser, ']', &more)) {
            free(items.items);
            return false;
        }
        values_insert(&items, items.len, item);
    }
    *out = values_take(&items);
    return true;
}

static bool hex4(JsonParser *parser, uint32_t *code) {
    size_t index;
    uint32_t value = 0;
    if (parser->len - parser->pos < 4) {
        return false;
    }
    for (index = 0; index < 4; index += 1) {
        int digit = parser->data[parser->pos + index];
        value *= 16;
        if (digit >= '0' && digit <= '9') {
            value += (uint32_t)(digit - '0');
        } else if (digit >= 'a' && digit <= 'f') {
            value += (uint32_t)(digit - 'a' + 10);
        } else if (digit >= 'A' && digit <= 'F') {
            value += (uint32_t)(digit - 'A' + 10);
        } else {
            return false;
        }
    }
    parser->pos += 4;
    *code = value;
    return true;
}

static void put_utf8(Buf *buffer, uint32_t code) {
    if (code < 0x80) {
        buf_putc(buffer, (char)code);
    } else if (code < 0x800) {
        buf_putc(buffer, (char)(0xc0 | (code >> 6)));
        buf_putc(buffer, (char)(0x80 | (code & 0x3f)));
    } else if (code < 0x10000) {
        buf_putc(buffer, (char)(0xe0 | (code >> 12)));
        buf_putc(buffer, (char)(0x80 | ((code >> 6) & 0x3f)));
        buf_putc(buffer, (char)(0x80 | (code & 0x3f)));
    } else {
        buf_putc(buffer, (char)(0xf0 | (code >> 18)));
        buf_putc(buffer, (char)(0x80 | ((code >> 12) & 0x3f)));
        buf_putc(buffer, (char)(0x80 | ((code >> 6) & 0x3f)));
        buf_putc(buffer, (char)(0x80 | (code & 0x3f)));
    }
}

/* The rest of `\uXXXX` (and a following low surrogate escape). */
static bool json_unicode(JsonParser *parser, size_t at, Buf *buffer) {
    uint32_t high;
    uint32_t low;
    if (!hex4(parser, &high)) {
        return json_fail(parser, "bad escape", at);
    }
    if (high >= 0xdc00 && high <= 0xdfff) {
        return json_fail(parser, "bad escape", at);
    }
    if (high >= 0xd800 && high <= 0xdbff) {
        if (parser->len - parser->pos < 2 ||
            parser->data[parser->pos] != '\\' ||
            parser->data[parser->pos + 1] != 'u') {
            return json_fail(parser, "bad escape", at);
        }
        parser->pos += 2;
        if (!hex4(parser, &low) || low < 0xdc00 || low > 0xdfff) {
            return json_fail(parser, "bad escape", at);
        }
        high = 0x10000 + ((high - 0xd800) << 10) + (low - 0xdc00);
    }
    put_utf8(buffer, high);
    return true;
}

static bool json_escape(JsonParser *parser, Buf *buffer) {
    size_t at = parser->pos;
    int byte;
    parser->pos += 1;
    byte = json_peek(parser);
    if (byte < 0) {
        return json_unexpected(parser);
    }
    parser->pos += 1;
    switch (byte) {
    case '"':
    case '\\':
    case '/':
        buf_putc(buffer, (char)byte);
        return true;
    case 'b':
        buf_putc(buffer, '\b');
        return true;
    case 'f':
        buf_putc(buffer, '\f');
        return true;
    case 'n':
        buf_putc(buffer, '\n');
        return true;
    case 'r':
        buf_putc(buffer, '\r');
        return true;
    case 't':
        buf_putc(buffer, '\t');
        return true;
    case 'u':
        return json_unicode(parser, at, buffer);
    default:
        return json_fail(parser, "bad escape", at);
    }
}

/* A string starting at the opening quote. */
static bool json_string(JsonParser *parser, AlValue *out) {
    Buf buffer = {NULL, 0, 0};
    parser->pos += 1;
    for (;;) {
        int byte = json_peek(parser);
        if (byte < 0x20) {
            free(buffer.data);
            return json_unexpected(parser);
        }
        if (byte == '"') {
            parser->pos += 1;
            break;
        }
        if (byte == '\\') {
            if (!json_escape(parser, &buffer)) {
                free(buffer.data);
                return false;
            }
        } else {
            buf_putc(&buffer, (char)byte);
            parser->pos += 1;
        }
    }
    *out = take_buffer(&buffer);
    return true;
}

/* An object member while parsing; sorted once when the object closes. */
typedef struct JsonMember {
    AlValue key;
    AlValue val;
    size_t at; /* offset of the key's opening quote */
} JsonMember;

typedef struct Members {
    JsonMember *items;
    size_t len;
    size_t cap;
} Members;

static void members_push(Members *members, JsonMember member) {
    if (members->len == members->cap) {
        size_t cap = members->cap == 0 ? 8 : members->cap * 2;
        JsonMember *items;
        if (cap > SIZE_MAX / 2 / sizeof(JsonMember)) {
            out_of_memory();
        }
        items = (JsonMember *)realloc(members->items, cap * sizeof(JsonMember));
        if (items == NULL) {
            out_of_memory();
        }
        members->items = items;
        members->cap = cap;
    }
    members->items[members->len] = member;
    members->len += 1;
}

/* By key, then offset: equal keys stay in input order. */
static int member_order(const void *left, const void *right) {
    const JsonMember *a = (const JsonMember *)left;
    const JsonMember *b = (const JsonMember *)right;
    int order = value_order(a->key, b->key);
    if (order != 0) {
        return order;
    }
    return a->at < b->at ? -1 : (a->at > b->at ? 1 : 0);
}

/* One `"key": value` member, appended to `members`. */
static bool json_member(JsonParser *parser, size_t depth, Members *members) {
    JsonMember member;
    json_skip_ws(parser);
    if (json_peek(parser) != '"') {
        return json_unexpected(parser);
    }
    member.at = parser->pos;
    if (!json_string(parser, &member.key)) {
        return false;
    }
    json_skip_ws(parser);
    if (json_peek(parser) != ':') {
        return json_unexpected(parser);
    }
    parser->pos += 1;
    if (!json_value(parser, depth, &member.val)) {
        return false;
    }
    members_push(members, member);
    return true;
}

/* Sorts the members; a duplicate key is reported only after the object
   parsed otherwise, at the smallest offset of a repeated key. */
static bool json_members_take(JsonParser *parser, Members *members,
                              AlValue *out) {
    size_t index;
    size_t duplicate = SIZE_MAX;
    AlValue dict;
    if (members->len > 1) {
        qsort(members->items, members->len, sizeof(JsonMember), member_order);
    }
    for (index = 1; index < members->len; index += 1) {
        if (value_order(members->items[index - 1].key,
                        members->items[index].key) == 0 &&
            members->items[index].at < duplicate) {
            duplicate = members->items[index].at;
        }
    }
    if (duplicate != SIZE_MAX) {
        free(members->items);
        return json_fail(parser, "duplicate key", duplicate);
    }
    dict = aleph_list_new(2 * members->len);
    for (index = 0; index < members->len; index += 1) {
        dict.u.list->items[2 * index] = members->items[index].key;
        dict.u.list->items[2 * index + 1] = members->items[index].val;
    }
    free(members->items);
    *out = as_dict(dict);
    return true;
}

static bool json_object(JsonParser *parser, size_t depth, AlValue *out) {
    Members members = {NULL, 0, 0};
    bool more = true;
    if (!json_open(parser, depth)) {
        return false;
    }
    if (json_peek(parser) == '}') {
        parser->pos += 1;
        more = false;
    }
    while (more) {
        if (!json_member(parser, depth, &members) ||
            !json_separator(parser, '}', &more)) {
            free(members.items);
            return false;
        }
    }
    return json_members_take(parser, &members, out);
}

static void json_digits(JsonParser *parser) {
    while (json_peek(parser) >= '0' && json_peek(parser) <= '9') {
        parser->pos += 1;
    }
}

static bool json_digits_required(JsonParser *parser) {
    size_t start = parser->pos;
    json_digits(parser);
    return parser->pos > start;
}

/* Longest number token converted; matches the Rust engine (ADR-0033). */
#define JSON_MAX_NUMBER 1024

static bool json_number(JsonParser *parser, AlValue *out) {
    size_t start = parser->pos;
    bool integral = true;
    int64_t integer;
    double number;
    int byte;
    if (json_peek(parser) == '-') {
        parser->pos += 1;
    }
    byte = json_peek(parser);
    if (byte == '0') {
        parser->pos += 1;
    } else if (byte >= '1' && byte <= '9') {
        json_digits(parser);
    } else {
        return json_fail(parser, "bad number", start);
    }
    if (json_peek(parser) == '.') {
        integral = false;
        parser->pos += 1;
        if (!json_digits_required(parser)) {
            return json_fail(parser, "bad number", start);
        }
    }
    byte = json_peek(parser);
    if (byte == 'e' || byte == 'E') {
        integral = false;
        parser->pos += 1;
        byte = json_peek(parser);
        if (byte == '+' || byte == '-') {
            parser->pos += 1;
        }
        if (!json_digits_required(parser)) {
            return json_fail(parser, "bad number", start);
        }
    }
    if (parser->pos - start > JSON_MAX_NUMBER) {
        return json_fail(parser, "bad number", start);
    }
    if (integral && parse_int_value((const char *)parser->data + start,
                                    parser->pos - start, &integer)) {
        *out = aleph_i64(integer);
        return true;
    }
    {
        /* The grammar is checked above; strtod rounds correctly. */
        size_t len = parser->pos - start;
        char *copy = (char *)malloc(len + 1);
        if (copy == NULL) {
            out_of_memory();
        }
        memcpy(copy, parser->data + start, len);
        copy[len] = '\0';
        number = strtod(copy, NULL);
        free(copy);
    }
    if (!isfinite(number)) {
        return json_fail(parser, "bad number", start);
    }
    *out = aleph_f64(number);
    return true;
}

static bool json_value(JsonParser *parser, size_t depth, AlValue *out) {
    AlValue payload;
    bool ok;
    json_skip_ws(parser);
    switch (json_peek(parser)) {
    case -1:
        return json_unexpected(parser);
    case '[':
        ok = json_list(parser, depth + 1, &payload);
        break;
    case '{':
        ok = json_object(parser, depth + 1, &payload);
        break;
    case '"':
        ok = json_string(parser, &payload);
        break;
    case 't':
        ok = json_literal(parser, "true", aleph_bool(true), &payload);
        break;
    case 'f':
        ok = json_literal(parser, "false", aleph_bool(false), &payload);
        break;
    case 'n':
        ok = json_literal(parser, "null", aleph_unit(), &payload);
        break;
    default:
        if (json_peek(parser) == '-' ||
            (json_peek(parser) >= '0' && json_peek(parser) <= '9')) {
            ok = json_number(parser, &payload);
        } else {
            return json_unexpected(parser);
        }
    }
    if (!ok) {
        return false;
    }
    *out = json_make(payload);
    return true;
}

AlValue aleph_b_jparse(AlValue text) {
    JsonParser parser;
    AlValue value;
    parser.data = (const unsigned char *)text.u.str->data;
    parser.len = text.u.str->len;
    parser.pos = 0;
    parser.error[0] = '\0';
    if (!json_value(&parser, 0, &value)) {
        return aleph_err(aleph_str(parser.error));
    }
    json_skip_ws(&parser);
    if (parser.pos < parser.len) {
        json_fail(&parser, "unexpected byte", parser.pos);
        return aleph_err(aleph_str(parser.error));
    }
    return aleph_ok(value);
}

static AlValue json_field(AlValue json, AlTag tag) {
    AlValue payload = json.u.box->value;
    return payload.tag == tag ? aleph_some(payload) : aleph_none();
}

AlValue aleph_b_jbool(AlValue json) { return json_field(json, AL_BOOL); }
AlValue aleph_b_jint(AlValue json) { return json_field(json, AL_I64); }
AlValue aleph_b_jstr(AlValue json) { return json_field(json, AL_STR); }
AlValue aleph_b_jlist(AlValue json) { return json_field(json, AL_LIST); }
AlValue aleph_b_jdict(AlValue json) { return json_field(json, AL_DICT); }

AlValue aleph_b_jnum(AlValue json) {
    AlValue payload = json.u.box->value;
    if (payload.tag == AL_I64) {
        return aleph_some(aleph_f64((double)payload.u.i64));
    }
    return json_field(json, AL_F64);
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

static bool ascii_case_equal(const char *left, const char *right,
                            size_t len) {
    size_t index;
    for (index = 0; index < len; index += 1) {
        unsigned char a = (unsigned char)left[index];
        unsigned char b = (unsigned char)right[index];
        if (a >= 'A' && a <= 'Z') {
            a = (unsigned char)(a - 'A' + 'a');
        }
        if (b >= 'A' && b <= 'Z') {
            b = (unsigned char)(b - 'A' + 'a');
        }
        if (a != b) {
            return false;
        }
    }
    return true;
}

/* Host of an http:// or https:// URL (ADR-0038), as a slice of `url`. */
static bool url_host_bytes(const char *url, size_t len, const char **host,
                           size_t *host_len) {
    const char *start;
    const char *limit = url + len;
    const char *authority_end;
    const char *host_end;
    const char *cursor;

    if (memchr(url, '\0', len) != NULL) {
        return false;
    }
    if (len >= 7 && memcmp(url, "http://", 7) == 0) {
        start = url + 7;
    } else if (len >= 8 && memcmp(url, "https://", 8) == 0) {
        start = url + 8;
    } else {
        return false;
    }
    authority_end = limit;
    for (cursor = start; cursor < limit; cursor += 1) {
        if (*cursor == '/' || *cursor == '?' || *cursor == '#') {
            authority_end = cursor;
            break;
        }
    }
    for (cursor = start; cursor < authority_end; cursor += 1) {
        if (*cursor == '@') {
            return false;
        }
    }
    host_end = authority_end;
    for (cursor = start; cursor < authority_end; cursor += 1) {
        if (*cursor == ':') {
            host_end = cursor;
            break;
        }
    }
    if (host_end == start) {
        return false;
    }
    *host = start;
    *host_len = (size_t)(host_end - start);
    return true;
}

static bool host_allowed_bytes(const char *host, size_t host_len) {
    size_t index;
    for (index = 0; index < g_use_count; index += 1) {
        const char *declared = g_uses[index].path;
        if (g_uses[index].cap != AL_CAP_NET_HOST || declared == NULL) {
            continue;
        }
        if (strlen(declared) == host_len &&
            ascii_case_equal(declared, host, host_len)) {
            return true;
        }
    }
    return false;
}

static void append_bytes(char **buffer, size_t *used, size_t *capacity,
                         const char *data, size_t count) {
    size_t next;
    char *grown;
    if (count == 0) {
        return;
    }
    if (*used > SIZE_MAX - count) {
        free(*buffer);
        out_of_memory();
    }
    next = *used + count;
    if (next > *capacity) {
        size_t wanted = *capacity == 0 ? 4096 : *capacity;
        while (wanted < next) {
            if (wanted > SIZE_MAX / 2) {
                free(*buffer);
                out_of_memory();
            }
            wanted *= 2;
        }
        grown = (char *)realloc(*buffer, wanted);
        if (grown == NULL) {
            free(*buffer);
            out_of_memory();
        }
        *buffer = grown;
        *capacity = wanted;
    }
    memcpy(*buffer + *used, data, count);
    *used = next;
}

/* `fetch: ` followed by `text`, as an `err` value. */
static AlValue fetch_error(const char *text, size_t len) {
    static const char prefix[] = "fetch: ";
    size_t prefix_len = sizeof(prefix) - 1;
    char *message = (char *)malloc(prefix_len + len + 1);
    AlValue value;
    if (message == NULL) {
        out_of_memory();
    }
    memcpy(message, prefix, prefix_len);
    if (len > 0) {
        memcpy(message + prefix_len, text, len);
    }
    message[prefix_len + len] = '\0';
    value = aleph_err(aleph_str_n(message, prefix_len + len));
    free(message);
    return value;
}

/* The first line of curl's stderr with its `curl: (N) ` prefix removed. */
static void fetch_curl_line(const char *stderr_data, size_t len,
                            const char **line_out, size_t *line_len_out) {
    const char *line = stderr_data;
    size_t line_len = 0;
    const char *close;
    size_t after;

    while (line_len < len && stderr_data[line_len] != '\n') {
        line_len += 1;
    }
    while (line_len > 0 &&
           (line[line_len - 1] == ' ' || line[line_len - 1] == '\t' ||
            line[line_len - 1] == '\r')) {
        line_len -= 1;
    }
    if (line_len >= 7 && memcmp(line, "curl: (", 7) == 0) {
        close = (const char *)memchr(line + 7, ')', line_len - 7);
        if (close != NULL) {
            after = (size_t)(close - line) + 1;
            if (after < line_len && line[after] == ' ') {
                after += 1;
            }
            line += after;
            line_len -= after;
        }
    }
    *line_out = line;
    *line_len_out = line_len;
}

static AlValue fetch_not_found(void) {
    static const char message[] = "curl not found";
    return fetch_error(message, sizeof(message) - 1);
}

/* Close `*fd` when it is a real descriptor and mark it closed. */
static void close_fd(int *fd) {
    if (*fd >= 0) {
        (void)close(*fd);
        *fd = -1;
    }
}

AlValue aleph_b_fetch(AlValue url_value) {
    const char *host;
    size_t host_len;
    char *url;
    int out_pipe[2] = {-1, -1};
    int err_pipe[2] = {-1, -1};
    int exec_pipe[2] = {-1, -1};
    int null_fd;
    pid_t child;
    ssize_t count;
    int exec_error = 0;
    int status = 0;
    bool io_failed = false;
    bool poll_failed = false;
    bool too_large = false;
    char *out = NULL;
    char *err = NULL;
    size_t out_len = 0;
    size_t out_cap = 0;
    size_t err_len = 0;
    size_t err_cap = 0;
    AlValue result;

    if (!url_host_bytes(url_value.u.str->data, url_value.u.str->len, &host,
                        &host_len)) {
        aleph_trap("R003", "capability violation");
    }
    if (!host_allowed_bytes(host, host_len)) {
        aleph_trap("R003", "capability violation");
    }
    url = string_to_cstr(url_value.u.str);
    null_fd = open("/dev/null", O_RDONLY);
    if (null_fd < 0) {
        free(url);
        return fetch_error("curl failed", 11);
    }
    if (pipe(out_pipe) != 0) {
        close_fd(&null_fd);
        free(url);
        return fetch_error("curl failed", 11);
    }
    if (pipe(err_pipe) != 0) {
        close_fd(&out_pipe[0]);
        close_fd(&out_pipe[1]);
        close_fd(&null_fd);
        free(url);
        return fetch_error("curl failed", 11);
    }
    if (pipe(exec_pipe) != 0) {
        close_fd(&out_pipe[0]);
        close_fd(&out_pipe[1]);
        close_fd(&err_pipe[0]);
        close_fd(&err_pipe[1]);
        close_fd(&null_fd);
        free(url);
        return fetch_error("curl failed", 11);
    }
    (void)fcntl(exec_pipe[1], F_SETFD, FD_CLOEXEC);

    child = fork();
    if (child < 0) {
        close_fd(&out_pipe[0]);
        close_fd(&out_pipe[1]);
        close_fd(&err_pipe[0]);
        close_fd(&err_pipe[1]);
        close_fd(&exec_pipe[0]);
        close_fd(&exec_pipe[1]);
        close_fd(&null_fd);
        free(url);
        return fetch_error("curl failed", 11);
    }
    if (child == 0) {
        char *const argv[] = {
            (char *)"curl",          (char *)"-q",
            (char *)"-g",            (char *)"-sS",
            (char *)"--fail",
            (char *)"--max-time",    (char *)"30",
            (char *)"--max-redirs",  (char *)"0",
            (char *)"--proto",       (char *)"=http,https",
            (char *)"--max-filesize", (char *)"16777216",
            (char *)"--",            url,
            NULL};
        int saved;
        ssize_t ignored;
        close_fd(&out_pipe[0]);
        close_fd(&err_pipe[0]);
        close_fd(&exec_pipe[0]);
        if (dup2(null_fd, STDIN_FILENO) < 0 ||
            dup2(out_pipe[1], STDOUT_FILENO) < 0 ||
            dup2(err_pipe[1], STDERR_FILENO) < 0) {
            _exit(127);
        }
        if (null_fd > STDERR_FILENO) {
            close(null_fd);
        }
        if (out_pipe[1] > STDERR_FILENO) {
            close(out_pipe[1]);
        }
        if (err_pipe[1] > STDERR_FILENO) {
            close(err_pipe[1]);
        }
        (void)signal(SIGPIPE, SIG_DFL); /* Tlön ignores it; curl should not inherit that */
        execvp("curl", argv);
        saved = errno;
        ignored = write(exec_pipe[1], &saved, sizeof(saved));
        (void)ignored;
        _exit(127);
    }

    close_fd(&out_pipe[1]);
    close_fd(&err_pipe[1]);
    close_fd(&exec_pipe[1]);
    close_fd(&null_fd);
    do {
        count = read(exec_pipe[0], &exec_error, sizeof(exec_error));
    } while (count < 0 && errno == EINTR);
    close_fd(&exec_pipe[0]);

    if (count != (ssize_t)sizeof(exec_error)) {
        exec_error = 0;
    }
    for (;;) {
        struct pollfd fds[2];
        nfds_t nfds = 0;
        int out_slot = -1;
        int err_slot = -1;
        char chunk[8192];

        if (out_pipe[0] >= 0) {
            fds[nfds].fd = out_pipe[0];
            fds[nfds].events = POLLIN;
            fds[nfds].revents = 0;
            out_slot = (int)nfds;
            nfds += 1;
        }
        if (err_pipe[0] >= 0) {
            fds[nfds].fd = err_pipe[0];
            fds[nfds].events = POLLIN;
            fds[nfds].revents = 0;
            err_slot = (int)nfds;
            nfds += 1;
        }
        if (nfds == 0) {
            break;
        }
        do {
            count = poll(fds, nfds, -1);
        } while (count < 0 && errno == EINTR);
        if (count < 0) {
            poll_failed = true;
            break;
        }

        if (out_slot >= 0 && fds[out_slot].revents != 0) {
            count = read(out_pipe[0], chunk, sizeof(chunk));
            if (count < 0) {
                if (errno == EINTR) {
                    continue;
                }
                io_failed = true;
                break;
            }
            if (count == 0) {
                close_fd(&out_pipe[0]);
            } else {
                append_bytes(&out, &out_len, &out_cap, chunk, (size_t)count);
                if (out_len > 16777216) {
                    /* Streaming cut-off: stop reading, kill curl and report. */
                    too_large = true;
                    break;
                }
            }
        }
        if (err_slot >= 0 && fds[err_slot].revents != 0) {
            count = read(err_pipe[0], chunk, sizeof(chunk));
            if (count < 0) {
                if (errno == EINTR) {
                    continue;
                }
                io_failed = true;
                break;
            }
            if (count == 0) {
                close_fd(&err_pipe[0]);
            } else {
                append_bytes(&err, &err_len, &err_cap, chunk, (size_t)count);
            }
        }
    }

    if (too_large || poll_failed || io_failed) {
        kill(child, SIGKILL);
        close_fd(&out_pipe[0]);
        close_fd(&err_pipe[0]);
        while (waitpid(child, &status, 0) < 0 && errno == EINTR) {
        }
        free(out);
        free(err);
        free(url);
        if (too_large) {
            return fetch_error("response too large", 18);
        }
        if (poll_failed) {
            return fetch_error("curl failed", 11);
        }
        return fetch_error("io error", 8);
    }

    while (waitpid(child, &status, 0) < 0) {
        if (errno != EINTR) {
            io_failed = true;
            break;
        }
    }

    if (exec_error != 0) {
        free(out);
        free(err);
        free(url);
        if (exec_error == ENOENT) {
            return fetch_not_found();
        }
        return fetch_error("curl failed", 11);
    }
    if (io_failed) {
        free(out);
        free(err);
        free(url);
        return fetch_error("io error", 8);
    }
    free(url);

    if (!WIFEXITED(status) || WEXITSTATUS(status) != 0) {
        const char *line;
        size_t line_len;
        if (WIFEXITED(status) && WEXITSTATUS(status) == 63) {
            result = fetch_error("response too large", 18);
        } else {
            fetch_curl_line(err, err_len, &line, &line_len);
            if (line_len == 0 ||
                !utf8_valid((const unsigned char *)line, line_len)) {
                result = fetch_error("curl failed", 11);
            } else {
                result = fetch_error(line, line_len);
            }
        }
        free(out);
        free(err);
        return result;
    }
    if (out_len > 16777216) {
        free(out);
        free(err);
        return fetch_error("response too large", 18);
    }
    if (!utf8_valid((const unsigned char *)out, out_len)) {
        free(out);
        free(err);
        return fetch_error("body is not valid UTF-8", 23);
    }
    result = aleph_ok(aleph_str_n(out != NULL ? out : "", out_len));
    free(out);
    free(err);
    return result;
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

AlValue aleph_b_env(AlValue name) {
    char *key;
    const char *value;

    if (name.u.str->len == 0 ||
        memchr(name.u.str->data, '\0', name.u.str->len) != NULL ||
        memchr(name.u.str->data, '=', name.u.str->len) != NULL) {
        return aleph_none();
    }
    key = string_to_cstr(name.u.str);
    value = getenv(key);
    free(key);
    if (value == NULL) {
        return aleph_none();
    }
    if (!utf8_valid((const unsigned char *)value, strlen(value))) {
        return aleph_none();
    }
    return aleph_some(aleph_str(value));
}

AlValue aleph_b_now(void) {
    struct timespec current;

    if (clock_gettime(CLOCK_REALTIME, &current) != 0) {
        return aleph_i64(0);
    }
    /* Milliseconds since the epoch, flooring nanoseconds by 1e6. `tv_nsec` is
       always non-negative, so this equals floor((tv_sec * 1e9 + tv_nsec) /
       1e6) and times before 1970 round down like the interpreter. */
    return aleph_i64((int64_t)current.tv_sec * 1000 +
                     (int64_t)(current.tv_nsec / 1000000));
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

    aleph_g_stack_base = (uintptr_t)&stack_base;
    aleph_g_stack_base_set = true;
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
    /* The main thread has exited; do not guard against a stale stack base. */
    aleph_g_stack_base = 0;
    aleph_g_stack_base_set = false;

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
