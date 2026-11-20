/*
 * Tlön, the Aleph runtime (v0.1).
 *
 * This header is the contract between generated C code and the runtime.  It is
 * C99 + libc (plus pthreads for aleph_main_run), compiled with GCC or Clang:
 * checked arithmetic uses __builtin_*_overflow, so tcc is not supported.
 * See ADR-0024.
 *
 * Values are immutable and arena-allocated.  Generated code never frees.
 */
#ifndef TLON_H
#define TLON_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#if defined(__GNUC__)
#define AL_NORETURN __attribute__((noreturn))
#else
#define AL_NORETURN
#endif

/* Runtime limits, mirroring compiler/src/interp.rs (ADR-0021). */
#define AL_MAX_CALL_DEPTH 10000
#define AL_MAX_EVAL_DEPTH 20000
#define AL_MAX_RANGE_LEN 10000000
#define AL_MAIN_STACK_SIZE (256u * 1024u * 1024u)

typedef enum {
    AL_UNIT = 0,
    AL_I64,
    AL_F64,
    AL_BOOL,
    AL_STR,
    AL_LIST,
    AL_NONE,
    AL_SOME,
    AL_OK,
    AL_ERR,
    AL_RECORD,
    AL_FN,
    /* `dict K V` (ADR-0032): u.list holds k0 v0 k1 v1 ..., keys ascending. */
    AL_DICT,
    /* `json` (ADR-0033): u.box points to an AlJson whose first member is
       the payload: unit (null), bool, i64, f64, str, a list of AL_JSON or
       an AL_DICT from str to AL_JSON. */
    AL_JSON
} AlTag;

typedef struct AlValue AlValue;
typedef struct AlStr AlStr;
typedef struct AlList AlList;
typedef struct AlBox AlBox;
typedef struct AlRecord AlRecord;
typedef struct AlRecordInfo AlRecordInfo;
typedef struct AlFn AlFn;

struct AlValue {
    AlTag tag;
    union {
        int64_t i64;
        double f64;
        bool b;
        const AlStr *str;
        AlList *list;
        AlBox *box;
        AlRecord *rec;
        const AlFn *fn;
    } u;
};

struct AlStr {
    size_t len;
    const char *data;
};

struct AlList {
    size_t len;
    AlValue items[];
};

struct AlBox {
    AlValue value;
};

/* An AL_JSON payload. `u.box` points to the `box` member, so
   `u.box->value` reads the payload; the depth is reached by converting
   back to the enclosing struct (its first member). */
typedef struct AlJson {
    AlBox box;
    uint32_t depth; /* nested lists/objects; 0 for a scalar */
} AlJson;

/* `name` must have static storage duration (generated code uses literals). */
struct AlRecordInfo {
    const char *name;
    size_t field_count;
};

struct AlRecord {
    const AlRecordInfo *info;
    size_t n;
    AlValue fields[];
};

typedef enum {
    AL_FN_USER = 0,
    AL_FN_CTOR,
    AL_FN_BUILTIN,
    AL_FN_LAM
} AlFnKind;

/* Callback code: receives its own AlFn (for captured values) and the
 * arguments (ADR-0037). */
typedef AlValue (*AlFnCode)(const AlFn *, AlValue *, size_t);

struct AlFn {
    AlFnCode code;
    /* `name` must have static storage duration (generated code uses literals). */
    const char *name;
    AlFnKind kind;
    /* Values a lambda captured when it was evaluated; NULL/0 for `@f`. */
    const AlValue *captured;
    size_t ncaptured;
};

/*
 * Source location attached to evaluation frames and traps.
 *
 * aleph_eval_enter stores the pointer on the location stack, so it must point
 * to an AlLoc with static storage duration (generated code emits
 * `static const AlLoc` values) and stay valid until the matching leave.
 * aleph_call_enter does not store it: the pointer is only passed to the trap
 * writer if the call-depth or native stack guard fires.  A NULL location to
 * aleph_call_enter means "use the current location" (used by generated
 * map/filter/fold callbacks).
 */
typedef struct AlLoc {
    uint32_t node;
    uint32_t line;
    uint32_t col;
} AlLoc;

