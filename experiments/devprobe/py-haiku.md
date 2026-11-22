### rpn

```python
def rpn(s):
    stack = []
    for tok in s.split():
        if tok in ("+", "-", "*"):
            if len(stack) < 2:
                return ("err", "bad")
            b = stack.pop()
            a = stack.pop()
            stack.append(a + b if tok == "+" else a - b if tok == "-" else a * b)
        else:
            try:
                stack.append(int(tok))
            except ValueError:
                return ("err", "bad")
    if len(stack) != 1:
        return ("err", "bad")
    return ("ok", stack[0])


assert rpn("3 4 + 2 *") == ("ok", 14)
assert rpn("1 +")[0] == "err"
```

### merge

```python
def merge(a, b):
    out = []
    i = j = 0
    while i < len(a) and j < len(b):
        if a[i] <= b[j]:
            out.append(a[i])
            i += 1
        else:
            out.append(b[j])
            j += 1
    return out + a[i:] + b[j:]


assert merge([1, 4], [2, 3, 5]) == [1, 2, 3, 4, 5]
assert merge([], []) == []
```

### runs

```python
def runs(xs):
    out = []
    for x in xs:
        if out and out[-1][-1] == x:
            out[-1].append(x)
        else:
            out.append([x])
    return out


assert runs([1, 1, 2, 1]) == [[1, 1], [2], [1]]
assert runs([]) == []
```

### toroman

```python
def toroman(n):
    table = [
        (1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"),
        (50, "L"), (40, "XL"), (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I"),
    ]
    out = ""
    for value, sym in table:
        while n >= value:
            out += sym
            n -= value
    return out


assert toroman(1994) == "MCMXCIV"
assert toroman(3) == "III"
```

### fromroman

```python
def fromroman(s):
    vals = {"I": 1, "V": 5, "X": 10, "L": 50, "C": 100, "D": 500, "M": 1000}
    total = 0
    for i, ch in enumerate(s):
        v = vals[ch]
        if i + 1 < len(s) and v < vals[s[i + 1]]:
            total -= v
        else:
            total += v
    return total


assert fromroman("MCMXCIV") == 1994
assert fromroman("III") == 3
```

### evalexpr

```python
from dataclasses import dataclass
from typing import Union


@dataclass
class Num:
    v: int


@dataclass
class Add:
    a: "Expr"
    b: "Expr"


@dataclass
class Mul:
    a: "Expr"
    b: "Expr"


Expr = Union[Num, Add, Mul]


def evalexpr(e):
    match e:
        case Num(v):
            return v
        case Add(a, b):
            return evalexpr(a) + evalexpr(b)
        case Mul(a, b):
            return evalexpr(a) * evalexpr(b)


assert evalexpr(Add(Num(2), Mul(Num(3), Num(4)))) == 14
assert evalexpr(Num(5)) == 5
```

### topn

```python
from collections import Counter


def topn(s, k):
    counts = Counter(s.split())
    return sorted(counts, key=lambda w: (-counts[w], w))[:k]


assert topn("b a b c a b", 2) == ["b", "a"]
assert topn("", 3) == []
```

### transpose

```python
def transpose(m):
    return [list(col) for col in zip(*m)]


assert transpose([[1, 2, 3], [4, 5, 6]]) == [[1, 4], [2, 5], [3, 6]]
assert transpose([]) == []
```

### brackets

```python
def brackets(s):
    pairs = {")": "(", "]": "[", "}": "{"}
    stack = []
    for ch in s:
        if ch in "([{":
            stack.append(ch)
        elif ch in pairs:
            if not stack or stack.pop() != pairs[ch]:
                return False
    return not stack


assert brackets("([]{x})") is True
assert brackets("(]") is False
```

### wrap

```python
def wrap(s, w):
    lines = []
    cur = ""
    for word in s.split():
        if not cur:
            cur = word
        elif len(cur) + 1 + len(word) <= w:
            cur += " " + word
        else:
            lines.append(cur)
            cur = word
    if cur:
        lines.append(cur)
    return lines


assert wrap("a bb ccc d", 4) == ["a bb", "ccc", "d"]
assert wrap("", 3) == []
```

### bank

```python
def bank(cs):
    balance = 0
    for c in cs:
        parts = c.split()
        if len(parts) != 2 or parts[0] not in ("dep", "wd") or not (parts[1].isascii() and parts[1].isdigit()):
            return ("err", "bad")
        n = int(parts[1])
        if parts[0] == "dep":
            balance += n
        elif n > balance:
            return ("err", "overdraft")
        else:
            balance -= n
    return ("ok", balance)


assert bank(["dep 10", "wd 3"]) == ("ok", 7)
assert bank(["wd 1"]) == ("err", "overdraft")
```

