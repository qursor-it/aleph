### avgstr
```aleph
fn avgstr s:str -> res f64 str
 ex avgstr "1 2" = ok 1.5
 ex isok avgstr "1 x" = false
 let ns try mapr lam t okor int t "bad" fields s
 if == len ns 0 err "bad" ok / tof sum ns tof len ns
```

### dedupe
```aleph
fn dedupe xs:list i64 -> list i64
 ex dedupe [3 1 3 2 1] = [3 1 2]
 ex dedupe empty i64 = empty i64
 dedupeh xs empty i64

fn dedupeh ys:list i64 acc:list i64 -> list i64
 ex dedupeh [2 1] [3] = [3 2 1]
 let x else at ys 0 0
 if == len ys 0 acc if has index acc x dedupeh slice ys 1 len ys acc dedupeh slice ys 1 len ys push acc x
```

### rle
```aleph
fn rle s:str -> str
 ex rle "aaab" = "a3b1"
 ex rle "" = ""
 let cs split s ""
 if == len cs 0 "" join rleh cs else at cs 0 "" 0 empty str ""

fn rleh cs:list str cur:str n:i64 out:list str -> list str
 ex rleh ["a"] "a" 0 empty str = ["a1"]
 let x else at cs 0 ""
 if == len cs 0 push out cat cur show n if == x cur rleh slice cs 1 len cs cur + n 1 out rleh slice cs 1 len cs x 1 push out cat cur show n
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
 if == len xs 0 none str some .name cheapesth xs nth xs 0

fn cheapesth ys:list item best:item -> item
 ex cheapesth [item "b" 3] item "a" 5 = item "b" 3
 let x else at ys 0 best
 if == len ys 0 best if < .price x .price best cheapesth slice ys 1 len ys x cheapesth slice ys 1 len ys best
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
fn parseip s:str -> res list i64 str
 ex parseip "10.0.0.1" = ok [10 0 0 1]
 ex isok parseip "1.2.3" = false
 let fs split s "."
 if != len fs 4 err "bad" parth fs

fn parth fs:list str -> res list i64 str
 ex parth ["1" "2" "3" "4"] = ok [1 2 3 4]
 let ns try mapr lam t okor int t "bad" fs
 if > len filter lam n or < n 0 > n 255 ns 0 err "bad" ok ns
```

### minmax
```aleph
fn minmax xs:list i64 -> opt list i64
 ex minmax [3 -1 2] = some [-1 3]
 ex minmax empty i64 = none list i64
 if == len xs 0 none list i64 some minmaxh xs nth xs 0 nth xs 0

fn minmaxh ys:list i64 lo:i64 hi:i64 -> list i64
 ex minmaxh [3 -1 2] 3 3 = [-1 3]
 let x else at ys 0 lo
 if == len ys 0 [lo hi] minmaxh slice ys 1 len ys min lo x max hi x
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
fn caesar s:str k:i64 -> str
 ex caesar "abz" 1 = "bca"
 ex caesar "" 3 = ""
 join map lam ch caesar1 ch k split s ""

fn caesar1 ch:str k:i64 -> str
 ex caesar1 "a" 1 = "b"
 let o else ord ch 0
 if and >= o 97 <= o 122 else chr + 97 % + - o 97 k 26 "" ch
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
 let p split s ":"
 if != len p 2 err "bad" hhmmp p

fn hhmmp p:list str -> res i64 str
 ex hhmmp ["01" "30"] = ok 90
 let h try int nth p 0
 let m try int nth p 1
 if or or or or or != len nth p 0 2 != len nth p 1 2 < h 0 > h 23 < m 0 > m 59 err "bad" ok + * h 60 m
```

### runops
```aleph
type op | inc | dbl | set v:i64

fn runops ops:list op -> i64
 ex runops [inc inc dbl] = 4
 ex runops empty op = 0
 fold lam acc o runop acc o 0 ops

fn runop acc:i64 o:op -> i64
 ex runop 3 inc = 4
 ex runop 4 dbl = 8
 ex runop 5 set 7 = 12
 match o inc + acc 1 dbl * acc 2 set x + acc x
```

### inorder
```aleph
type tree | tip | node l:tree v:i64 r:tree

fn inorder t:tree -> list i64
 ex inorder node node tip 1 tip 2 node tip 3 tip = [1 2 3]
 ex inorder tip = empty i64
 match t tip empty i64 node l n r cat cat inorder l [n] inorder r
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
fn firstdup xs:list i64 -> opt i64
 ex firstdup [1 2 3 2 1] = some 2
 ex firstdup [1 2] = none i64
 firstduph xs empty i64

fn firstduph ys:list i64 seen:list i64 -> opt i64
 ex firstduph [2 1] [1] = some 1
 let x else at ys 0 0
 if == len ys 0 none i64 if has index seen x some x firstduph slice ys 1 len ys push seen x
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
 and == len a len b == show sort split a "" show sort split b ""
```

### sumcol
```aleph
fn sumcol s:str c:i64 -> res i64 str
 ex sumcol "1,2\n3,4" 1 = ok 6
 ex isok sumcol "1,x" 1 = false
 if == len s 0 ok 0 sumcol2 s c

fn sumcol2 s:str c:i64 -> res i64 str
 ex sumcol2 "1,2\n3,4" 1 = ok 6
 let ns try mapr lam ln if or < c 0 >= c len split ln "," err "bad" okor int nth split ln "," c "bad" split s "\n"
 ok sum ns
```