/* Filesystem and network capabilities (ADR-0020, ADR-0038). */
typedef enum {
    AL_CAP_FS_READ = 0,
    AL_CAP_FS_WRITE = 1,
    AL_CAP_NET_HOST = 2
} AlCap;

typedef struct AlUse {
    AlCap cap;
    const char *path; /* NULL when the capability has no path. */
} AlUse;

/*
 * Process-wide setup performed by generated main / the driver.
 *
 * These pointers are stored, not copied: `file`, the `AlUse` table and its
 * path strings, and the `argv` strings must remain valid for the whole
 * program (static storage duration is the normal choice).
 */
void aleph_set_file(const char *file);
void aleph_set_uses(const AlUse *uses, size_t count);
void aleph_set_args(int argc, const char *const *argv);

/* Arena allocation (ADR-0007): 16-byte aligned, freed only by regions
   (ADR-0039, below). */
void *aleph_alloc(size_t size);

/*
 * Evaluation depth accounting and the current source location (R004).
 * aleph_call_enter also checks the native stack guard installed by
 * aleph_main_run; outside the main thread the guard is disabled.
 */
void aleph_eval_enter(const AlLoc *loc);
void aleph_eval_leave(void);
void aleph_call_enter(const AlLoc *loc);
void aleph_call_leave(void);

/* Structured traps (SPEC §6/§9): write one JSON line to stderr, exit 70. */
void aleph_trap(const char *code, const char *msg) AL_NORETURN;
void aleph_trap_at(const char *code, const char *msg, const AlLoc *loc)
    AL_NORETURN;

/*
 * Fast path (ADR-0027).
 *
 * Generated code emits every function twice: an instrumented "slow" body that
 * mirrors interp.rs node by node (aleph_eval_enter/leave around every
 * expression) and a "fast" body with unboxed i64/f64/bool values and no
 * per-node accounting.  The fast body is taken only when the function's
 * static maximum evaluation depth cannot reach AL_MAX_EVAL_DEPTH from the
 * current depth, so it can never trap R004 on evaluation; user calls and
 * higher-order builtins still advance the counters by the exact number of
 * frames the interpreter would have open, so R004 traps stay byte-identical.
 */
#define AL_STACK_GUARD (1024u * 1024u)
#if defined(__GNUC__)
#define AL_COLD __attribute__((noinline, cold))
#define AL_UNLIKELY(x) __builtin_expect(!!(x), 0)
#else
#define AL_COLD
#define AL_UNLIKELY(x) (x)
#endif

extern size_t aleph_g_eval_depth;
extern size_t aleph_g_call_depth;
extern uintptr_t aleph_g_stack_base;
extern bool aleph_g_stack_base_set;
extern const AlLoc *aleph_g_locs[AL_MAX_EVAL_DEPTH + 1];
extern size_t aleph_g_loc_depth;

/* True when a fast body whose deepest node sits `max_depth` frames below the
 * function root cannot reach the evaluation limit. */
static inline bool aleph_fast_ok(size_t max_depth) {
    return aleph_g_eval_depth + max_depth < AL_MAX_EVAL_DEPTH;
}

/* A fast frame owns one slot on the location stack; it is set right before
 * each operation that can trap or that reads the current location. */
static inline void aleph_loc_push(void) {
    aleph_g_locs[aleph_g_loc_depth++] = NULL;
}
static inline void aleph_loc_pop(void) { aleph_g_loc_depth -= 1; }
static inline void aleph_here(const AlLoc *loc) {
    aleph_g_locs[aleph_g_loc_depth - 1] = loc;
}

/* `frames` is the number of evaluation frames the interpreter has open below
 * the caller's root when the callee's body starts (call node nesting + 2). */
