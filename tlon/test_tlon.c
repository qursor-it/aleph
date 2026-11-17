#define _POSIX_C_SOURCE 200809L

/*
 * Tlön runtime self-tests.  Built and run by compiler/tests/tlon.rs.
 *
 * With no arguments the driver runs the in-process checks plus a set of
 * forked trap cases.  With `--format` it reads one hexadecimal f64 bit pattern
 * per line and prints `show` for each value (used for Rust {:?} parity).
 */
#include "tlon.h"

#include <errno.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>

static int failures;

static void fail(const char *what, const char *detail) {
    fprintf(stderr, "FAIL: %s%s%s\n", what, detail != NULL ? ": " : "",
            detail != NULL ? detail : "");
    failures += 1;
}

static void check(bool condition, const char *what) {
    if (!condition) {
        fail(what, NULL);
    }
}

static void check_i64(AlValue value, int64_t expected, const char *what) {
    if (value.tag != AL_I64 || value.u.i64 != expected) {
        char detail[128];
        snprintf(detail, sizeof(detail), "expected i64 %lld, got tag %d",
                 (long long)expected, (int)value.tag);
        fail(what, detail);
    }
}

static void check_str(AlValue value, const char *expected, const char *what) {
    size_t expected_len = strlen(expected);
    if (value.tag != AL_STR) {
        char detail[128];
        snprintf(detail, sizeof(detail), "expected str, got tag %d",
                 (int)value.tag);
        fail(what, detail);
        return;
    }
    if (value.u.str->len != expected_len ||
        memcmp(value.u.str->data, expected, expected_len) != 0) {
        char detail[512];
        snprintf(detail, sizeof(detail), "expected \"%s\", got \"%.*s\"",
                 expected, (int)value.u.str->len, value.u.str->data);
        fail(what, detail);
    }
}

static void check_bool(AlValue value, bool expected, const char *what) {
    if (value.tag != AL_BOOL || value.u.b != expected) {
        fail(what, "unexpected bool value");
    }
}

static void check_show(AlValue value, const char *expected, const char *what) {
    check_str(aleph_show(value), expected, what);
}

static AlValue list2(AlValue first, AlValue second) {
    AlValue items[2];
    items[0] = first;
    items[1] = second;
    return aleph_list(items, 2);
}

static AlValue list3(AlValue first, AlValue second, AlValue third) {
    AlValue items[3];
    items[0] = first;
    items[1] = second;
    items[2] = third;
    return aleph_list(items, 3);
}

static AlValue i64v(int64_t number) { return aleph_i64(number); }
static AlValue strv(const char *text) { return aleph_str(text); }

static AlValue dummy_fn(AlValue *args, size_t count) {
    (void)args;
    (void)count;
    return aleph_unit();
}

static AlValue hof_inc(AlValue *args, size_t count) {
    (void)count;
    return aleph_i64(args[0].u.i64 + 1);
}

static AlValue hof_positive(AlValue *args, size_t count) {
    (void)count;
    return aleph_bool(args[0].u.i64 > 0);
}

static AlValue hof_add(AlValue *args, size_t count) {
    (void)count;
    return aleph_i64(args[0].u.i64 + args[1].u.i64);
}

/* The static AlFn values used through aleph_fn_static. */
static const AlFn hof_inc_fn = {hof_inc, "hof_inc", AL_FN_USER};
static const AlFn hof_add_fn = {hof_add, "hof_add", AL_FN_USER};

/* ------------------------------------------------------------------ */
/* show / literal                                                      */
/* ------------------------------------------------------------------ */

static void test_show(void) {
    check_show(aleph_i64(42), "42", "show i64");
    check_show(aleph_f64(1.0), "1.0", "show f64 integral");
    check_show(aleph_f64(1.5), "1.5", "show f64 fraction");
    check_show(aleph_f64(-0.0), "-0.0", "show f64 negative zero");
    check_show(aleph_f64(0.0), "0.0", "show f64 zero");
    check_show(aleph_f64(1.0 / 0.0), "inf", "show f64 inf");
    check_show(aleph_f64(-1.0 / 0.0), "-inf", "show f64 -inf");
    check_show(aleph_f64(0.0 / 0.0), "NaN", "show f64 NaN");
    check_show(aleph_f64(0.0001), "0.0001", "show f64 1e-4");
    check_show(aleph_f64(0.00001), "1e-5", "show f64 1e-5");
    check_show(aleph_f64(1e16), "1e16", "show f64 1e16");
    check_show(aleph_f64(1e15), "1000000000000000.0", "show f64 1e15");
    check_show(aleph_bool(true), "true", "show bool");
    check_show(aleph_unit(), "nil", "show unit");
    check_show(strv("hi"), "hi", "show top-level str");
    check_show(list3(i64v(1), strv("a b"), strv("c\nd")),
               "[1 \"a b\" \"c\\nd\"]", "show list literal quoting");
    check_show(aleph_list_new(0), "[]", "show empty list");
    check_show(aleph_none(), "none", "show none");
    check_show(aleph_some(i64v(3)), "some 3", "show some");
    check_show(aleph_ok(strv("x")), "ok \"x\"", "show ok");
    check_show(aleph_err(strv("x")), "err \"x\"", "show err");
    check_show(aleph_fn(dummy_fn, "f"), "@f", "show fn");
    check(aleph_values_equal(aleph_fn(dummy_fn, "f"),
                             aleph_fn(dummy_fn, "f")),
          "function equality same kind");
    check(!aleph_values_equal(aleph_fn(dummy_fn, "f"),
                              aleph_fn_kind(dummy_fn, "f", AL_FN_CTOR)),
          "function equality kind differs");
    check(aleph_values_equal(aleph_fn_kind(dummy_fn, "f", AL_FN_BUILTIN),
                             aleph_fn_kind(dummy_fn, "f", AL_FN_BUILTIN)),
          "function equality builtin kind");
    check(!aleph_values_equal(aleph_fn(dummy_fn, "f"),
                              aleph_fn(dummy_fn, "g")),
          "function equality name differs");

    {
        AlRecordInfo point = {"point", 2};
        AlValue fields[2];
        fields[0] = i64v(1);
        fields[1] = i64v(2);
        check_show(aleph_record(&point, fields, 2), "point 1 2", "show record");
        check_show(aleph_record(&point, NULL, 2), "point nil nil",
                   "record NULL fields zeroed");
    }
    {
        AlValue text = aleph_str_n(NULL, 3);
        check(text.tag == AL_STR && text.u.str->len == 3 &&
                  text.u.str->data[0] == '\0' &&
                  text.u.str->data[1] == '\0' &&
                  text.u.str->data[2] == '\0',
              "str_n NULL data zeroed");
    }

    check_str(aleph_literal(strv("hi")), "\"hi\"", "literal quotes str");
    check_str(aleph_literal(strv("a\\b\"c\nd\te")),
              "\"a\\\\b\\\"c\\nd\\te\"", "literal escapes str");
    check_str(aleph_literal(list2(strv("a"), i64v(1))), "[\"a\" 1]",
              "literal list");
}