### jsonsum

```python
import json


def jsonsum(s):
    try:
        xs = json.loads(s)
    except ValueError:
        return ("err", "bad")
    if not isinstance(xs, list) or any(type(x) is not int for x in xs):
        return ("err", "bad")
    return ("ok", sum(xs))


assert jsonsum("[1, 2, 3]") == ("ok", 6)
assert jsonsum('[1, "a"]')[0] == "err"
```

### median

```python
def median(xs):
    if not xs:
        return None
    s = sorted(xs)
    n = len(s)
    mid = n // 2
    return float(s[mid]) if n % 2 else (s[mid - 1] + s[mid]) / 2


assert median([3, 1, 2]) == 2.0
assert median([]) is None
```

### primes

```python
import math


def primes(n):
    if n < 2:
        return []
    sieve = [True] * (n + 1)
    sieve[0] = sieve[1] = False
    for i in range(2, math.isqrt(n) + 1):
        if sieve[i]:
            sieve[i * i :: i] = [False] * len(range(i * i, n + 1, i))
    return [i for i, is_prime in enumerate(sieve) if is_prime]


assert primes(10) == [2, 3, 5, 7]
assert primes(1) == []
```

### titlecase

```python
def titlecase(s):
    return " ".join(w[:1].upper() + w[1:].lower() for w in s.split())


assert titlecase("hello  wORLD") == "Hello World"
assert titlecase("") == ""
```

### gcdlist

```python
from functools import reduce
from math import gcd


def gcdlist(xs):
    return reduce(gcd, xs, 0)


assert gcdlist([12, 18, 30]) == 6
assert gcdlist([]) == 0
```

### kvdict

```python
def kvdict(s):
    d = {}
    if s == "":
        return ("ok", d)
    for pair in s.split(","):
        k, sep, v = pair.partition("=")
        if not sep or not k:
            return ("err", "bad")
        try:
            d[k] = int(v)
        except ValueError:
            return ("err", "bad")
    return ("ok", d)


assert kvdict("a=1,b=2") == ("ok", {"a": 1, "b": 2})
assert kvdict("a")[0] == "err"
```

### leaves

```python
from dataclasses import dataclass
from typing import Union


@dataclass
class Leaf:
    v: int


@dataclass
class Node:
    l: "Tree"
    r: "Tree"


Tree = Union[Leaf, Node]


def leaves(t):
    match t:
        case Leaf(v):
            return [v]
        case Node(l, r):
            return leaves(l) + leaves(r)


assert leaves(Node(Leaf(1), Node(Leaf(2), Leaf(3)))) == [1, 2, 3]
assert leaves(Leaf(4)) == [4]
```

### reachable

```python
def reachable(g, s):
    seen = {s}
    stack = [s]
    while stack:
        node = stack.pop()
        for nxt in g.get(node, []):
            if nxt not in seen:
                seen.add(nxt)
                stack.append(nxt)
    return len(seen)


assert reachable({"a": ["b"], "b": ["a", "c"]}, "a") == 3
assert reachable({}, "x") == 1
```

### secondmax

```python
def secondmax(xs):
    distinct = sorted(set(xs), reverse=True)
    return distinct[1] if len(distinct) > 1 else None


assert secondmax([3, 1, 3, 2]) == 2
assert secondmax([5, 5]) is None
```

### avgstr

```python
def avgstr(s):
    toks = s.split()
    if not toks:
        return ("err", "bad")
    try:
        nums = [int(t) for t in toks]
    except ValueError:
        return ("err", "bad")
    return ("ok", sum(nums) / len(nums))


assert avgstr("1 2") == ("ok", 1.5)
assert avgstr("1 x")[0] == "err"
```

### dedupe

```python
def dedupe(xs):
    return list(dict.fromkeys(xs))


assert dedupe([3, 1, 3, 2, 1]) == [3, 1, 2]
assert dedupe([]) == []
```

### rle

```python
from itertools import groupby


def rle(s):
    return "".join(f"{c}{len(list(g))}" for c, g in groupby(s))


assert rle("aaab") == "a3b1"
assert rle("") == ""
```

### dot

```python
def dot(a, b):
    return sum(x * y for x, y in zip(a, b))


assert dot([1, 2, 3], [4, 5, 6]) == 32
assert dot([], []) == 0
```

### cheapest

```python
from dataclasses import dataclass


@dataclass
class Item:
    name: str
    price: int


def cheapest(xs):
    if not xs:
        return None
    return min(xs, key=lambda it: it.price).name


assert cheapest([Item("a", 5), Item("b", 3)]) == "b"
assert cheapest([]) is None
```

### bylen

