### avgstr
```aleph
fn myint t:str -> res i64 str
 match int t ok v ok v err x err "bad"
fn avgstr s:str -> res f64 str
 ex avgstr "1 2" = ok 1.5
 ex isok avgstr "1 x" = false
 let ws fields s
 let ns try mapr @myint ws
 if == len ns 0 err "bad" ok / tof sum ns tof len ns
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
fn enc cs:list str out:str last:str cnt:i64 -> str
 match at cs 0 some c if == c last enc slice cs 1 len cs out last + cnt 1 enc slice cs 1 len cs cat out cat last show cnt c 1 none cat out cat last show cnt
fn rlego cs:list str -> str
 match at cs 0 some c enc slice cs 1 len cs "" c 1 none ""
fn rle s:str -> str
 ex rle "aaab" = "a3b1"
 ex rle "" = ""
 rlego split s ""
```

### dot
```aleph
fn dot a:list i64 b:list i64 -> i64
 ex dot [1 2 3] [4 5 6] = 32
 ex dot empty i64 empty i64 = 0
 fold lam acc i + acc * nth a i nth b i 0 range 0 len a
```

### cheapest
```aleph
type item name:str price:i64
fn pick a:opt item b:item -> opt item
 match a some x if < .price b .price x some b x none some b
fn cheapest xs:list item -> opt str
 ex cheapest [item "a" 5 item "b" 3] = some "b"
 ex cheapest empty item = none str
 match fold @pick none item xs some x some .name x none none str
```

### bylen
```aleph
fn bylen s:str -> dict i64 list str
 ex bylen "a bb c" = put put dict i64 list str 1 ["a" "c"] 2 ["bb"]
 ex bylen "" = dict i64 list str
 fold lam d w put d len w push else get d len w empty str w fields s
```

### parseip
```aleph
fn myint t:str -> res i64 str
 match int t ok v ok v err x err "bad"
fn valid ns:list i64 -> bool
 fold lam a n and a and <= 0 n <= n 255 true ns
fn p4 ps:list str -> res list i64 str
 let ns try mapr @myint ps
 if valid ns ok ns err "bad"
fn parseip s:str -> res list i64 str
 ex parseip "10.0.0.1" = ok [10 0 0 1]
 ex isok parseip "1.2.3" = false
 let ps split s "."
 if != len ps 4 err "bad" p4 ps
```

### minmax
```aleph
fn mn xs:list i64 -> i64
 fold lam a x min a x nth xs 0 xs
fn mx xs:list i64 -> i64
 fold lam a x max a x nth xs 0 xs
fn minmax xs:list i64 -> opt list i64
 ex minmax [3 -1 2] = some [-1 3]
 ex minmax empty i64 = none list i64
 if == len xs 0 none list i64 some [mn xs mx xs]
```

### digitsum
```aleph
fn digitsum n:i64 -> i64
 ex digitsum 1234 = 10
 ex digitsum 0 = 0
 if == n 0 0 + % n 10 digitsum / n 10
```

### caesar
```aleph
fn shift c:str k:i64 -> str
 match ord c some x if and <= 97 x <= x 122 else chr + 97 % + - x 97 k 26 c c none c
fn caesar s:str k:i64 -> str
 ex caesar "abz" 1 = "bca"
 ex caesar "" 3 = ""
 join map lam c shift c k split s "" ""
```

### occurs
```aleph
fn occurs s:str w:str -> i64
 ex occurs "a b a" "a" = 2
 ex occurs "" "a" = 0
 fold lam a x if == x w + a 1 a 0 fields s
```

### hhmm
```aleph
fn myint t:str -> res i64 str
 match int t ok v ok v err x err "bad"
fn two t:str -> res i64 str
 if != len t 2 err "bad" myint t
fn hp ps:list str -> res i64 str
 let h try two nth ps 0
 let m try two nth ps 1
 if and and <= 0 h <= h 23 and <= 0 m <= m 59 ok + * h 60 m err "bad"
fn hhmm s:str -> res i64 str
 ex hhmm "01:30" = ok 90
 ex isok hhmm "24:00" = false
 let ps split s ":"
 if != len ps 2 err "bad" hp ps
```

### runops
```aleph
type op | inc | dbl | set v:i64
fn apply a:i64 o:op -> i64
 match o inc + a 1 dbl * a 2 set n n
fn runops ops:list op -> i64
 ex runops [inc inc dbl] = 4
 ex runops empty op = 0
 fold @apply 0 ops
```

### inorder
```aleph
type tree | tip | node l:tree v:i64 r:tree
fn inorder t:tree -> list i64
 ex inorder node node tip 1 tip 2 node tip 3 tip = [1 2 3]
 ex inorder tip = empty i64
 match t tip empty i64 node l v r cat inorder l cat [v] inorder r
```

### diag
```aleph
fn diag m:list list i64 -> list i64
 ex diag [[1 2] [3 4]] = [1 4]
 ex diag empty list i64 = empty i64
 map lam i nth nth m i i range 0 len m
```

### prefixes
```aleph
fn prefixes s:str -> list str
 ex prefixes "abc" = ["a" "ab" "abc"]
 ex prefixes "" = empty str
 fold lam a c push a cat else at a - len a 1 "" c empty str split s ""
```

### firstdup
```aleph
fn fdgo xs:list i64 i:i64 -> opt i64
 match at xs i some x if has index slice xs 0 i x some x fdgo xs + i 1 none none i64
fn firstdup xs:list i64 -> opt i64
 ex firstdup [1 2 3 2 1] = some 2
 ex firstdup [1 2] = none i64
 fdgo xs 0
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
 == join sort split a "" "" join sort split b "" ""
```

### sumcol
```aleph
fn myint t:str -> res i64 str
 match int t ok v ok v err x err "bad"
fn scnum fs:list str c:i64 acc:i64 rest:list str -> res i64 str
 let n try myint nth fs c
 scgo rest c + acc n
fn scparse line:str c:i64 acc:i64 rest:list str -> res i64 str
 let fs split line ","
 if or < c 0 <= len fs c err "bad" scnum fs c acc rest
fn scgo lines:list str c:i64 acc:i64 -> res i64 str
 match at lines 0 some line scparse line c acc slice lines 1 len lines none ok acc
fn sumcol s:str c:i64 -> res i64 str
 ex sumcol "1,2\n3,4" 1 = ok 6
 ex isok sumcol "1,x" 1 = false
 if == len s 0 ok 0 scgo split s "\n" c 0
```