/* ------------------------------------------------------------------ */
/* Builtins                                                            */
/* ------------------------------------------------------------------ */

static void test_builtins(void) {
    AlValue xs = list3(i64v(1), i64v(2), i64v(3));
    AlValue negative = i64v(-1);
    AlValue huge = i64v(99);
    AlValue result;

    check_i64(aleph_b_len(xs), 3, "len");
    check_i64(aleph_b_len(aleph_list_new(0)), 0, "len empty");

    result = aleph_b_at(xs, i64v(1));
    check(result.tag == AL_SOME && result.u.box->value.u.i64 == 2, "at valid");
    check(aleph_b_at(xs, negative).tag == AL_NONE, "at negative");
    check(aleph_b_at(xs, huge).tag == AL_NONE, "at past end");

    check_show(aleph_b_push(xs, i64v(4)), "[1 2 3 4]", "push");
    check_show(aleph_b_slice(xs, i64v(1), i64v(3)), "[2 3]", "slice");
    check_show(aleph_b_slice(xs, i64v(-5), i64v(99)), "[1 2 3]",
               "slice clamped");
    check_show(aleph_b_slice(xs, i64v(2), i64v(1)), "[]", "slice reversed");
    check_show(aleph_b_rev(xs), "[3 2 1]", "rev");

    check_show(aleph_b_sort(list3(i64v(3), i64v(1), i64v(2))), "[1 2 3]",
               "sort i64");
    check_show(aleph_b_sort(list3(aleph_f64(2.5), aleph_f64(-1.0),
                                 aleph_f64(0.5))),
               "[-1.0 0.5 2.5]", "sort f64");
    check_show(aleph_b_sort(list3(aleph_f64(0.0), aleph_f64(-0.0),
                                 aleph_f64(1.0))),
               "[-0.0 0.0 1.0]", "sort f64 total order zeros");
    {
        AlValue numbers[6];
        numbers[0] = aleph_f64(0.0 / 0.0);
        numbers[1] = aleph_f64(1.0);
        numbers[2] = aleph_f64(-1.0 / 0.0);
        numbers[3] = aleph_f64(1.0 / 0.0);
        numbers[4] = aleph_f64(-0.0);
        numbers[5] = aleph_f64(0.0);
        check_show(aleph_b_sort(aleph_list(numbers, 6)),
                   "[-inf -0.0 0.0 1.0 inf NaN]",
                   "sort f64 total order NaN");
    }
    {
        AlValue numbers[8];
        numbers[0] = aleph_f64(0.0 / 0.0);
        numbers[1] = aleph_f64(-(0.0 / 0.0));
        numbers[2] = aleph_f64(-1.0 / 0.0);
        numbers[3] = aleph_f64(1.0 / 0.0);
        numbers[4] = aleph_f64(0.0);
        numbers[5] = aleph_f64(-0.0);
        numbers[6] = aleph_f64(1.0);
        numbers[7] = aleph_f64(-1.0);
        check_show(aleph_b_sort(aleph_list(numbers, 8)),
                   "[-inf -1.0 -0.0 0.0 1.0 inf NaN NaN]",
                   "sort f64: every NaN last, whatever its sign");
    }
    check_show(aleph_b_sort(list3(strv("b"), strv("a"), strv("c"))),
               "[\"a\" \"b\" \"c\"]", "sort str");
    check_show(aleph_b_sort(list3(aleph_bool(true), aleph_bool(false),
                                 aleph_bool(true))),
               "[false true true]", "sort bool");

    check_show(aleph_b_range(i64v(0), i64v(3)), "[0 1 2]", "range");
    check_show(aleph_b_range(i64v(3), i64v(3)), "[]", "range empty");
    check_show(aleph_b_range(i64v(-2), i64v(1)), "[-2 -1 0]", "range negative");
    check_i64(aleph_b_sum(list3(i64v(1), i64v(2), i64v(3))), 6, "sum");
    check_i64(aleph_b_sum(aleph_list_new(0)), 0, "sum empty");

    check_str(aleph_b_cat(strv("ab"), strv("cd")), "abcd", "cat");
    check_i64(aleph_b_count(strv("abc")), 3, "count ascii");
    check_i64(aleph_b_count(strv("\xC3\xA9")), 2, "count bytes");

    check_show(aleph_b_split(strv("a,b"), strv(",")), "[\"a\" \"b\"]",
               "split separator");
    check_show(aleph_b_split(strv("abc"), strv("")),
               "[\"a\" \"b\" \"c\"]", "split empty separator");
    check_show(aleph_b_split(strv(""), strv("")), "[]", "split empty empty");
    check_show(aleph_b_split(strv(""), strv(",")), "[\"\"]",
               "split empty with separator");
    check_show(aleph_b_split(strv(",a,"), strv(",")),
               "[\"\" \"a\" \"\"]", "split leading and trailing");
    check_show(aleph_b_split(strv("\xC3\xA9x"), strv("")),
               "[\"\xC3\xA9\" \"x\"]", "split Unicode scalar");

    check_str(aleph_b_join(list2(strv("a"), strv("b")), strv("-")), "a-b",
              "join");
    check_str(aleph_b_join(aleph_list_new(0), strv("-")), "", "join empty");

    check_str(aleph_b_trim(strv("  hi \t")), "hi", "trim ASCII");
    check_str(aleph_b_trim(strv("\xC2\xA0hi\xE2\x80\x83")), "hi",
              "trim Unicode whitespace");
    check_str(aleph_b_trim(strv("\xC2\xA0")), "", "trim all whitespace");

    check_str(aleph_b_upper(strv("aBc!")), "ABC!", "upper ASCII");
    check_str(aleph_b_lower(strv("aBc!")), "abc!", "lower ASCII");
    check_str(aleph_b_upper(strv("\xC3\xA9")), "\xC3\xA9",
              "upper leaves non-ASCII");

    check_bool(aleph_b_contains(strv("abc"), strv("b")), true, "contains yes");
    check_bool(aleph_b_contains(strv("abc"), strv("z")), false,
               "contains no");
    check_bool(aleph_b_contains(strv("abc"), strv("")), true,
               "contains empty");

    check_show(aleph_b_int(strv("42")), "ok 42", "int positive");
    check_show(aleph_b_int(strv("-7")), "ok -7", "int negative");
    check_show(aleph_b_int(strv("007")), "ok 7", "int leading zeros");
    check_show(aleph_b_int(strv("-0")), "ok 0", "int negative zero");
    check_show(aleph_b_int(strv("")), "err \"invalid int\"", "int empty");
    check_show(aleph_b_int(strv("+1")), "err \"invalid int\"", "int plus");
    check_show(aleph_b_int(strv("1.0")), "err \"invalid int\"", "int decimal");
    check_show(aleph_b_int(strv("999999999999999999999999999")),
               "err \"invalid int\"", "int overflow");
    check_show(aleph_b_int(strv("-9223372036854775808")),
               "ok -9223372036854775808", "int min");
    check_show(aleph_b_int(strv("9223372036854775808")),
               "err \"invalid int\"", "int max plus one");

    check_show(aleph_b_float(strv("1.5")), "ok 1.5", "float");
    check_show(aleph_b_float(strv("-0.50")), "ok -0.5", "float negative");
    check_show(aleph_b_float(strv("1")), "err \"invalid float\"",
               "float missing dot");
    check_show(aleph_b_float(strv("1.")), "err \"invalid float\"",
               "float missing fraction");
    check_show(aleph_b_float(strv(".5")), "err \"invalid float\"",
               "float missing whole");
    check_show(aleph_b_float(strv("1e3")), "err \"invalid float\"",
               "float exponent rejected");
    check_show(aleph_b_float(strv("")), "err \"invalid float\"", "float empty");
    check_show(aleph_b_float(strv("1.2.3")), "err \"invalid float\"",
               "float two dots");

    {
        const char *argv[] = {"a", "b"};
        aleph_set_args(2, argv);
        check_show(aleph_b_args(), "[\"a\" \"b\"]", "args");
        aleph_set_args(0, NULL);
    }
}

