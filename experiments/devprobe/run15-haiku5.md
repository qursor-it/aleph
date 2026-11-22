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
fn same cs:list str c:str -> i64
 ex same ["a" "a" "b"] "a" = 2
 ex same empty str "a" = 0
 if == len cs 0 0
 if != nth cs 0 c 0
 + 1 same slice cs 1 len cs c

fn rlefrom cs:list str -> str
 ex rlefrom ["a" "a" "b"] = "a2b1"
 ex rlefrom empty str = ""
 if == len cs 0 ""
 let n same cs nth cs 0
 cat cat nth cs 0 show n rlefrom slice cs n len cs

fn rle s:str -> str
 ex rle "aaab" = "a3b1"
 ex rle "" = ""
 rlefrom split s ""
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
 if == len xs 0 none str
 let best fold lam b x if < .price x .price b x b nth xs 0 xs
 some .name best
```

### bylen
```aleph
fn bylen s:str -> dict i64 list str
 ex bylen "a bb c" = put put dict i64 list str 1 ["a" "c"] 2 ["bb"]
 ex bylen "" = dict i64 list str
 fold lam d w put d len w push else get d len w empty str w dict i64 list str fields s
```

### parseip
```aleph
fn octet t:str -> res i64 str
 ex octet "255" = ok 255
 ex isok octet "256" = false
 let n try okor int t "bad"
 if or < n 0 > n 255 err "bad"
 ok n

fn parseip s:str -> res list i64 str
 ex parseip "10.0.0.1" = ok [10 0 0 1]
 ex isok parseip "1.2.3" = false
 let parts split s "."
 if != len parts 4 err "bad"
 mapr lam t octet t parts
```

### minmax
```aleph
fn minmax xs:list i64 -> opt list i64
 ex minmax [3 -1 2] = some [-1 3]
 ex minmax empty i64 = none list i64
 if == len xs 0 none list i64
 let lo fold lam a x min a x nth xs 0 xs
 let hi fold lam a x max a x nth xs 0 xs
 some [lo hi]
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
fn shiftch c:str k:i64 -> str
 ex shiftch "a" 1 = "b"
 ex shiftch "z" 1 = "a"
 ex shiftch "A" 1 = "A"
 let n else ord c 0
 if not and >= n 97 <= n 122 c
 else chr + 97 % + - n 97 k 26 c

fn caesar s:str k:i64 -> str
 ex caesar "abz" 1 = "bca"
 ex caesar "" 3 = ""
 join map lam c shiftch c k split s "" ""
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
fn part p:str lim:i64 -> res i64 str
 ex part "09" 59 = ok 9
 ex isok part "60" 59 = false
 if != len p 2 err "bad"
 let n try okor int p "bad"
 if or < n 0 > n lim err "bad"
 ok n

fn hhmm s:str -> res i64 str
 ex hhmm "01:30" = ok 90
 ex isok hhmm "24:00" = false
 let parts split s ":"
 if != len parts 2 err "bad"
 let h try part nth parts 0 23
 let m try part nth parts 1 59
 ok + * h 60 m
```

### runops
```aleph
type op | inc | dbl | set v:i64
fn step a:i64 o:op -> i64
 ex step 3 dbl = 6
 ex step 3 set 7 = 7
 match o inc + a 1 dbl * a 2 set k k
fn runops ops:list op -> i64
 ex runops [inc inc dbl] = 4
 ex runops empty op = 0
 fold @step 0 ops
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
 map lam n join slice cs 0 n "" range 1 + len cs 1
```

### firstdup
```aleph
fn firstdup xs:list i64 -> opt i64
 ex firstdup [1 2 3 2 1] = some 2
 ex firstdup [1 2] = none i64
 let ds filter lam i has index slice xs 0 i nth xs i range 0 len xs
 if == len ds 0 none i64
 some nth xs nth ds 0
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
 ex isok cell "1" 1 = false
 ex isok cell "1,x" 1 = false
 let fs split line ","
 if or < c 0 >= c len fs err "bad"
 okor int nth fs c "bad"

fn sumcol s:str c:i64 -> res i64 str
 ex sumcol "1,2\n3,4" 1 = ok 6
 ex isok sumcol "1,x" 1 = false
 if == s "" ok 0
 let vs try mapr lam l cell l c split s "\n"
 ok sum vs
```