static inline void aleph_fast_call_enter(const AlLoc *loc, size_t frames) {
    if (aleph_g_stack_base_set) {
        uintptr_t here = (uintptr_t)&here;
        uintptr_t used = here <= aleph_g_stack_base ? aleph_g_stack_base - here
                                                    : here - aleph_g_stack_base;
        if (AL_UNLIKELY(used > (uintptr_t)(AL_MAIN_STACK_SIZE - AL_STACK_GUARD))) {
            aleph_trap_at("R004", "stack overflow", loc);
        }
    }
    if (AL_UNLIKELY(aleph_g_call_depth >= AL_MAX_CALL_DEPTH)) {
        aleph_trap_at("R004", "stack overflow", loc);
    }
    aleph_g_call_depth += 1;
    aleph_g_eval_depth += frames;
}
static inline void aleph_fast_call_leave(size_t frames) {
    aleph_g_call_depth -= 1;
    aleph_g_eval_depth -= frames;
}

/* A lambda call mirrors interp `call_lam`: the depth check traps at the
 * current location (the map/filter/fold/map2 call), then only the call depth
 * grows; the body's own frames are counted by the body. */
static inline void aleph_lam_enter(void) {
    if (aleph_g_stack_base_set) {
        uintptr_t here = (uintptr_t)&here;
        uintptr_t used = here <= aleph_g_stack_base ? aleph_g_stack_base - here
                                                    : here - aleph_g_stack_base;
        if (AL_UNLIKELY(used > (uintptr_t)(AL_MAIN_STACK_SIZE - AL_STACK_GUARD))) {
            aleph_trap_at("R004", "stack overflow", NULL);
        }
    }
    if (AL_UNLIKELY(aleph_g_call_depth >= AL_MAX_CALL_DEPTH ||
                    aleph_g_eval_depth >= AL_MAX_EVAL_DEPTH)) {
        aleph_trap_at("R004", "stack overflow", NULL);
    }
    aleph_g_call_depth += 1;
}
static inline void aleph_lam_leave(void) { aleph_g_call_depth -= 1; }

/* Unboxed checked i64 arithmetic; traps match aleph_add_i64 & co. */
static inline int64_t aleph_fast_add(int64_t a, int64_t b) {
    int64_t r;
    if (AL_UNLIKELY(__builtin_add_overflow(a, b, &r))) {
        aleph_trap("R001", "integer overflow");
    }
    return r;
}
static inline int64_t aleph_fast_sub(int64_t a, int64_t b) {
    int64_t r;
    if (AL_UNLIKELY(__builtin_sub_overflow(a, b, &r))) {
        aleph_trap("R001", "integer overflow");
    }
    return r;
}
static inline int64_t aleph_fast_mul(int64_t a, int64_t b) {
    int64_t r;
    if (AL_UNLIKELY(__builtin_mul_overflow(a, b, &r))) {
        aleph_trap("R001", "integer overflow");
    }
    return r;
}
static inline int64_t aleph_fast_div(int64_t a, int64_t b) {
    if (AL_UNLIKELY(b == 0)) {
        aleph_trap("R002", "division by zero");
    }
    if (AL_UNLIKELY(a == INT64_MIN && b == -1)) {
        aleph_trap("R001", "integer overflow");
    }
    return a / b;
}
static inline int64_t aleph_fast_mod(int64_t a, int64_t b) {
    if (AL_UNLIKELY(b == 0)) {
        aleph_trap("R002", "division by zero");
    }
    if (AL_UNLIKELY(a == INT64_MIN && b == -1)) {
        aleph_trap("R001", "integer overflow");
    }
    return a % b;
}
static inline int64_t aleph_fast_neg(int64_t a) {
    if (AL_UNLIKELY(a == INT64_MIN)) {
        aleph_trap("R001", "integer overflow");
    }
    return -a;
}

/* Exact bit-pattern float literal without a function call. */
static inline double aleph_f64_from_bits(uint64_t bits) {
    union {
        uint64_t u;
        double d;
    } pun;
    pun.u = bits;
    return pun.d;
}

/* Inline boxing for the fast path (same values as aleph_i64 & co). */
static inline AlValue aleph_box_i64(int64_t v) {
    AlValue r;
    r.tag = AL_I64;
    r.u.i64 = v;
    return r;
}
static inline AlValue aleph_box_f64(double v) {
    AlValue r;
    r.tag = AL_F64;
    r.u.f64 = v;
    return r;
}
static inline AlValue aleph_box_bool(bool v) {
    AlValue r;
    r.tag = AL_BOOL;
    r.u.b = v;
    return r;
}

