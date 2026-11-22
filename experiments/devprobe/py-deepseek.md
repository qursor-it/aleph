### rpn

```python
def rpn(s):
    stack = []
    for token in s.split():
        if token in ("+", "-", "*"):
            if len(stack) < 2:
                return ("err", "bad")
            b = stack.pop()
            a = stack.pop()
            if token == "+":
                stack.append(a + b)
            elif token == "-":
                stack.append(a - b)
            else:
                stack.append(a * b)
        else:
            if not token or (token[0] in "+-" and len(token) > 1):
                body = token[1:] if token[0] in "+-" else token
            else:
                body = token
            if not body.isdigit():
                return ("err", "bad")
            stack.append(int(token))
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
    out.extend(a[i:])
    out.extend(b[j:])
    return out


assert merge([1, 4], [2, 3, 5]) == [1, 2, 3, 4, 5]
assert merge([], []) == []
```

### runs

```python
def runs(xs):
    out = []
    for x in xs:
        if out and out[-1][0] == x:
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
        (1000, "M"), (900, "CM"), (500, "D"), (400, "CD"),
        (100, "C"), (90, "XC"), (50, "L"), (40, "XL"),
        (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I"),
    ]
    out = []
    for value, symbol in table:
        while n >= value:
            out.append(symbol)
            n -= value
    return "".join(out)


assert toroman(1994) == "MCMXCIV"
assert toroman(3) == "III"
```

### fromroman

```python
def fromroman(s):
    values = {"I": 1, "V": 5, "X": 10, "L": 50,
              "C": 100, "D": 500, "M": 1000}
    total = 0
    for i, ch in enumerate(s):
        if i + 1 < len(s) and values[ch] < values[s[i + 1]]:
            total -= values[ch]
        else:
            total += values[ch]
    return total


assert fromroman("MCMXCIV") == 1994
assert fromroman("III") == 3
```

### evalexpr

```python
from dataclasses import dataclass


@dataclass
class Num:
    v: int


@dataclass
class Add:
    a: object
    b: object


@dataclass
class Mul:
    a: object
    b: object


def evalexpr(e):
    if isinstance(e, Num):
        return e.v
    if isinstance(e, Add):
        return evalexpr(e.a) + evalexpr(e.b)
    return evalexpr(e.a) * evalexpr(e.b)


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
    if not m:
        return []
    return [[row[j] for row in m] for j in range(len(m[0]))]


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
        elif ch in ")]}":
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
    current = ""
    for word in s.split():
        if len(word) > w:
            if current:
                lines.append(current)
                current = ""
            lines.append(word)
        elif not current:
            current = word
        elif len(current) + 1 + len(word) <= w:
            current += " " + word
        else:
            lines.append(current)
            current = word
    if current:
        lines.append(current)
    return lines


assert wrap("a bb ccc d", 4) == ["a bb", "ccc", "d"]
assert wrap("", 3) == []
```

### bank

```python
def bank(cs):
    balance = 0
    for command in cs:
        parts = command.split()
        if len(parts) != 2 or parts[0] not in ("dep", "wd"):
            return ("err", "bad")
        try:
            amount = int(parts[1])
        except ValueError:
            return ("err", "bad")
        if amount < 0:
            return ("err", "bad")
        balance += amount if parts[0] == "dep" else -amount
        if balance < 0:
            return ("err", "overdraft")
    return ("ok", balance)


assert bank(["dep 10", "wd 3"]) == ("ok", 7)
assert bank(["wd 1"]) == ("err", "overdraft")
```

### jsonsum

```python
import json


def jsonsum(s):
    try:
        value = json.loads(s)
    except ValueError:
        return ("err", "bad")
    if not isinstance(value, list):
        return ("err", "bad")
    total = 0
    for x in value:
        if isinstance(x, bool) or not isinstance(x, int):
            return ("err", "bad")
        total += x
    return ("ok", total)


assert jsonsum("[1, 2, 3]") == ("ok", 6)
assert jsonsum("[1, \"a\"]")[0] == "err"
```

### median

```python
def median(xs):
    if not xs:
        return None
    ordered = sorted(xs)
    n = len(ordered)
    if n % 2:
        return float(ordered[n // 2])
    return (ordered[n // 2 - 1] + ordered[n // 2]) / 2


assert median([3, 1, 2]) == 2.0
assert median([]) is None
```

### primes

```python
def primes(n):
    if n < 2:
        return []
    sieve = [True] * (n + 1)
    sieve[0] = sieve[1] = False
    i = 2
    while i * i <= n:
        if sieve[i]:
            for j in range(i * i, n + 1, i):
                sieve[j] = False
        i += 1
    return [x for x in range(2, n + 1) if sieve[x]]


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
import math


def gcdlist(xs):
    g = 0
    for x in xs:
        g = math.gcd(g, x)
    return g


assert gcdlist([12, 18, 30]) == 6
assert gcdlist([]) == 0
```

### kvdict

