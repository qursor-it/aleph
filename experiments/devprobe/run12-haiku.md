### avgstr

```aleph
fn avgstr s:str -> res f64 str
 ex avgstr "1 2" = ok 1.5
 ex isok avgstr "1 x" = false
 let ns try mapr lam t okor int t "bad" fields s
 if == len ns 0 err "bad"
 ok / tof sum ns tof len ns
```

### dedupe

```aleph
fn dedupe xs:list i64 -> list i64
 ex dedupe [3 1 3 2 1] = [3 1 2]
 ex dedupe empty i64 = empty i64
 fold lam a x if has index a x a push a x empty i64 xs
```

### rle

```aleph
fn lead cs:list str c:str -> i64
 ex lead split "aab" "" "a" = 2
 ex lead empty str "a" = 0
 if == len cs 0 0
 if != else at cs 0 "" c 0
 + 1 lead slice cs 1 len cs c
fn rl cs:list str -> str
 ex rl split "aab" "" = "a2b1"
 ex rl empty str = ""
 if == len cs 0 ""
 let c else at cs 0 ""
 let n lead cs c
 cat cat c show n rl slice cs n len cs
fn rle s:str -> str
 ex rle "aaab" = "a3b1"
 ex rle "" = ""
 rl split s ""
```

### dot

```aleph
fn dot a:list i64 b:list i64 -> i64
 ex dot [1 2 3] [4 5 6] = 32
 ex dot empty i64 empty i64 = 0
 sum map2 lam x y * x y a b
```

### cheapest

```aleph
type item name:str price:i64
fn cheapest xs:list item -> opt str
 ex cheapest [item "a" 5 item "b" 3] = some "b"
 ex cheapest empty item = none str
 at map lam i .name i sortby lam i .price i xs 0
```

### bylen

```aleph
fn bylen s:str -> dict i64 list str
 ex bylen "a bb c" = put put dict i64 list str 1 ["a" "c"] 2 ["bb"]
 ex bylen "" = dict i64 list str
 fold lam a x put a count x push else get a count x empty str x dict i64 list str fields s
```

### parseip

```aleph
fn parseip s:str -> res list i64 str
 ex parseip "10.0.0.1" = ok [10 0 0 1]
 ex isok parseip "1.2.3" = false
 let ps split s "."
 if != len ps 4 err "bad"
 let ns try mapr lam t okor int t "bad" ps
 if > len filter lam x or < x 0 > x 255 ns 0 err "bad"
 ok ns
```

### minmax

```aleph
fn minmax xs:list i64 -> opt list i64
 ex minmax [3 -1 2] = some [-1 3]
 ex minmax empty i64 = none list i64
 if == len xs 0 none list i64
 let f else at xs 0 0
 some [fold lam a x min a x f xs fold lam a x max a x f xs]
```

### digitsum

```aleph
fn digitsum n:i64 -> i64
 ex digitsum 1234 = 10
 ex digitsum 0 = 0
 if < n 10 n
 + % n 10 digitsum / n 10
```

### caesar

```aleph
fn shiftc c:str k:i64 -> str
 ex shiftc "a" 1 = "b"
 ex shiftc "z" 1 = "a"
 ex shiftc "-" 1 = "-"
 let o else ord c 0
 if and >= o 97 <= o 122 else chr + 97 % + - o 97 k 26 c
 c
fn caesar s:str k:i64 -> str
 ex caesar "abz" 1 = "bca"
 ex caesar "" 3 = ""
 join map lam c shiftc c k split s "" ""
```

### occurs

```aleph
fn occurs s:str w:str -> i64
 ex occurs "a b a" "a" = 2
 ex occurs "" "a" = 0
 len filter lam x == x w fields s
```

### hhmm

```aleph
fn hhmm s:str -> res i64 str
 ex hhmm "01:30" = ok 90
 ex isok hhmm "24:00" = false
 let ps split s ":"
 if or != len ps 2 != len filter lam p == count p 2 ps 2 err "bad"
 let h try okor int else at ps 0 "" "bad"
 let m try okor int else at ps 1 "" "bad"
 if or or < h 0 > h 23 or < m 0 > m 59 err "bad"
 ok + * h 60 m
```

### runops

```aleph
type op | inc | dbl | set v:i64
fn runops ops:list op -> i64
 ex runops [inc inc dbl] = 4
 ex runops empty op = 0
 fold lam a o match o inc + a 1 dbl * a 2 set v v 0 ops
```

### inorder

```aleph
type tree | tip | node l:tree v:i64 r:tree
fn inorder t:tree -> list i64
 ex inorder node node tip 1 tip 2 node tip 3 tip = [1 2 3]
 ex inorder tip = empty i64
 match t tip empty i64 node l v r cat cat inorder l [v] inorder r
```

### diag

```aleph
fn diag m:list list i64 -> list i64
 ex diag [[1 2] [3 4]] = [1 4]
 ex diag empty list i64 = empty i64
 map lam i else at else at m i empty i64 i 0 range 0 len m
```

### prefixes

```aleph
fn prefixes s:str -> list str
 ex prefixes "abc" = ["a" "ab" "abc"]
 ex prefixes "" = empty str
 let cs split s ""
 map lam i join slice cs 0 i "" range 1 + len cs 1
```

### firstdup

```aleph
fn firstdup xs:list i64 -> opt i64
 ex firstdup [1 2 3 2 1] = some 2
 ex firstdup [1 2] = none i64
 at map lam j else at xs j 0 filter lam i has index slice xs 0 i else at xs i 0 range 0 len xs 0
```

### lookall

```aleph
fn lookall d:dict str i64 ks:list str -> res list i64 str
 ex lookall put dict str i64 "a" 1 ["a" "a"] = ok [1 1]
 ex lookall dict str i64 ["b"] = err "b"
 mapr lam k okor get d k k ks
```

### anagram

```aleph
fn anagram a:str b:str -> bool
 ex anagram "listen" "silent" = true
 ex anagram "ab" "abb" = false
 == show sort split a "" show sort split b ""
```

### sumcol

```aleph
fn sumcol s:str c:i64 -> res i64 str
 ex sumcol "1,2\n3,4" 1 = ok 6
 ex isok sumcol "1,x" 1 = false
 if == s "" ok 0
 let vs try mapr lam r okor int else at split r "," c "" "bad" split s "\n"
 ok sum vs
```