/*
 * Scoped regions (ADR-0039).  A user function body saves the arena top at
 * entry (aleph_region_mark) and, at return, either resets the arena to it
 * (pointer-free result, or a result whose root is older than the mark),
 * compacts a small result out of a large region, or leaves the region to the
 * caller.  Invariant: an object allocated before a mark never points to one
 * allocated after it.
 */
typedef struct AlChunk {
    struct AlChunk *next; /* older chunk */
    size_t used;
    size_t cap;
    unsigned char *data;
} AlChunk;

typedef struct AlMark {
    AlChunk *chunk;
    size_t used;
    size_t base; /* aleph_g_live_base at the mark */
} AlMark;

extern AlChunk *aleph_g_chunks; /* newest chunk first */
/* Live arena bytes are aleph_g_live_base + aleph_g_chunks->used: the used
   bytes of every chunk but the newest.  aleph_g_freed counts the bytes
   regions have given back, so live + freed only grows (bytes allocated). */
extern size_t aleph_g_live_base;
extern size_t aleph_g_freed;

void aleph_region_free_to(AlMark mark);
AlValue aleph_region_leave(AlMark mark, AlValue result);
/* Bytes of chunk capacity currently held by the arena (for the tests). */
size_t aleph_arena_bytes(void);
/* Bytes scanned by failed compaction attempts so far (for the tests). */
size_t aleph_region_scanned(void);

static inline AlMark aleph_region_mark(void) {
    AlMark mark;
    mark.chunk = aleph_g_chunks;
    mark.used = mark.chunk != NULL ? mark.chunk->used : 0;
    mark.base = aleph_g_live_base;
    return mark;
}
static inline void aleph_region_reset(AlMark mark) {
    if (AL_UNLIKELY(aleph_g_chunks != mark.chunk)) {
        aleph_region_free_to(mark);
    } else if (mark.chunk != NULL) {
        aleph_g_freed += mark.chunk->used - mark.used;
        mark.chunk->used = mark.used;
    }
}
static inline AlValue aleph_region_exit(AlMark mark, AlValue result) {
    switch (result.tag) {
    case AL_UNIT:
    case AL_I64:
    case AL_F64:
    case AL_BOOL:
    case AL_NONE:
        aleph_region_reset(mark);
        return result;
    default:
        return aleph_region_leave(mark, result);
    }
}

/* Value constructors. */
AlValue aleph_unit(void);
AlValue aleph_i64(int64_t value);
AlValue aleph_f64(double value);
/* Exact bit-pattern constructor used for generated float literals. */
AlValue aleph_f64_bits(uint64_t bits);
AlValue aleph_bool(bool value);
AlValue aleph_str(const char *text);
AlValue aleph_str_n(const char *text, size_t len);
AlValue aleph_list(const AlValue *items, size_t len);
AlValue aleph_list_new(size_t len);
AlValue aleph_none(void);
AlValue aleph_some(AlValue value);
AlValue aleph_ok(AlValue value);
AlValue aleph_err(AlValue value);
AlValue aleph_record(const AlRecordInfo *info, const AlValue *fields, size_t count);
AlValue aleph_fn(AlFnCode code, const char *name);
AlValue aleph_fnval_kind(AlFnCode code, const char *name, AlFnKind kind);
/* A lambda value: copies `count` captured values into the arena. */
AlValue aleph_fnval_lam(AlFnCode code, const AlValue *captured, size_t count);
/* Wrap a generated static const AlFn without copying it. */
AlValue aleph_fnval_static(const AlFn *fn);

/* Checked i64 arithmetic (R001/R002). */
AlValue aleph_add_i64(AlValue left, AlValue right);
AlValue aleph_sub_i64(AlValue left, AlValue right);
AlValue aleph_mul_i64(AlValue left, AlValue right);
AlValue aleph_div_i64(AlValue left, AlValue right);
AlValue aleph_mod_i64(AlValue left, AlValue right);
AlValue aleph_neg_i64(AlValue value);

