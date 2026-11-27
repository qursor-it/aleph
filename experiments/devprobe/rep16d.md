### avgstr
```aleph
fn avgstr s:str -> res f64 str
 ex avgstr "1 2" = ok 1.5
 ex isok avgstr "1 x" = false
 ex avgstr "" = err "bad"
 let ns try mapr lam t okor int t "bad" fields s
 if == len ns 0 err "bad" ok / tof sum ns tof len ns
```

### dedupe
```aleph
fn adduniq seen:list i64 xs:list i64 -> list i64
 ex adduniq empty i64 [3 1 3] = [3 1]
 if == len xs 0 seen if has index seen nth xs 0 adduniq seen slice xs 1 len xs adduniq push seen nth xs 0 slice xs 1 len xs
fn dedupe xs:list i64 -> list i64
 ex dedupe [3 1 3 2 1] = [3 1 2]
 ex dedupe empty i64 = empty i64
 adduniq empty i64 xs
```

### rle
```aleph
fn rlego cs:list str cur:str n:i64 acc:list str -> list str
 ex rlego ["a" "a" "b"] "" 0 empty str = ["a2" "b1"]
 if == len cs 0 if == n 0 acc push acc cat cur show n if == nth cs 0 cur rlego slice cs 1 len cs cur + n 1 acc rlego slice cs 1 len cs nth cs 0 1 if == n 0 acc push acc cat cur show n
fn rle s:str -> str
 ex rle "aaab" = "a3b1"
 ex rle "" = ""
 join rlego split s "" "" 0 empty str ""
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
fn cheaper a:item b:item -> item
 ex cheaper item "a" 5 item "b" 3 = item "b" 3
 if < .price b .price a b a
fn best xs:list item acc:item -> item
 ex best [item "a" 5 item "b" 3] item "z" 100 = item "b" 3
 if == len xs 0 acc best slice xs 1 len xs cheaper acc nth xs 0
fn cheapest xs:list item -> opt str
 ex cheapest [item "a" 5 item "b" 3] = some "b"
 ex cheapest empty item = none str
 if == len xs 0 none str some .name best xs nth xs 0
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
fn ipfield t:str -> res i64 str
 ex ipfield "10" = ok 10
 ex ipfield "256" = err "bad"
 let n try okor int t "bad"
 if or < n 0 > n 255 err "bad" ok n
fn parseip s:str -> res list i64 str
 ex parseip "10.0.0.1" = ok [10 0 0 1]
 ex isok parseip "1.2.3" = false
 let fs split s "."
 if != len fs 4 err "bad" mapr @ipfield fs
```

### minmax
```aleph
fn minmax xs:list i64 -> opt list i64
 ex minmax [3 -1 2] = some [-1 3]
 ex minmax empty i64 = none list i64
 if == len xs 0 none list i64 some [nth sort xs 0 nth sort xs - len xs 1]
```

### digitsum
```aleph
fn digitsum n:i64 -> i64
 ex digitsum 1234 = 10
 ex digitsum 0 = 0
 ex digitsum 10 = 1
 if < n 10 n + % n 10 digitsum / n 10
```

### caesar
```aleph
fn shiftc c:str k:i64 -> str
 ex shiftc "a" 1 = "b"
 ex shiftc "z" 1 = "a"
 ex shiftc "A" 1 = "A"
 let o else ord c 0
 if and >= o 97 <= o 122 else chr + 97 % + - o 97 k 26 c c
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
 fold lam a x + a if == x w 1 0 0 fields s
```

### hhmm
```aleph
fn hhmm2 p:list str -> res i64 str
 ex hhmm2 ["01" "30"] = ok 90
 ex hhmm2 ["1" "30"] = err "bad"
 let hh try okor int nth p 0 "bad"
 let mm try okor int nth p 1 "bad"
 if or != len nth p 0 2 != len nth p 1 2 err "bad" if or < hh 0 or > hh 23 or < mm 0 > mm 59 err "bad" ok + * hh 60 mm
fn hhmm s:str -> res i64 str
 ex hhmm "01:30" = ok 90
 ex isok hhmm "24:00" = false
 ex isok hhmm "1:30" = false
 let parts split s ":"
 if != len parts 2 err "bad" hhmm2 parts
```

### runops
```aleph
type op | inc | dbl | set v:i64
fn applyop acc:i64 o:op -> i64
 ex applyop 0 inc = 1
 ex applyop 5 dbl = 10
 ex applyop 5 set 3 = 3
 match o inc + acc 1 dbl * acc 2 set x x
fn runopsgo acc:i64 ops:list op -> i64
 ex runopsgo 0 [inc inc dbl] = 4
 if == len ops 0 acc runopsgo applyop acc nth ops 0 slice ops 1 len ops
fn runops ops:list op -> i64
 ex runops [inc inc dbl] = 4
 ex runops empty op = 0
 runopsgo 0 ops
```

### inorder
```aleph
type tree | tip | node l:tree v:i64 r:tree
fn inorder t:tree -> list i64
 ex inorder node node tip 1 tip 2 node tip 3 tip = [1 2 3]
 ex inorder tip = empty i64
 match t tip empty i64 node a b c cat inorder a cat [b] inorder c
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
fn fd seen:list i64 xs:list i64 -> opt i64
 ex fd empty i64 [1 2 1] = some 1
 if == len xs 0 none i64 if has index seen nth xs 0 some nth xs 0 fd push seen nth xs 0 slice xs 1 len xs
fn firstdup xs:list i64 -> opt i64
 ex firstdup [1 2 3 2 1] = some 2
 ex firstdup [1 2] = none i64
 fd empty i64 xs
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
fn colval line:str c:i64 -> res i64 str
 ex colval "1,2" 1 = ok 2
 ex colval "1,2" 5 = err "bad"
 ex colval "1,x" 1 = err "bad"
 let ps split line ","
 if or < c 0 >= c len ps err "bad" okor int nth ps c "bad"
fn sumlines ls:list str c:i64 -> res i64 str
 ex sumlines ["1,2" "3,4"] 1 = ok 6
 let vs try mapr lam l colval l c ls
 ok sum vs
fn sumcol s:str c:i64 -> res i64 str
 ex sumcol "1,2\n3,4" 1 = ok 6
 ex isok sumcol "1,x" 1 = false
 ex sumcol "" 0 = ok 0
 if == len s 0 ok 0 sumlines split s "\n" c
```