/* ------------------------------------------------------------------ */
/* Higher-order builtins and function values                           */
/* ------------------------------------------------------------------ */

/* Same cases as compiler/src/json.rs tests: the engines must agree. */
static void check_json(const char *input, const char *expected) {
    AlValue result = aleph_b_jparse(strv(input));
    AlValue text = result.tag == AL_OK ? aleph_show(result.u.box->value)
                                       : result.u.box->value;
    check_str(text, expected, input);
}

static void test_json(void) {
    char deep[2 * 256 + 2];
    char deeper[258];
    volatile double zero = 0.0;
    check_json(" {\"b\" : [1, 2.5e0, -0, true, null], \"a\":\"x\\u00e9\\n\"} ",
               "{\"a\":\"x\xc3\xa9\\n\",\"b\":[1,2.5,0,true,null]}");
    check_json("1e400", "json: bad number at byte 0");
    check_json("12345678901234567890", "1.2345678901234567e19");
    check_json("-9223372036854775808", "-9223372036854775808");
    check_json("\"\\ud83d\\ude00\\u0001\"", "\"\xf0\x9f\x98\x80\\u0001\"");
    check_json("[1e2,1E-2,0.5]", "[100.0,0.01,0.5]");
    check_json("[-0.0,1e15,1e16,\"\\u0000\\/\"]",
               "[-0.0,1000000000000000.0,1e16,\"\\u0000/\"]");
    check_json("\"\\u00", "json: bad escape at byte 1");
    check_json("{\"b\":1,\"a\":2,\"b\":3,\"a\":4}",
               "json: duplicate key at byte 13");
    check_json("{\"a\":1,\"a\":2,\"a\":3", "json: unexpected end at byte 18");

    check_json("", "json: unexpected end at byte 0");
    check_json("[1,]", "json: unexpected byte at byte 3");
    check_json("[1 2]", "json: unexpected byte at byte 3");
    check_json("{\"a\":1", "json: unexpected end at byte 6");
    check_json("tru", "json: unexpected byte at byte 0");
    check_json("01", "json: unexpected byte at byte 1");
    check_json("-", "json: bad number at byte 0");
    check_json("1.", "json: bad number at byte 0");
    check_json("[\"a\\x\"]", "json: bad escape at byte 3");
    check_json("\"\\ud800\"", "json: bad escape at byte 1");
    check_json("\"\\udc00\"", "json: bad escape at byte 1");
    check_json("\"\\u12\"", "json: bad escape at byte 1");
    check_json("\"a\nb\"", "json: unexpected byte at byte 2");
    check_json("{\"a\":1,\"a\":2}", "json: duplicate key at byte 7");
    check_json("{1:2}", "json: unexpected byte at byte 1");
    check_json("1 x", "json: unexpected byte at byte 2");

    memset(deep, '[', 256);
    memset(deep + 256, ']', 256);
    deep[512] = '\0';
    check_json(deep, deep);
    memset(deeper, '[', 257);
    deeper[257] = '\0';
    check_json(deeper, "json: too deep at byte 256");

    check_show(aleph_b_json(aleph_f64(0.0 / zero)), "null", "json of NaN");
    check_show(aleph_b_json(aleph_f64(1.0)), "1.0", "json of f64");
    check(aleph_b_jint(aleph_b_json(aleph_f64(1.0))).tag == AL_NONE,
          "jint of an f64");
    check_show(aleph_b_jnum(aleph_b_json(i64v(3))), "some 3.0", "jnum of i64");
    check_show(aleph_b_json(aleph_b_json(i64v(1))), "1", "json of json");
    {
        /* 256 levels are allowed; the 257th traps (tested in test_traps). */
        AlValue value = aleph_b_json(aleph_unit());
        int level;
        for (level = 0; level < 256; level += 1) {
            AlValue items[1];
            items[0] = value;
            value = aleph_b_json(aleph_list(items, 1));
        }
        check(((const AlJson *)value.u.box)->depth == 256,
              "json depth 256");
        check(((const AlJson *)aleph_b_json(aleph_list(NULL, 0)).u.box)
                      ->depth == 1,
              "empty list depth 1");
    }
}