/* Structural equality and primitive comparisons (ADR-0021). */
bool aleph_values_equal(AlValue left, AlValue right);
bool aleph_lt(AlValue left, AlValue right);
bool aleph_le(AlValue left, AlValue right);
bool aleph_gt(AlValue left, AlValue right);
bool aleph_ge(AlValue left, AlValue right);

/* show / literal; both return a fresh AL_STR value. */
AlValue aleph_show(AlValue value);
AlValue aleph_literal(AlValue value);

/* Non-higher-order builtins (SPEC §5.1). */
AlValue aleph_b_len(AlValue xs);
AlValue aleph_b_at(AlValue xs, AlValue index);
AlValue aleph_b_push(AlValue xs, AlValue value);
AlValue aleph_b_slice(AlValue xs, AlValue start, AlValue end);
AlValue aleph_b_rev(AlValue xs);
AlValue aleph_b_sort(AlValue xs);
AlValue aleph_dict_empty(void);
AlValue aleph_b_put(AlValue dict, AlValue key, AlValue value);
AlValue aleph_b_get(AlValue dict, AlValue key);
AlValue aleph_b_del(AlValue dict, AlValue key);
AlValue aleph_b_keys(AlValue dict);
AlValue aleph_b_vals(AlValue dict);
AlValue aleph_b_size(AlValue dict);
AlValue aleph_b_json(AlValue value);
AlValue aleph_b_jparse(AlValue text);
AlValue aleph_b_jbool(AlValue json);
AlValue aleph_b_jint(AlValue json);
AlValue aleph_b_jnum(AlValue json);
AlValue aleph_b_jstr(AlValue json);
AlValue aleph_b_jlist(AlValue json);
AlValue aleph_b_jdict(AlValue json);
AlValue aleph_b_range(AlValue start, AlValue end);
AlValue aleph_b_sum(AlValue xs);
AlValue aleph_b_map(AlValue function, AlValue xs);
AlValue aleph_b_filter(AlValue function, AlValue xs);
AlValue aleph_b_fold(AlValue function, AlValue init, AlValue xs);
/* ADR-0036 */
AlValue aleph_b_abs(AlValue x);
AlValue aleph_b_min(AlValue left, AlValue right);
AlValue aleph_b_max(AlValue left, AlValue right);
AlValue aleph_b_starts(AlValue text, AlValue prefix);
AlValue aleph_b_ord(AlValue text);
AlValue aleph_b_chr(AlValue code);
AlValue aleph_b_ends(AlValue text, AlValue suffix);
AlValue aleph_b_uniq(AlValue xs);
AlValue aleph_b_map2(AlValue function, AlValue left, AlValue right);
AlValue aleph_b_cat(AlValue left, AlValue right);
AlValue aleph_b_count(AlValue text);
AlValue aleph_b_split(AlValue text, AlValue separator);
AlValue aleph_b_join(AlValue parts, AlValue separator);
AlValue aleph_b_trim(AlValue text);
AlValue aleph_b_upper(AlValue text);
AlValue aleph_b_lower(AlValue text);
AlValue aleph_b_contains(AlValue text, AlValue needle);
AlValue aleph_b_show(AlValue value);
AlValue aleph_b_int(AlValue text);
AlValue aleph_b_float(AlValue text);
AlValue aleph_b_args(void);
AlValue aleph_b_print(AlValue text);
AlValue aleph_b_eprint(AlValue text);
AlValue aleph_b_env(AlValue name);
AlValue aleph_b_now(void);
AlValue aleph_b_read(AlValue path);
AlValue aleph_b_fetch(AlValue url);
AlValue aleph_b_write(AlValue path, AlValue text);

/* Capability path check (exposed for the tests and future codegen). */
bool aleph_path_allowed(AlCap cap, const char *path);

/*
 * Run main on a pthread with a 256 MiB stack.
 *
 * `argc`/`argv` are the program arguments (excluding the executable name).
 * Returns 0 for unit/ok, 1 for `err` (printed to stderr), or exits 70 on a
 * trap.
 */
int aleph_main_run(AlValue (*main_code)(AlValue *, size_t), int argc,
                   const char *const *argv);

#endif /* TLON_H */
