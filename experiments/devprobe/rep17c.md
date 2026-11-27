### avgstr

```aleph
fn avgstr s:str -> res f64 str
 ex avgstr "1 2" = ok 1.5
 ex isok avgstr "1 x" = false
 match mapr lam t if isok int t ok else int t 0 err "bad" fields s ok ns if == len ns 0 err "bad" ok / tof sum ns tof len ns err e err e
```

### dedupe

```aleph
type dedup acc:list i64
fn dedupadd st:dedup x:i64 -> dedup
 ex dedupadd dedup empty i64 1 = dedup [1]
 ex dedupadd dedup [1] 1 = dedup [1]
 if has index .acc st x st dedup push .acc st x
fn dedupe xs:list i64 -> list i64
 ex dedupe [3 1 3 2 1] = [3 1 2]
 ex dedupe empty i64 = empty i64
 .acc fold @dedupadd dedup empty i64 xs
```

### rle

```aleph
type run c:str n:i64
fn rleadd acc:list run ch:str -> list run
 ex rleadd empty run "a" = [run "a" 1]
 ex rleadd [run "a" 2] "a" = [run "a" 3]
 let hd else at acc 0 run "" 0
 if == ch .c hd cat [run ch + .n hd 1] slice acc 1 len acc cat [run ch 1] acc
fn rle s:str -> str
 ex rle "aaab" = "a3b1"
 ex rle "" = ""
 join map lam r cat .c r show .n r rev fold @rleadd empty run split s "" ""
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
 match fold lam st it if has st if < .price it .price else st it some it st some it none item xs some x some .name x none none str
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
fn tok2ip t:str -> res i64 str
 ex tok2ip "007" = ok 7
 ex isok tok2ip "x" = false
 let n else int t -1
 if or < n 0 > n 255 err "bad" ok n
fn parseip s:str -> res list i64 str
 ex parseip "10.0.0.1" = ok [10 0 0 1]
 ex isok parseip "1.2.3" = false
 match mapr @tok2ip split s "." ok ns if == len ns 4 ok ns err "bad" err e err e
```

### minmax

```aleph
fn minmax xs:list i64 -> opt list i64
 ex minmax [3 -1 2] = some [-1 3]
 ex minmax empty i64 = none list i64
 if == len xs 0 none list i64 some [fold lam a x min a x nth xs 0 xs fold lam a x max a x nth xs 0 xs]
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
fn shiftc ch:str k:i64 -> str
 ex shiftc "a" 1 = "b"
 ex shiftc "z" 1 = "a"
 ex shiftc "-" 5 = "-"
 let o else ord ch 0
 if and >= o 97 <= o 122 else chr + 97 % + - o 97 k 26 ch ch
fn caesar s:str k:i64 -> str
 ex caesar "abz" 1 = "bca"
 ex caesar "" 3 = ""
 join map lam ch shiftc ch k split s "" ""
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
fn digch c:str -> bool
 ex digch "5" = true
 ex digch "x" = false
 and >= else ord c -1 48 <= else ord c -1 57
fn pfield t:str -> res i64 str
 ex pfield "05" = ok 5
 ex isok pfield "5" = false
 let cs split t ""
 if != len cs 2 err "bad" if and digch nth cs 0 digch nth cs 1 ok else int t 0 err "bad"
fn combine ns:list i64 -> res i64 str
 ex combine [1 30] = ok 90
 ex isok combine [24 0] = false
 if and <= nth ns 0 23 <= nth ns 1 59 ok + * nth ns 0 60 nth ns 1 err "bad"
fn hhmm s:str -> res i64 str
 ex hhmm "01:30" = ok 90
 ex isok hhmm "24:00" = false
 let ps split s ":"
 match at ps 2 some x err "bad" none match mapr @pfield ps ok ns if == len ns 2 combine ns err "bad" err e err "bad"
```

### runops

```aleph
type op | inc | dbl | set v:i64
fn runops ops:list op -> i64
 ex runops [inc inc dbl] = 4
 ex runops empty op = 0
 fold lam st o match o inc + st 1 dbl * st 2 set v v 0 ops
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
type pfx cur:str out:list str
fn pfxadd st:pfx ch:str -> pfx
 ex pfxadd pfx "" empty str "a" = pfx "a" ["a"]
 pfx cat .cur st ch push .out st cat .cur st ch
fn prefixes s:str -> list str
 ex prefixes "abc" = ["a" "ab" "abc"]
 ex prefixes "" = empty str
 .out fold @pfxadd pfx "" empty str split s ""
```

### firstdup

```aleph
fn fd xs:list i64 i:i64 -> opt i64
 ex fd [1 1] 0 = some 1
 ex fd [1 2] 0 = none i64
 if >= i len xs none i64 if < else index xs nth xs i -1 i some nth xs i fd xs + i 1
fn firstdup xs:list i64 -> opt i64
 ex firstdup [1 2 3 2 1] = some 2
 ex firstdup [1 2] = none i64
 fd xs 0
```

### lookall

```aleph
fn lookall d:dict str i64 ks:list str -> res list i64 str
 ex lookall put dict str i64 "a" 1 ["a" "a"] = ok [1 1]
 ex lookall dict str i64 ["b"] = err "b"
 fold lam acc k if isok acc if has get d k ok push else acc empty i64 else get d k 0 err k acc ok empty i64 ks
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
fn col ln:str c:i64 -> res i64 str
 ex col "1,2" 1 = ok 2
 ex isok col "1" 3 = false
 let fs split ln ","
 if or < c 0 >= c len fs err "bad" if isok int nth fs c ok else int nth fs c 0 err "bad"
fn sumcol s:str c:i64 -> res i64 str
 ex sumcol "1,2\n3,4" 1 = ok 6
 ex isok sumcol "1,x" 1 = false
 let ls split s "\n"
 let vs if == len s 0 ok empty i64 mapr lam ln col ln c ls
 match vs ok ns ok sum ns err e err e
```