static void test_dict(void) {
    AlValue d = aleph_dict_empty();
    check_i64(aleph_b_size(d), 0, "dict empty size");
    check_i64(aleph_b_size(aleph_b_del(d, i64v(1))), 0, "dict del on empty");
    check(aleph_b_get(d, i64v(1)).tag == AL_NONE, "dict get on empty");

    /* Insert out of order: the middle, then the front, then the end. */
    d = aleph_b_put(d, i64v(5), aleph_str("five"));
    d = aleph_b_put(d, i64v(-3), aleph_str("minus"));
    d = aleph_b_put(d, i64v(INT64_MAX), aleph_str("max"));
    d = aleph_b_put(d, i64v(0), aleph_str("zero"));
    check_show(aleph_b_keys(d), "[-3 0 5 9223372036854775807]", "dict keys");
    check_show(aleph_b_vals(d), "[\"minus\" \"zero\" \"five\" \"max\"]",
               "dict vals");

    /* Replace keeps the size; the old dict is unchanged. */
    AlValue replaced = aleph_b_put(d, i64v(5), aleph_str("FIVE"));
    check_i64(aleph_b_size(replaced), 4, "dict replace size");
    check_show(aleph_b_vals(replaced),
               "[\"minus\" \"zero\" \"FIVE\" \"max\"]", "dict replace");
    check_show(aleph_b_vals(d), "[\"minus\" \"zero\" \"five\" \"max\"]",
               "dict immutable");

    /* Delete at the front, the middle, the end, and an absent key. */
    check_show(aleph_b_keys(aleph_b_del(d, i64v(-3))),
               "[0 5 9223372036854775807]", "dict del front");
    check_show(aleph_b_keys(aleph_b_del(d, i64v(5))),
               "[-3 0 9223372036854775807]", "dict del middle");
    check_show(aleph_b_keys(aleph_b_del(d, i64v(INT64_MAX))), "[-3 0 5]",
               "dict del end");
    check_i64(aleph_b_size(aleph_b_del(d, i64v(7))), 4, "dict del absent");

    AlValue found = aleph_b_get(d, i64v(0));
    check(found.tag == AL_SOME, "dict get present");
    check(aleph_b_get(d, i64v(1)).tag == AL_NONE, "dict get absent");

    /* str keys order by bytes: uppercase before lowercase, ASCII before UTF-8. */
    AlValue s = aleph_dict_empty();
    s = aleph_b_put(s, aleph_str("b"), i64v(1));
    s = aleph_b_put(s, aleph_str("\xc3\xa9"), i64v(2));
    s = aleph_b_put(s, aleph_str("B"), i64v(3));
    s = aleph_b_put(s, aleph_str(""), i64v(4));
    s = aleph_b_put(s, aleph_str("ab"), i64v(5));
    check_show(aleph_b_vals(s), "[4 3 5 1 2]", "dict str byte order");
}