```python
from collections import defaultdict


def bylen(s):
    groups = defaultdict(list)
    for w in s.split():
        groups[len(w)].append(w)
    return dict(groups)


assert bylen("a bb c") == {1: ["a", "c"], 2: ["bb"]}
assert bylen("") == {}
```

### parseip

```python
def parseip(s):
    parts = s.split(".")
    if len(parts) != 4 or not all(p.isascii() and p.isdigit() for p in parts):
        return ("err", "bad")
    nums = [int(p) for p in parts]
    if any(n > 255 for n in nums):
        return ("err", "bad")
    return ("ok", nums)


assert parseip("10.0.0.1") == ("ok", [10, 0, 0, 1])
assert parseip("1.2.3")[0] == "err"
```

### minmax

```python
def minmax(xs):
    if not xs:
        return None
    return [min(xs), max(xs)]


assert minmax([3, -1, 2]) == [-1, 3]
assert minmax([]) is None
```

### digitsum

```python
def digitsum(n):
    return sum(int(d) for d in str(n))


assert digitsum(1234) == 10
assert digitsum(0) == 0
```

### caesar

```python
def caesar(s, k):
    return "".join(
        chr((ord(c) - ord("a") + k) % 26 + ord("a")) if "a" <= c <= "z" else c
        for c in s
    )


assert caesar("abz", 1) == "bca"
assert caesar("", 3) == ""
```

### occurs

```python
def occurs(s, w):
    return s.split().count(w)


assert occurs("a b a", "a") == 2
assert occurs("", "a") == 0
```

### hhmm

```python
import re


def hhmm(s):
    if not re.fullmatch(r"[0-9]{2}:[0-9]{2}", s):
        return ("err", "bad")
    h, m = int(s[:2]), int(s[3:])
    if h > 23 or m > 59:
        return ("err", "bad")
    return ("ok", h * 60 + m)


assert hhmm("01:30") == ("ok", 90)
assert hhmm("24:00")[0] == "err"
```

### runops

```python
from dataclasses import dataclass
from typing import Union


@dataclass
class Inc:
    pass


@dataclass
class Dbl:
    pass


@dataclass
class SetV:
    v: int


Op = Union[Inc, Dbl, SetV]


def runops(ops):
    acc = 0
    for op in ops:
        match op:
            case Inc():
                acc += 1
            case Dbl():
                acc *= 2
            case SetV(v):
                acc = v
    return acc


assert runops([Inc(), Inc(), Dbl()]) == 4
assert runops([]) == 0
```

### inorder

```python
from dataclasses import dataclass
from typing import Union


@dataclass
class Tip:
    pass


@dataclass
class Node:
    l: "Tree"
    v: int
    r: "Tree"


Tree = Union[Tip, Node]


def inorder(t):
    match t:
        case Tip():
            return []
        case Node(l, v, r):
            return inorder(l) + [v] + inorder(r)


assert inorder(Node(Node(Tip(), 1, Tip()), 2, Node(Tip(), 3, Tip()))) == [1, 2, 3]
assert inorder(Tip()) == []
```

### diag

```python
def diag(m):
    return [row[i] for i, row in enumerate(m)]


assert diag([[1, 2], [3, 4]]) == [1, 4]
assert diag([]) == []
```

### prefixes

```python
def prefixes(s):
    return [s[:i] for i in range(1, len(s) + 1)]


assert prefixes("abc") == ["a", "ab", "abc"]
assert prefixes("") == []
```

### firstdup

```python
def firstdup(xs):
    seen = set()
    for x in xs:
        if x in seen:
            return x
        seen.add(x)
    return None


assert firstdup([1, 2, 3, 2, 1]) == 2
assert firstdup([1, 2]) is None
```

### lookall

```python
def lookall(d, ks):
    out = []
    for k in ks:
        if k not in d:
            return ("err", k)
        out.append(d[k])
    return ("ok", out)


assert lookall({"a": 1}, ["a", "a"]) == ("ok", [1, 1])
assert lookall({}, ["b"]) == ("err", "b")
```

### anagram

```python
from collections import Counter


def anagram(a, b):
    return Counter(a) == Counter(b)


assert anagram("listen", "silent") is True
assert anagram("ab", "abb") is False
```

### sumcol

```python
def sumcol(s, c):
    if s == "":
        return ("ok", 0)
    total = 0
    for line in s.split("\n"):
        fields = line.split(",")
        if c >= len(fields):
            return ("err", "bad")
        try:
            total += int(fields[c])
        except ValueError:
            return ("err", "bad")
    return ("ok", total)


assert sumcol("1,2\n3,4", 1) == ("ok", 6)
assert sumcol("1,x", 1)[0] == "err"
```
