### avgstr

```aleph
fn avgstr s:str -> res f64 str
 ex avgstr "1 2" = ok 1.5
 ex isok avgstr "1 x" = false
 let ns try mapr lam t okor int t "bad" fields s
 let n len ns
 if == n 0 err "bad"
 ok / tof sum ns tof n
```

### dedupe

```aleph
fn addone acc:list i64 x:i64 -> list i64
 ex addone [1 2] 1 = [1 2]
 ex addone [1 2] 3 = [1 2 3]
 if has index acc x acc
 push acc x
fn dedupe xs:list i64 -> list i64
 ex dedupe [3 1 3 2 1] = [3 1 2]
 ex dedupe empty i64 = empty i64
 fold @addone empty i64 xs
```

### rle

```aleph
fn rlego cs:list str out:str cur:str n:i64 -> str
 ex rlego ["a" "a" "b"] "" "" 0 = "a2b1"
 if == len cs 0 cat cat out cur show n
 let c else at cs 0 ""
 if == n 0 rlego slice cs 1 len cs out c 1
 if == c cur rlego slice cs 1 len cs out cur + n 1
 rlego slice cs 1 len cs cat cat out cur show n c 1
fn rle s:str -> str
 ex rle "aaab" = "a3b1"
 ex rle "" = ""
 if == count s 0 ""
 rlego split s "" "" "" 0
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
fn better b:opt item x:item -> opt item
 ex better none item item "a" 5 = some item "a" 5
 ex better some item "a" 5 item "b" 3 = some item "b" 3
 if not has b some x
 let cur else b x
 if < .price x .price cur some x
 some cur
fn cheapest xs:list item -> opt str
 ex cheapest [item "a" 5 item "b" 3] = some "b"
 ex cheapest empty item = none str
 let b fold @better none item xs
 let best else b item "" 0
 if has b some .name best none str
```

### bylen

```aleph
fn byadd d:dict i64 list str w:str -> dict i64 list str
 ex byadd dict i64 list str "a" = put dict i64 list str 1 ["a"]
 let k count w
 let ws else get d k empty str
 put d k push ws w
fn bylen s:str -> dict i64 list str
 ex bylen "a bb c" = put put dict i64 list str 1 ["a" "c"] 2 ["bb"]
 ex bylen "" = dict i64 list str
 fold @byadd dict i64 list str fields s
```

### parseip

```aleph
fn inrange xs:list i64 -> bool
 ex inrange [0 255] = true
 ex inrange [256] = false
 fold lam b x and b and <= 0 x <= x 255 true xs
fn parseip s:str -> res list i64 str
 ex parseip "10.0.0.1" = ok [10 0 0 1]
 ex isok parseip "1.2.3" = false
 let ps split s "."
 if != len ps 4 err "bad"
 let ns try mapr lam p okor int p "bad" ps
 if not inrange ns err "bad"
 ok ns
```

### minmax

```aleph
fn pickmin a:i64 b:i64 -> i64
 ex pickmin 3 1 = 1
 min a b
fn pickmax a:i64 b:i64 -> i64
 ex pickmax 3 1 = 3
 max a b
fn minmax xs:list i64 -> opt list i64
 ex minmax [3 -1 2] = some [-1 3]
 ex minmax empty i64 = none list i64
 let first else at xs 0 0
 let lo fold @pickmin first xs
 let hi fold @pickmax first xs
 if == len xs 0 none list i64 some [lo hi]
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
 let o else ord c 0
 if not and <= 97 o <= o 122 c
 let s + 97 % + - o 97 k 26
 else chr s c
fn caesargo cs:list str k:i64 -> str
 ex caesargo ["a" "b"] 1 = "bc"
 if == len cs 0 ""
 let c else at cs 0 ""
 cat shiftc c k caesargo slice cs 1 len cs k
fn caesar s:str k:i64 -> str
 ex caesar "abz" 1 = "bca"
 ex caesar "" 3 = ""
 caesargo split s "" k
```

### occurs

```aleph
type ost w:str n:i64
fn ocount st:ost x:str -> ost
 ex .n ocount ost "a" 0 "a" = 1
 if == x .w st ost .w st + .n st 1
 ost .w st .n st
fn occurs s:str w:str -> i64
 ex occurs "a b a" "a" = 2
 ex occurs "" "a" = 0
 .n fold @ocount ost w 0 fields s
```