static void test_higher_order(void) {
    AlValue xs = list3(i64v(1), i64v(2), i64v(3));
    AlValue static_inc = aleph_fn_static(&hof_inc_fn);

    check(static_inc.tag == AL_FN, "fn_static tag");
    check(static_inc.u.fn == &hof_inc_fn,
          "fn_static points at the static AlFn");
    check(aleph_values_equal(static_inc, aleph_fn(hof_inc, "hof_inc")),
          "fn_static equality with equal AlFn");
    check_show(aleph_b_map(static_inc, xs), "[2 3 4]", "map via fn_static");
    check_show(aleph_b_map(aleph_fn(hof_inc, "hof_inc"), xs), "[2 3 4]",
               "map via aleph_fn");
    check_show(aleph_b_filter(aleph_fn(hof_positive, "hof_positive"),
                              list3(i64v(-1), i64v(0), i64v(2))),
               "[2]", "filter");
    check_show(aleph_b_fold(aleph_fn_static(&hof_add_fn), i64v(0), xs), "6",
               "fold via fn_static");
    check_show(aleph_b_fold(aleph_fn(hof_add, "hof_add"), i64v(10), xs), "16",
               "fold via aleph_fn");
    check_show(aleph_b_map(static_inc, aleph_list_new(0)), "[]",
               "map empty");
    check_show(aleph_b_filter(aleph_fn(hof_positive, "hof_positive"),
                              aleph_list_new(0)),
               "[]", "filter empty");
    check_show(aleph_b_fold(aleph_fn_static(&hof_add_fn), i64v(7),
                            aleph_list_new(0)),
               "7", "fold empty");
}

/* ------------------------------------------------------------------ */
/* Filesystem                                                          */
/* ------------------------------------------------------------------ */

static void test_filesystem(void) {
    char directory[] = "/tmp/aleph-tlon-test-XXXXXX";
    char inside[512];
    char missing[512];
    char outside[] = "/etc/hostname";
    char *made = mkdtemp(directory);
    AlUse uses[2];
    AlValue written;
    AlValue read_back;
    FILE *raw;

    if (made == NULL) {
        fail("mkdtemp", strerror(errno));
        return;
    }
    snprintf(inside, sizeof(inside), "%s/out.txt", made);
    snprintf(missing, sizeof(missing), "%s/nope.txt", made);
    uses[0].cap = AL_CAP_FS_READ;
    uses[0].path = made;
    uses[1].cap = AL_CAP_FS_WRITE;
    uses[1].path = made;
    aleph_set_uses(uses, 2);
    aleph_set_file("fs.aph");

    check(aleph_path_allowed(AL_CAP_FS_READ, inside), "path inside prefix");
    check(!aleph_path_allowed(AL_CAP_FS_READ, outside),
          "path outside prefix");
    {
        char slash[520];
        snprintf(slash, sizeof(slash), "%s/", inside);
        check(aleph_path_allowed(AL_CAP_FS_READ, slash),
              "trailing slash inside prefix");
        snprintf(slash, sizeof(slash), "%s/", outside);
        check(!aleph_path_allowed(AL_CAP_FS_READ, slash),
              "trailing slash outside prefix");
    }
    {
        char parent[512];
        snprintf(parent, sizeof(parent), "%s/../etc/passwd", made);
        check(!aleph_path_allowed(AL_CAP_FS_READ, parent),
              "path with parent dir rejected");
    }

    {
        char raw[512];
        size_t base = strlen(made);
        size_t length;
        AlValue nul_path;
        memcpy(raw, made, base);
        memcpy(raw + base, "/bad", 4);
        raw[base + 4] = '\0';
        memcpy(raw + base + 5, "name", 4);
        length = base + 9;
        nul_path = aleph_str_n(raw, length);
        check_show(aleph_b_read(nul_path), "err \"invalid input\"",
                   "read embedded NUL");
        check_show(aleph_b_write(nul_path, strv("x")),
                   "err \"invalid input\"", "write embedded NUL");
    }

    written = aleph_b_write(strv(inside), strv("hello"));
    check(written.tag == AL_OK, "write ok");
    read_back = aleph_b_read(strv(inside));
    check(read_back.tag == AL_OK, "read ok");
    if (read_back.tag == AL_OK) {
        check_str(read_back.u.box->value, "hello", "read back contents");
    }

    check_show(aleph_b_read(strv(missing)), "err \"not found\"",
               "read missing");
    check_show(aleph_b_read(strv(made)), "err \"is a directory\"",
               "read directory");

    raw = fopen(inside, "wb");
    if (raw != NULL) {
        fputc(0xFF, raw);
        fclose(raw);
        check_show(aleph_b_read(strv(inside)), "err \"invalid data\"",
                   "read invalid utf8");
    } else {
        fail("fopen invalid utf8", strerror(errno));
    }

    {
        char link_path[512];
        char target_path[512];
        int linked;
        snprintf(link_path, sizeof(link_path), "%s/link.txt", made);
        snprintf(target_path, sizeof(target_path), "/tmp/aleph-tlon-outside-%ld",
                 (long)getpid());
        linked = symlink(target_path, link_path);
        if (linked == 0) {
            check(!aleph_path_allowed(AL_CAP_FS_READ, link_path),
                  "dangling symlink rejected");
            unlink(link_path);
        }
    }

    unlink(inside);
    rmdir(made);
    aleph_set_uses(NULL, 0);
}

