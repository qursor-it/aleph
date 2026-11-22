### avgstr

```aleph
fn avgstr s:str -> res f64 str
 ex avgstr "1 2" = ok 1.5
 ex isok avgstr "1 x" = false
 let ws fields s
 if == len ws 0 err "bad" avgws ws
fn avgws ws:list str -> res f64 str
 ex avgws ["1" "2"] = ok 1.5
 let ns try mapr lam t okor int t "bad" ws
 ok / tof sum ns tof len ns
```

### dedupe

```aleph
fn dedupe xs:list i64 -> list i64
 ex dedupe [3 1 3 2 1] = [3 1 2]
 ex dedupe empty i64 = empty i64
 fold lam ys y if has index ys y ys push ys y empty i64 xs
```

### rle

```aleph
fn rle s:str -> str
 ex rle "aaab" = "a3b1"
 ex rle "" = ""
 rlego split s "" "" 0 ""
fn rlego cs:list str prev:str cnt:i64 acc:str -> str
 ex rlego ["a" "a" "b"] "" 0 "" = "a2b1"
 if == len cs 0 rleflush prev cnt acc rlenext nth cs 0 slice cs 1 len cs prev cnt acc
fn rlenext c:str rest:list str prev:str cnt:i64 acc:str -> str
 ex rlenext "b" empty str "a" 2 "" = "a2b1"
 if == c prev rlego rest prev + cnt 1 acc rlego rest c 1 rleflush prev cnt acc
fn rleflush prev:str cnt:i64 acc:str -> str
 ex rleflush "a" 2 "" = "a2"
 ex rleflush "" 0 "" = ""
 if == cnt 0 acc cat acc cat prev show cnt
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
 match fold @cheaper none item xs some z some .name z none none str
fn cheaper best:opt item it:item -> opt item
 ex cheaper none item item "a" 5 = some item "a" 5
 ex cheaper some item "a" 5 item "b" 3 = some item "b" 3
 match best some b some if < .price it .price b it b none some it
```

### bylen

```aleph
fn bylen s:str -> dict i64 list str
 ex bylen "a bb c" = put put dict i64 list str 1 ["a" "c"] 2 ["bb"]
 ex bylen "" = dict i64 list str
 fold lam d w put d len w push else get d len w empty list str w dict i64 list str fields s
```

### parseip

```aleph
fn parseip s:str -> res list i64 str
 ex parseip "10.0.0.1" = ok [10 0 0 1]
 ex isok parseip "1.2.3" = false
 let xs split s "."
 if != len xs 4 err "bad" parseoctets xs
fn parseoctets xs:list str -> res list i64 str
 ex parseoctets ["1" "2" "3" "4"] = ok [1 2 3 4]
 let ns try mapr @octet xs
 ok ns
fn octet t:str -> res i64 str
 ex octet "10" = ok 10
 ex isok octet "300" = false
 let n try okor int t "bad"
 if and >= n 0 <= n 255 ok n err "bad"
```

### minmax

```aleph
fn minmax xs:list i64 -> opt list i64
 ex minmax [3 -1 2] = some [-1 3]
 ex minmax empty i64 = none list i64
 fold @mm none list i64 xs
fn mm best:opt list i64 x:i64 -> opt list i64
 ex mm none i64 5 = some [5 5]
 ex mm some [1 3] 2 = some [1 3]
 match best some b some [min nth b 0 x max nth b 1 x] none some [x x]
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
 join map lam c shiftc c k split s "" ""
fn shiftc c:str k:i64 -> str
 ex shiftc "a" 1 = "b"
 ex shiftc "Z" 1 = "Z"
 let o else ord c 0
 if and >= o 97 <= o 122 else chr + 97 % + - o 97 k 26 c c
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
 let xs split s ":"
 if != len xs 2 err "bad" hhcalc xs
fn hhcalc xs:list str -> res i64 str
 ex hhcalc ["01" "30"] = ok 90
 let h try hhpart nth xs 0 0 23
 let m try hhpart nth xs 1 0 59
 ok + * h 60 m
fn hhpart t:str lo:i64 hi:i64 -> res i64 str
 ex hhpart "01" 0 23 = ok 1
 ex isok hhpart "1" 0 23 = false
 let n try okor int t "bad"
 if != len t 2 err "bad" if and >= n lo <= n hi ok n err "bad"
```

### runops

```aleph
type op | inc | dbl | set v:i64
fn runops ops:list op -> i64
 ex runops [inc inc dbl] = 4
 ex runops empty op = 0
 fold lam acc o match o inc + acc 1 dbl * acc 2 set v v 0 ops
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
fn firstdup xs:list i64 -> opt i64
 ex firstdup [1 2 3 2 1] = some 2
 ex firstdup [1 2] = none i64
 firstdupgo xs empty i64
fn firstdupgo xs:list i64 seen:list i64 -> opt i64
 ex firstdupgo [1 2 3 2] empty i64 = some 2
 match at xs 0 some x if has index seen x some x firstdupgo slice xs 1 len xs push seen x none none i64
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
fn sumcol s:str c:i64 -> res i64 str
 ex sumcol "1,2\n3,4" 1 = ok 6
 ex isok sumcol "1,x" 1 = false
 if == s "" ok 0 sumcolgo s c
fn sumcolgo s:str c:i64 -> res i64 str
 ex sumcolgo "1,2\n3,4" 1 = ok 6
 let ls split s "\n"
 let vs try mapr lam l colval l c ls
 ok sum vs
fn colval l:str c:i64 -> res i64 str
 ex colval "1,2" 1 = ok 2
 ex isok colval "1" 1 = false
 let fs split l ","
 if or < c 0 <= len fs c err "bad" okor int nth fs c "bad"
```
