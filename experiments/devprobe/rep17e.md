### avgstr
```aleph
fn avgstr s:str -> res f64 str
 ex avgstr "1 2" = ok 1.5
 ex isok avgstr "1 x" = false
 ex isok avgstr "" = false
 let ns try mapr lam t okor int t "bad" fields s
 if == len ns 0 err "bad"
 ok / tof sum ns tof len ns
```

### dedupe
```aleph
fn dedupe xs:list i64 -> list i64
 ex dedupe [3 1 3 2 1] = [3 1 2]
 ex dedupe empty i64 = empty i64
 fold lam acc x if has index acc x acc push acc x empty i64 xs
```

### rle
```aleph
fn runlen cs:list str c:str -> i64
 ex runlen ["a" "a" "b"] "a" = 2
 ex runlen empty str "a" = 0
 if == 0 len cs 0
 if == nth cs 0 c + 1 runlen slice cs 1 len cs c
 0
fn rlego cs:list str -> str
 ex rlego ["a" "a" "b"] = "a2b1"
 ex rlego empty str = ""
 if == 0 len cs ""
 let c nth cs 0
 let n runlen cs c
 cat cat c show n rlego slice cs n len cs
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
fn cheapest xs:list item -> opt str
 ex cheapest [item "a" 5 item "b" 3] = some "b"
 ex cheapest empty item = none str
 let best fold lam acc it match acc some cur if < .price it .price cur some it cur none some it none item xs
 match best some nm some .name nm none none str
```

### bylen
```aleph
fn bylen s:str -> dict i64 list str
 ex bylen "a bb c" = put put dict i64 list str 1 ["a" "c"] 2 ["bb"]
 ex bylen "" = dict i64 list str
 fold lam acc w put acc len w push else get acc len w empty str w dict i64 list str fields s
```

### parseip
```aleph
fn parseip s:str -> res list i64 str
 ex parseip "10.0.0.1" = ok [10 0 0 1]
 ex isok parseip "1.2.3" = false
 ex isok parseip "256.0.0.1" = false
 let ps split s "."
 if != len ps 4 err "bad"
 let ns try mapr lam t okor int t "bad" ps
 if not fold lam a x and a and >= x 0 <= x 255 true ns err "bad"
 ok ns
```

### minmax
```aleph
fn minmax xs:list i64 -> opt list i64
 ex minmax [3 -1 2] = some [-1 3]
 ex minmax empty i64 = none list i64
 if == 0 len xs none list i64
 let lo fold lam a x min a x nth xs 0 xs
 let hi fold lam a x max a x nth xs 0 xs
 some [lo hi]
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
fn shch c:str k:i64 -> str
 ex shch "a" 1 = "b"
 ex shch "z" 1 = "a"
 ex shch "A" 1 = "A"
 let o else ord c 0
 if and >= o 97 <= o 122 else chr + 97 % + - o 97 k 26 c c
fn caesar s:str k:i64 -> str
 ex caesar "abz" 1 = "bca"
 ex caesar "" 3 = ""
 join map lam c shch c k split s "" ""
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
 ex isok hhmm "12:60" = false
 let ps split s ":"
 if != len ps 2 err "bad"
 let h try okor int nth ps 0 "bad"
 let m try okor int nth ps 1 "bad"
 if or < h 0 > h 23 err "bad"
 if or < m 0 > m 59 err "bad"
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
 map lam i join slice cs 0 + i 1 "" range 0 len cs
```

### firstdup
```aleph
type fd seen:list i64 found:opt i64
fn fdstep a:fd x:i64 -> fd
 ex fdstep fd empty i64 none i64 1 = fd [1] none i64
 ex fdstep fd [1] some 1 2 = fd [1] some 1
 if has .found a a
 if has index .seen a x fd .seen a some x
 fd push .seen a x none i64
fn firstdup xs:list i64 -> opt i64
 ex firstdup [1 2 3 2 1] = some 2
 ex firstdup [1 2] = none i64
 let r fold @fdstep fd empty i64 none i64 xs
 .found r
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
 let ca sort split a ""
 let cb sort split b ""
 and == len ca len cb fold lam acc i and acc == nth ca i nth cb i true range 0 len ca
```

### sumcol
```aleph
fn sumcol s:str c:i64 -> res i64 str
 ex sumcol "1,2\n3,4" 1 = ok 6
 ex isok sumcol "1,x" 1 = false
 ex sumcol "" 0 = ok 0
 if == 0 len s ok 0
 let rows mapr lam ln okor int else at split ln "," c "bad" "bad" split s "\n"
 let ns try rows
 ok sum ns
```