/* ------------------------------------------------------------------ */
/* print / eprint and main results                                     */
/* ------------------------------------------------------------------ */

static int run_capture(void (*callback)(void), char *out, size_t out_cap,
                       char *err, size_t err_cap, int *exit_code) {
    int out_pipe[2];
    int err_pipe[2];
    pid_t child;
    ssize_t count;
    size_t used;
    int status = 0;

    out[0] = '\0';
    err[0] = '\0';
    if (pipe(out_pipe) != 0 || pipe(err_pipe) != 0) {
        fail("pipe", strerror(errno));
        return 0;
    }
    child = fork();
    if (child < 0) {
        fail("fork", strerror(errno));
        close(out_pipe[0]);
        close(out_pipe[1]);
        close(err_pipe[0]);
        close(err_pipe[1]);
        return 0;
    }
    if (child == 0) {
        close(out_pipe[0]);
        close(err_pipe[0]);
        dup2(out_pipe[1], STDOUT_FILENO);
        dup2(err_pipe[1], STDERR_FILENO);
        close(out_pipe[1]);
        close(err_pipe[1]);
        callback();
        fflush(NULL);
        _exit(0);
    }
    close(out_pipe[1]);
    close(err_pipe[1]);

    used = 0;
    while ((count = read(out_pipe[0], out + used, out_cap - 1 - used)) > 0) {
        used += (size_t)count;
        if (used + 1 >= out_cap) {
            break;
        }
    }
    out[used] = '\0';
    used = 0;
    while ((count = read(err_pipe[0], err + used, err_cap - 1 - used)) > 0) {
        used += (size_t)count;
        if (used + 1 >= err_cap) {
            break;
        }
    }
    err[used] = '\0';
    close(out_pipe[0]);
    close(err_pipe[0]);
    waitpid(child, &status, 0);
    if (WIFEXITED(status)) {
        *exit_code = WEXITSTATUS(status);
    } else {
        *exit_code = -1;
    }
    return 1;
}

static void print_eprint_case(void) {
    aleph_b_print(strv("out"));
    aleph_b_eprint(strv("err"));
    aleph_b_print(strv("out2"));
}

static void test_effects(void) {
    char out[256];
    char err[256];
    int code;

    if (run_capture(print_eprint_case, out, sizeof(out), err, sizeof(err),
                    &code)) {
        check(code == 0, "print/eprint exit");
        check_str(aleph_str(out), "out\nout2\n", "print stdout order");
        check_str(aleph_str(err), "err\n", "eprint stderr");
    }
}

static AlValue main_unit(AlValue *args, size_t count) {
    (void)args;
    (void)count;
    return aleph_unit();
}

static AlValue main_ok(AlValue *args, size_t count) {
    (void)args;
    (void)count;
    return aleph_ok(aleph_unit());
}

static AlValue main_err_str(AlValue *args, size_t count) {
    (void)args;
    (void)count;
    return aleph_err(strv("bad"));
}

static AlValue main_err_value(AlValue *args, size_t count) {
    (void)args;
    (void)count;
    return aleph_err(i64v(7));
}

static void main_case(AlValue (*code)(AlValue *, size_t)) {
    int status = aleph_main_run(code, 0, NULL);
    _exit(status);
}

static void main_case_unit(void);
static void main_case_ok(void);
static void main_case_err_str(void);
static void main_case_err_value(void);

static void test_main_results(void) {
    char out[256];
    char err[256];
    int code;

    struct Case {
        void (*callback)(void);
        int expected;
        const char *stderr_text;
    } cases[4];
    size_t index;

    cases[0].callback = main_case_unit;
    cases[0].expected = 0;
    cases[0].stderr_text = "";
    cases[1].callback = main_case_ok;
    cases[1].expected = 0;
    cases[1].stderr_text = "";
    cases[2].callback = main_case_err_str;
    cases[2].expected = 1;
    cases[2].stderr_text = "bad\n";
    cases[3].callback = main_case_err_value;
    cases[3].expected = 1;
    cases[3].stderr_text = "7\n";

    for (index = 0; index < 4; index += 1) {
        if (run_capture(cases[index].callback, out, sizeof(out), err,
                        sizeof(err), &code)) {
            check(code == cases[index].expected, "main result exit");
            check_str(aleph_str(err), cases[index].stderr_text,
                      "main result stderr");
            check(out[0] == '\0', "main result stdout");
        }
    }
}

static void main_case_unit(void) { main_case(main_unit); }
static void main_case_ok(void) { main_case(main_ok); }
static void main_case_err_str(void) { main_case(main_err_str); }
static void main_case_err_value(void) { main_case(main_err_value); }

/* ------------------------------------------------------------------ */
/* Traps                                                               */
/* ------------------------------------------------------------------ */

static const AlLoc trap_location = {7, 3, 5};

static char g_symlink_dir[512];
static char g_symlink_path[512];

static void enter_location(void) { aleph_eval_enter(&trap_location); }

static void trap_add(void) {
    enter_location();
    (void)aleph_add_i64(i64v(INT64_MAX), i64v(1));
}

static void trap_sub(void) {
    enter_location();
    (void)aleph_sub_i64(i64v(INT64_MIN), i64v(1));
}

static void trap_mul(void) {
    enter_location();
    (void)aleph_mul_i64(i64v(INT64_MAX), i64v(2));
}

static void trap_neg(void) {
    enter_location();
    (void)aleph_neg_i64(i64v(INT64_MIN));
}

static void trap_div_zero(void) {
    enter_location();
    (void)aleph_div_i64(i64v(1), i64v(0));
}