```python
def kvdict(s):
    result = {}
    if s == "":
        return ("ok", result)
    for pair in s.split(","):
        parts = pair.split("=")
        if len(parts) != 2 or not parts[0]:
            return ("err", "bad")
        try:
            result[parts[0]] = int(parts[1])
        except ValueError:
            return ("err", "bad")
    return ("ok", result)


assert kvdict("a=1,b=2") == ("ok", {"a": 1, "b": 2})
assert kvdict("a")[0] == "err"
```

### leaves

```python
from dataclasses import dataclass


@dataclass
class Leaf:
    v: int


@dataclass
class Node:
    l: object
    r: object


def leaves(t):
    if isinstance(t, Leaf):
        return [t.v]
    return leaves(t.l) + leaves(t.r)


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
        for neighbour in g.get(node, []):
            if neighbour not in seen:
                seen.add(neighbour)
                stack.append(neighbour)
    return len(seen)


assert reachable({"a": ["b"], "b": ["a", "c"]}, "a") == 3
assert reachable({}, "x") == 1
```

### secondmax

```python
def secondmax(xs):
    unique = sorted(set(xs), reverse=True)
    return unique[1] if len(unique) >= 2 else None


assert secondmax([3, 1, 3, 2]) == 2
assert secondmax([5, 5]) is None
```

### avgstr

```python
def avgstr(s):
    tokens = s.split()
    if not tokens:
        return ("err", "bad")
    values = []
    for token in tokens:
        try:
            values.append(int(token))
        except ValueError:
            return ("err", "bad")
    return ("ok", sum(values) / len(values))


assert avgstr("1 2") == ("ok", 1.5)
assert avgstr("1 x")[0] == "err"
```

### dedupe

```python
def dedupe(xs):
    seen = set()
    out = []
    for x in xs:
        if x not in seen:
            seen.add(x)
            out.append(x)
    return out


assert dedupe([3, 1, 3, 2, 1]) == [3, 1, 2]
assert dedupe([]) == []
```

### rle

```python
def rle(s):
    out = []
    i = 0
    while i < len(s):
        j = i
        while j < len(s) and s[j] == s[i]:
            j += 1
        out.append(s[i] + str(j - i))
        i = j
    return "".join(out)


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
    best = xs[0]
    for x in xs[1:]:
        if x.price < best.price:
            best = x
    return best.name


assert cheapest([Item("a", 5), Item("b", 3)]) == "b"
assert cheapest([]) is None
```

### bylen

```python
def bylen(s):
    result = {}
    for word in s.split():
        result.setdefault(len(word), []).append(word)
    return result


assert bylen("a bb c") == {1: ["a", "c"], 2: ["bb"]}
assert bylen("") == {}
```

### parseip

```python
def parseip(s):
    parts = s.split(".")
    if len(parts) != 4:
        return ("err", "bad")
    numbers = []
    for part in parts:
        if not part or not part.isdigit():
            return ("err", "bad")
        value = int(part)
        if value > 255:
            return ("err", "bad")
        numbers.append(value)
    return ("ok", numbers)


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
    return sum(int(ch) for ch in str(n))


assert digitsum(1234) == 10
assert digitsum(0) == 0
```

### caesar

```python
def caesar(s, k):
    return "".join(
        chr((ord(ch) - 97 + k) % 26 + 97) if "a" <= ch <= "z" else ch
        for ch in s
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
def hhmm(s):
    parts = s.split(":")
    if len(parts) != 2 or not all(
        len(p) == 2 and p.isdigit() for p in parts
    ):
        return ("err", "bad")
    hours = int(parts[0])
    minutes = int(parts[1])
    if hours > 23 or minutes > 59:
        return ("err", "bad")
    return ("ok", hours * 60 + minutes)


assert hhmm("01:30") == ("ok", 90)
assert hhmm("24:00")[0] == "err"
```

### runops

```python
from dataclasses import dataclass


@dataclass
class Inc:
    pass


@dataclass
class Dbl:
    pass


@dataclass
class Set:
    v: int


def runops(ops):
    value = 0
    for op in ops:
        if isinstance(op, Inc):
            value += 1
        elif isinstance(op, Dbl):
            value *= 2
        else:
            value = op.v
    return value


assert runops([Inc(), Inc(), Dbl()]) == 4
assert runops([]) == 0
```

### inorder

```python
from dataclasses import dataclass


@dataclass
class Tip:
    pass


@dataclass
class Node:
    l: object
    v: int
    r: object


def inorder(t):
    if isinstance(t, Tip):
        return []
    return inorder(t.l) + [t.v] + inorder(t.r)


assert inorder(Node(Node(Tip(), 1, Tip()), 2, Node(Tip(), 3, Tip()))) == [1, 2, 3]
assert inorder(Tip()) == []
```

### diag

```python
def diag(m):
    return [m[i][i] for i in range(len(m))]


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
    for key in ks:
        if key not in d:
            return ("err", key)
        out.append(d[key])
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
        if c < 0 or c >= len(fields):
            return ("err", "bad")
        try:
            total += int(fields[c])
        except ValueError:
            return ("err", "bad")
    return ("ok", total)


assert sumcol("1,2\n3,4", 1) == ("ok", 6)
assert sumcol("1,x", 1)[0] == "err"
```