### hhmm

```aleph
fn hhmm s:str -> res i64 str
 ex hhmm "01:30" = ok 90
 ex isok hhmm "24:00" = false
 let ps split s ":"
 if != len ps 2 err "bad"
 let h try okor int else at ps 0 "" "bad"
 let m try okor int else at ps 1 "" "bad"
 if or < h 0 > h 23 err "bad"
 if or < m 0 > m 59 err "bad"
 ok + * h 60 m
```

### runops

```aleph
type op | inc | dbl | set v:i64
fn applyop acc:i64 o:op -> i64
 ex applyop 5 inc = 6
 ex applyop 5 dbl = 10
 ex applyop 5 set 3 = 3
 match o inc + acc 1 dbl * acc 2 set v v
fn runops ops:list op -> i64
 ex runops [inc inc dbl] = 4
 ex runops empty op = 0
 fold @applyop 0 ops
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
fn diagon m:list list i64 i:i64 -> list i64
 ex diagon [[1 2] [3 4]] 0 = [1 4]
 if == i len m empty i64
 let row else at m i empty i64
 cat [else at row i 0] diagon m + i 1
fn diag m:list list i64 -> list i64
 ex diag [[1 2] [3 4]] = [1 4]
 ex diag empty list i64 = empty i64
 diagon m 0
```

### prefixes

```aleph
fn prefs cs:list str i:i64 -> list str
 ex prefs ["a" "b" "c"] 1 = ["a" "ab" "abc"]
 if > i len cs empty str
 cat [join slice cs 0 i ""] prefs cs + i 1
fn prefixes s:str -> list str
 ex prefixes "abc" = ["a" "ab" "abc"]
 ex prefixes "" = empty str
 prefs split s "" 1
```

### firstdup

```aleph
type dstate seen:list i64 dup:opt i64
fn dstep st:dstate x:i64 -> dstate
 ex .dup dstep dstate empty i64 none i64 1 = none i64
 let found has .dup st
 if found st
 if has index .seen st x dstate .seen st some x
 dstate push .seen st x .dup st
fn firstdup xs:list i64 -> opt i64
 ex firstdup [1 2 3 2 1] = some 2
 ex firstdup [1 2] = none i64
 .dup fold @dstep dstate empty i64 none i64 xs
```

### lookall

```aleph
fn lookks d:dict str i64 ks:list str -> res list i64 str
 ex lookks put dict str i64 "a" 1 ["a"] = ok [1]
 if == len ks 0 ok empty i64
 let k else at ks 0 ""
 let v try okor get d k k
 let rest try lookks d slice ks 1 len ks
 ok push rest v
fn lookall d:dict str i64 ks:list str -> res list i64 str
 ex lookall put dict str i64 "a" 1 ["a" "a"] = ok [1 1]
 ex lookall dict str i64 ["b"] = err "b"
 lookks d ks
```

### anagram

```aleph
fn eqstrs xs:list str ys:list str -> bool
 ex eqstrs ["a" "b"] ["a" "b"] = true
 ex eqstrs ["a"] ["b"] = false
 if != len xs len ys false
 if == len xs 0 true
 if != else at xs 0 "" else at ys 0 "" false
 eqstrs slice xs 1 len xs slice ys 1 len ys
fn anagram a:str b:str -> bool
 ex anagram "listen" "silent" = true
 ex anagram "ab" "abb" = false
 eqstrs sort split a "" sort split b ""
```

### sumcol

```aleph
fn colval line:str c:i64 -> res i64 str
 ex colval "1,2" 1 = ok 2
 ex isok colval "1" 1 = false
 let fs split line ","
 if <= len fs c err "bad"
 let v try okor int else at fs c "" "bad"
 ok v
fn colsum ls:list str c:i64 -> res i64 str
 ex colsum ["1,2" "3,4"] 1 = ok 6
 if == len ls 0 ok 0
 let v try colval else at ls 0 "" c
 let rest try colsum slice ls 1 len ls c
 ok + v rest
fn sumcol s:str c:i64 -> res i64 str
 ex sumcol "1,2\n3,4" 1 = ok 6
 ex isok sumcol "1,x" 1 = false
 if == count s 0 ok 0
 colsum split s "\n" c
```