static void trap_mod_zero(void) {
    enter_location();
    (void)aleph_mod_i64(i64v(1), i64v(0));
}

static void trap_div_min(void) {
    enter_location();
    (void)aleph_div_i64(i64v(INT64_MIN), i64v(-1));
}

static void trap_mod_min(void) {
    enter_location();
    (void)aleph_mod_i64(i64v(INT64_MIN), i64v(-1));
}

static void trap_sum(void) {
    AlValue items[2];
    enter_location();
    items[0] = i64v(INT64_MAX);
    items[1] = i64v(1);
    (void)aleph_b_sum(aleph_list(items, 2));
}

static void trap_range(void) {
    enter_location();
    (void)aleph_b_range(i64v(0), i64v(INT64_MAX));
}

static void trap_read_no_uses(void) {
    aleph_set_uses(NULL, 0);
    enter_location();
    (void)aleph_b_read(strv("/etc/passwd"));
}

static void trap_write_outside(void) {
    static const AlUse uses[] = {{AL_CAP_FS_WRITE, "/tmp/aleph-tlon-prefix"}};
    aleph_set_uses(uses, 1);
    enter_location();
    (void)aleph_b_write(strv("/etc/aleph-tlon-outside"), strv("x"));
}

static void trap_nul_outside(void) {
    static const AlUse uses[] = {{AL_CAP_FS_READ, "/tmp/aleph-tlon-prefix"}};
    static const char path[] = "/etc/pass\0wd";
    aleph_set_uses(uses, 1);
    enter_location();
    (void)aleph_b_read(aleph_str_n(path, sizeof(path) - 1));
}

static void set_symlink_uses(void) {
    static AlUse uses[2];
    uses[0].cap = AL_CAP_FS_READ;
    uses[0].path = g_symlink_dir;
    uses[1].cap = AL_CAP_FS_WRITE;
    uses[1].path = g_symlink_dir;
    aleph_set_uses(uses, 2);
}

static void trap_read_symlink_outside(void) {
    set_symlink_uses();
    enter_location();
    (void)aleph_b_read(aleph_str(g_symlink_path));
}

static void trap_write_symlink_outside(void) {
    set_symlink_uses();
    enter_location();
    (void)aleph_b_write(aleph_str(g_symlink_path), strv("x"));
}

static void trap_eval_depth(void) {
    size_t index;
    for (index = 0; index < AL_MAX_EVAL_DEPTH + 1; index += 1) {
        aleph_eval_enter(&trap_location);
    }
}

static void trap_call_depth(void) {
    size_t index;
    for (index = 0; index < AL_MAX_CALL_DEPTH + 1; index += 1) {
        aleph_call_enter(&trap_location);
    }
}

static void trap_print_then_divide(void) {
    enter_location();
    aleph_b_print(strv("out"));
    (void)aleph_div_i64(i64v(1), i64v(0));
}

static void trap_special_file(void) {
    aleph_set_file("a\"b\\c.aph");
    enter_location();
    (void)aleph_div_i64(i64v(1), i64v(0));
}

static void expect_trap(const char *name, void (*callback)(void),
                        const char *expected_stderr,
                        const char *expected_stdout) {
    char out[512];
    char err[1024];
    int code;

    if (!run_capture(callback, out, sizeof(out), err, sizeof(err), &code)) {
        return;
    }
    if (code != 70) {
        char detail[128];
        snprintf(detail, sizeof(detail), "%s: exit %d, expected 70", name,
                 code);
        fail("trap exit", detail);
    }
    if (strcmp(err, expected_stderr) != 0) {
        char detail[1400];
        snprintf(detail, sizeof(detail), "%s:\nexpected: %s\n     got: %s",
                 name, expected_stderr, err);
        fail("trap stderr", detail);
    }
    if (expected_stdout != NULL && strcmp(out, expected_stdout) != 0) {
        char detail[600];
        snprintf(detail, sizeof(detail), "%s: stdout expected \"%s\" got \"%s\"",
                 name, expected_stdout, out);
        fail("trap stdout", detail);
    }
}

