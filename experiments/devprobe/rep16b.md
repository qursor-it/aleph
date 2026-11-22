### avgstr
```aleph
fn avgstr s:str -> res f64 str
 ex avgstr "1 2" = ok 1.5
 ex isok avgstr "1 x" = false
 ex isok avgstr " " = false
 let ws fields s
 let ns try mapr lam t okor int t "bad" ws
 if == len ns 0 err "bad" ok / tof sum ns tof len ns
```

### dedupe
```aleph
fn add1 acc:list i64 x:i64 -> list i64
 ex add1 [1] 2 = [1 2]
 ex add1 [1] 1 = [1]
 if has index acc x acc push acc x
fn dedupe xs:list i64 -> list i64
 ex dedupe [3 1 3 2 1] = [3 1 2]
 ex dedupe empty i64 = empty i64
 fold @add1 empty i64 xs
```

### rle
```aleph
fn runend cs:list str i:i64 ch:str -> i64
 ex runend ["a" "a" "b"] 0 "a" = 2
 if >= i len cs i if == nth cs i ch runend cs + i 1 ch i
fn rlego cs:list str i:i64 -> str
 ex rlego ["a" "a"] 0 = "a2"
 if >= i len cs ""
 let j runend cs i nth cs i
 cat nth cs i cat show - j i rlego cs j
fn rle s:str -> str
 ex rle "aaab" = "a3b1"
 ex rle "" = ""
 let cs split s ""
 rlego cs 0
```

### dot
```aleph
fn dot a:list i64 b:list i64 -> i64
 ex dot [1 2 3] [4 5 6] = 32
 ex dot empty i64 empty i64 = 0
 fold lam s i + s * nth a i nth b i 0 range 0 len a
```

### cheapest
```aleph
type item name:str price:i64
fn best xs:list item i:i64 bi:i64 -> i64
 ex best [item "a" 5 item "b" 3] 0 0 = 1
 if >= i len xs bi
 if < .price nth xs i .price nth xs bi i best xs + i 1 bi
fn cheapest xs:list item -> opt str
 ex cheapest [item "a" 5 item "b" 3] = some "b"
 ex cheapest empty item = none str
 if == len xs 0 none str
 some .name nth xs best xs 0 0
```

### bylen
```aleph
fn addw d:dict i64 list str w:str -> dict i64 list str
 ex addw dict i64 list str "ab" = put dict i64 list str 2 ["ab"]
 let k len w
 let old else get d k empty str
 put d k push old w
fn bylen s:str -> dict i64 list str
 ex bylen "a bb c" = put put dict i64 list str 1 ["a" "c"] 2 ["bb"]
 ex bylen "" = dict i64 list str
 fold @addw dict i64 list str fields s
```

### parseip
```aleph
fn parseip s:str -> res list i64 str
 ex parseip "10.0.0.1" = ok [10 0 0 1]
 ex isok parseip "1.2.3" = false
 ex isok parseip "1.2.3.256" = false
 let ps split s "."
 let ns try mapr lam t okor int t "bad" ps
 if or != len ps 4 != len filter lam x and >= x 0 <= x 255 ns len ns err "bad" ok ns
```

### minmax
```aleph
fn minmax xs:list i64 -> opt list i64
 ex minmax [3 -1 2] = some [-1 3]
 ex minmax empty i64 = none list i64
 if == len xs 0 none list i64
 some [fold lam a x min a x nth xs 0 xs fold lam a x max a x nth xs 0 xs]
```

### digitsum
```aleph
fn digitsum n:i64 -> i64
 ex digitsum 1234 = 10
 ex digitsum 0 = 0
 if < n 10 n + % n 10 digitsum / n 10
```

### caesar
```aleph
fn sh c:str k:i64 -> str
 ex sh "a" 1 = "b"
 ex sh "z" 1 = "a"
 let o else ord c -1
 if or < o 97 > o 122 c else chr + 97 % + - o 97 k 26 c
fn caesar s:str k:i64 -> str
 ex caesar "abz" 1 = "bca"
 ex caesar "" 3 = ""
 join map lam c sh c k split s "" ""
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
 if != len ps 2 err "bad"
 let hs nth ps 0
 let ms nth ps 1
 let h try okor int hs "bad"
 let m try okor int ms "bad"
 if or or or != len hs 2 != len ms 2 or < h 0 > h 23 or < m 0 > m 59 err "bad" ok + * h 60 m
```

### runops
```aleph
type op | inc | dbl | set v:i64
fn apply st:i64 o:op -> i64
 ex apply 3 inc = 4
 ex apply 3 set 10 = 10
 match o inc + st 1 dbl * st 2 set v v
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
 let cs split s ""
 map lam i join slice cs 0 i "" range 1 + len cs 1
```

### firstdup
```aleph
fn dupat xs:list i64 i:i64 -> opt i64
 ex dupat [1 2 1] 2 = some 1
 ex dupat [1 2] 1 = none i64
 if >= i len xs none i64
 if has index slice xs 0 i nth xs i some nth xs i dupat xs + i 1
fn firstdup xs:list i64 -> opt i64
 ex firstdup [1 2 3 2 1] = some 2
 ex firstdup [1 2] = none i64
 dupat xs 0
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
fn cell line:str c:i64 -> res i64 str
 ex cell "1,2" 1 = ok 2
 let fs split line ","
 if or < c 0 >= c len fs err "bad"
 okor int nth fs c "bad"
fn sumcol s:str c:i64 -> res i64 str
 ex sumcol "1,2\n3,4" 1 = ok 6
 ex isok sumcol "1,x" 1 = false
 if == s "" ok 0
 let ns try mapr lam l cell l c split s "\n"
 ok sum ns
```
