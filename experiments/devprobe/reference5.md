### avgstr
```aleph
fn avgstr s:str -> res f64 str
 ex avgstr "1 2" = ok 1.5
 let ns try mapr lam t okor int t "bad" fields s
 if == len ns 0 err "bad"
 ok / tof sum ns tof len ns
```

### dedupe
```aleph
fn dedupe xs:list i64 -> list i64
 ex dedupe [3 1 3 2 1] = [3 1 2]
 fold lam a x if has index a x a push a x empty i64 xs
```

### rle
```aleph
fn rle s:str -> str
 ex rle "aaab" = "a3b1"
 let cs split s ""
 let bs filter lam i or == i 0 != else at cs i "" else at cs - i 1 "" range 0 len cs
 join map2 lam b e cat else at cs b "" show - e b bs push slice bs 1 len bs len cs ""
```

### dot
```aleph
fn dot a:list i64 b:list i64 -> i64
 ex dot [1 2 3] [4 5 6] = 32
 sum map2 lam x y * x y a b
```

### cheapest
```aleph
type item name:str price:i64
fn cheapest xs:list item -> opt str
 ex cheapest [item "a" 5 item "b" 3] = some "b"
 let s sortby lam i .price i xs
 if == len s 0 none str
 some .name else at s 0 item "" 0
```

### bylen
```aleph
fn bylen s:str -> dict i64 list str
 ex bylen "a bb c" = put put dict i64 list str 1 ["a" "c"] 2 ["bb"]
 fold lam d w put d count w push else get d count w empty str w dict i64 list str fields s
```

### parseip
```aleph
fn parseip s:str -> res list i64 str
 ex parseip "10.0.0.1" = ok [10 0 0 1]
 let ns try mapr lam t okor int t "bad" split s "."
 if or != len ns 4 != len filter lam n and >= n 0 <= n 255 ns 4 err "bad"
 ok ns
```

### minmax
```aleph
fn minmax xs:list i64 -> opt list i64
 ex minmax [3 -1 2] = some [-1 3]
 let s sort xs
 if == len s 0 none list i64
 some [else at s 0 0 else at s - len s 1 0]
```

### digitsum
```aleph
fn digitsum n:i64 -> i64
 ex digitsum 1234 = 10
 if < n 10 n + % n 10 digitsum / n 10
```

### caesar
```aleph
fn caes1 c:str k:i64 -> str
 ex caes1 "z" 1 = "a"
 let o else ord c 0
 if or < o 97 > o 122 c else chr + 97 % + - o 97 k 26 c
fn caesar s:str k:i64 -> str
 ex caesar "abz" 1 = "bca"
 join map lam c caes1 c k split s "" ""
```

### occurs
```aleph
fn occurs s:str w:str -> i64
 ex occurs "a b a" "a" = 2
 len filter lam x == x w fields s
```

### hhmm
```aleph
fn hhmm s:str -> res i64 str
 ex hhmm "01:30" = ok 90
 let p split s ":"
 if != len p 2 err "bad"
 let h try okor int else at p 0 "" "bad"
 let m try okor int else at p 1 "" "bad"
 if or or < h 0 > h 23 or < m 0 > m 59 err "bad"
 ok + * h 60 m
```

### runops
```aleph
type op | inc | dbl | set v:i64
fn step a:i64 o:op -> i64
 ex step 2 dbl = 4
 match o inc + a 1 dbl * a 2 set v v
fn runops ops:list op -> i64
 ex runops [inc inc dbl] = 4
 fold @step 0 ops
```

### inorder
```aleph
type tree | tip | node l:tree v:i64 r:tree
fn inorder t:tree -> list i64
 ex inorder node node tip 1 tip 2 node tip 3 tip = [1 2 3]
 match t tip empty i64 node l v r cat push inorder l v inorder r
```

### diag
```aleph
fn diag m:list list i64 -> list i64
 ex diag [[1 2] [3 4]] = [1 4]
 map lam i else at else at m i empty i64 i 0 range 0 len m
```

### prefixes
```aleph
fn prefixes s:str -> list str
 ex prefixes "abc" = ["a" "ab" "abc"]
 let cs split s ""
 map lam i join slice cs 0 i "" range 1 + len cs 1
```

### firstdup
```aleph
fn firstdup xs:list i64 -> opt i64
 ex firstdup [1 2 3 2 1] = some 2
 let is filter lam i != else index xs else at xs i 0 i i range 0 len xs
 if == len is 0 none i64 at xs else at is 0 0
```

### lookall
```aleph
fn lookall d:dict str i64 ks:list str -> res list i64 str
 ex lookall dict str i64 ["b"] = err "b"
 mapr lam k okor get d k k ks
```

### anagram
```aleph
fn anagram a:str b:str -> bool
 ex anagram "listen" "silent" = true
 == show sort split a "" show sort split b ""
```

### sumcol
```aleph
fn sumcol s:str c:i64 -> res i64 str
 ex sumcol "1,2\n3,4" 1 = ok 6
 if == s "" ok 0
 let ns try mapr lam l okor int else at split l "," c "" "bad" split s "\n"
 ok sum ns
```