static void test_traps(void) {
    static const char *normal =
        "{\"code\":\"%s\",\"severity\":\"error\",\"file\":\"trap.aph\","
        "\"line\":3,\"col\":5,\"node\":\"n7\",\"msg\":\"%s\"}\n";
    char expected[512];

    aleph_set_file("trap.aph");
    aleph_set_uses(NULL, 0);

    snprintf(expected, sizeof(expected), normal, "R001", "integer overflow");
    expect_trap("add overflow", trap_add, expected, NULL);
    expect_trap("sub overflow", trap_sub, expected, NULL);
    expect_trap("mul overflow", trap_mul, expected, NULL);
    expect_trap("neg overflow", trap_neg, expected, NULL);
    expect_trap("sum overflow", trap_sum, expected, NULL);
    expect_trap("div min", trap_div_min, expected, NULL);
    expect_trap("mod min", trap_mod_min, expected, NULL);

    snprintf(expected, sizeof(expected), normal, "R002", "division by zero");
    expect_trap("div zero", trap_div_zero, expected, NULL);
    expect_trap("mod zero", trap_mod_zero, expected, NULL);
    expect_trap("print then trap", trap_print_then_divide, expected, "out\n");

    snprintf(expected, sizeof(expected), normal, "R003",
             "capability violation");
    expect_trap("read without uses", trap_read_no_uses, expected, NULL);
    expect_trap("write outside prefix", trap_write_outside, expected, NULL);
    expect_trap("NUL path outside prefix", trap_nul_outside, expected, NULL);
    {
        char directory[] = "/tmp/aleph-tlon-symlink-XXXXXX";
        char outside[512];
        char *made = mkdtemp(directory);
        FILE *raw;
        if (made == NULL) {
            fail("mkdtemp symlink", strerror(errno));
        } else {
            snprintf(outside, sizeof(outside), "/tmp/aleph-tlon-outside-%ld.txt",
                     (long)getpid());
            raw = fopen(outside, "wb");
            if (raw == NULL) {
                fail("fopen symlink target", strerror(errno));
            } else {
                fputs("outside", raw);
                fclose(raw);
                snprintf(g_symlink_dir, sizeof(g_symlink_dir), "%s", made);
                snprintf(g_symlink_path, sizeof(g_symlink_path), "%s/link.txt",
                         made);
                if (symlink(outside, g_symlink_path) != 0) {
                    fail("symlink inside prefix", strerror(errno));
                } else {
                    expect_trap("read symlink escape",
                                trap_read_symlink_outside, expected, NULL);
                    expect_trap("write symlink escape",
                                trap_write_symlink_outside, expected, NULL);
                    unlink(g_symlink_path);
                }
                unlink(outside);
            }
            rmdir(made);
        }
    }

    snprintf(expected, sizeof(expected), normal, "R004", "stack overflow");
    expect_trap("eval depth", trap_eval_depth, expected, NULL);
    expect_trap("call depth", trap_call_depth, expected, NULL);

    snprintf(expected, sizeof(expected), normal, "R005",
             "allocation limit exceeded");
    expect_trap("range limit", trap_range, expected, NULL);

    expect_trap(
        "json escaping", trap_special_file,
        "{\"code\":\"R002\",\"severity\":\"error\","
        "\"file\":\"a\\\"b\\\\c.aph\",\"line\":3,\"col\":5,\"node\":\"n7\","
        "\"msg\":\"division by zero\"}\n",
        NULL);

    aleph_set_file("trap.aph");
}

/* ------------------------------------------------------------------ */
/* Split mode                                                          */
/* ------------------------------------------------------------------ */

static int hex_digit(int byte) {
    if (byte >= '0' && byte <= '9') {
        return byte - '0';
    }
    if (byte >= 'a' && byte <= 'f') {
        return byte - 'a' + 10;
    }
    if (byte >= 'A' && byte <= 'F') {
        return byte - 'A' + 10;
    }
    return -1;
}

static bool decode_hex(const char *text, unsigned char *out, size_t cap,
                       size_t *len) {
    size_t index = 0;
    size_t used = 0;
    if (strcmp(text, "-") == 0) {
        *len = 0;
        return true;
    }
    while (text[index] != '\0') {
        int high = hex_digit((unsigned char)text[index]);
        int low = hex_digit((unsigned char)text[index + 1]);
        if (high < 0 || low < 0 || used >= cap) {
            return false;
        }
        out[used++] = (unsigned char)((high << 4) | low);
        index += 2;
    }
    *len = used;
    return true;
}

static int split_mode(void) {
    char line[8192];
    while (fgets(line, sizeof(line), stdin) != NULL) {
        char source_hex[2048];
        char separator_hex[2048];
        unsigned char source[1024];
        unsigned char separator[1024];
        size_t source_len;
        size_t separator_len;
        AlValue parts;
        size_t index;
        if (sscanf(line, "%2047s %2047s", source_hex, separator_hex) != 2) {
            continue;
        }
        if (!decode_hex(source_hex, source, sizeof(source), &source_len) ||
            !decode_hex(separator_hex, separator, sizeof(separator),
                        &separator_len)) {
            continue;
        }
        parts = aleph_b_split(
            aleph_str_n((const char *)source, source_len),
            aleph_str_n((const char *)separator, separator_len));
        printf("%zu\n", parts.u.list->len);
        for (index = 0; index < parts.u.list->len; index += 1) {
            const AlStr *part = parts.u.list->items[index].u.str;
            size_t byte;
            for (byte = 0; byte < part->len; byte += 1) {
                printf("%02x", (unsigned int)(unsigned char)part->data[byte]);
            }
            putchar('\n');
        }
    }
    fflush(stdout);
    return 0;
}

/* ------------------------------------------------------------------ */
/* f64 formatting mode                                                 */
/* ------------------------------------------------------------------ */

static int format_mode(void) {
    char line[128];
    while (fgets(line, sizeof(line), stdin) != NULL) {
        unsigned long long bits;
        double number;
        AlValue shown;
        if (sscanf(line, "%llx", &bits) != 1) {
            continue;
        }
        memcpy(&number, &bits, sizeof(number));
        shown = aleph_show(aleph_f64(number));
        if (shown.u.str->len > 0) {
            fwrite(shown.u.str->data, 1, shown.u.str->len, stdout);
        }
        fputc('\n', stdout);
    }
    fflush(stdout);
    return 0;
}

int main(int argc, char **argv) {
    if (argc == 2 && strcmp(argv[1], "--format") == 0) {
        return format_mode();
    }
    if (argc == 2 && strcmp(argv[1], "--split") == 0) {
        return split_mode();
    }

    test_show();
    test_builtins();
    test_dict();
    test_json();
    test_higher_order();
    test_filesystem();
    test_effects();
    test_main_results();
    test_traps();

    if (failures != 0) {
        fprintf(stderr, "%d failure(s)\n", failures);
        return 1;
    }
    puts("ok");
    return 0;
}
