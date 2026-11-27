### avgstr
```aleph
fn avgstr s:str -> res f64 str
 ex avgstr "1 2" = ok 1.5
 ex isok avgstr "1 x" = false
 ex isok avgstr "" = false
 let ns try mapr lam t okor int t "bad" fields s
 if == 0 len ns err "bad"
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
type rleacc c:str n:i64 o:str
fn rstep a:rleacc x:str -> rleacc
 ex rstep rleacc "a" 1 "a1" "a" = rleacc "a" 2 "a1"
 if == .c a x rleacc .c a + .n a 1 .o a
 if == 0 .n a rleacc x 1 .o a
 rleacc x 1 cat .o a cat .c a show .n a
fn rle s:str -> str
 ex rle "aaab" = "a3b1"
 ex rle "" = ""
 let a fold @rstep rleacc "" 0 "" split s ""
 if == 0 .n a .o a
 cat .o a cat .c a show .n a
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
 let best fold lam acc x match acc some b if < .price x .price b some x some b none some x none item xs
 match best some b some .name b none none str
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
 ex isok parseip "256.0.0.1" = false
 let parts split s "."
 let ns try mapr lam t okor int t "bad" parts
 if or != 4 len ns != 0 len filter lam x or < x 0 > x 255 ns err "bad"
 ok ns
```

### minmax
```aleph
fn minmax xs:list i64 -> opt list i64
 ex minmax [3 -1 2] = some [-1 3]
 ex minmax empty i64 = none list i64
 fold lam acc x match acc some p some [min x nth p 0 max x nth p 1] none some [x x] none list i64 xs
```

### digitsum
```aleph
fn digitsum n:i64 -> i64
 ex digitsum 1234 = 10
 ex digitsum 0 = 0
 sum map lam c else int c 0 split show n ""
```

### caesar
```aleph
fn cshift c:str k:i64 -> str
 ex cshift "a" 1 = "b"
 ex cshift "z" 1 = "a"
 ex cshift "!" 5 = "!"
 let o else ord c 0
 if and >= o 97 <= o 122 else chr + 97 % + - o 97 k 26 c c
fn caesar s:str k:i64 -> str
 ex caesar "abz" 1 = "bca"
 ex caesar "" 3 = ""
 join map lam c cshift c k split s "" ""
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
 ex isok hhmm "1:30" = false
 let ps split s ":"
 if != 2 len ps err "bad"
 let hs try mapr lam t okor int t "bad" ps
 let h nth hs 0
 let m nth hs 1
 if not and == 2 len nth ps 0 and == 2 len nth ps 1 and >= h 0 and <= h 23 and >= m 0 and <= m 59 err "bad"
 ok + * h 60 m
```

### runops
```aleph
type op | inc | dbl | set v:i64
fn runops ops:list op -> i64
 ex runops [inc inc dbl] = 4
 ex runops empty op = 0
 fold lam acc o match o inc + acc 1 dbl * acc 2 set x x 0 ops
```

### inorder
```aleph
type tree | tip | node l:tree v:i64 r:tree
fn inorder t:tree -> list i64
 ex inorder node node tip 1 tip 2 node tip 3 tip = [1 2 3]
 ex inorder tip = empty i64
 match t tip empty i64 node l v r cat cat inorder l [v] inorder r
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
 map lam k join slice split s "" 0 k "" range 1 + len split s "" 1
```

### firstdup
```aleph
type dupst seen:list i64 hit:opt i64
fn dstep a:dupst x:i64 -> dupst
 ex dstep dupst [1] none i64 2 = dupst [1 2] none i64
 if has .hit a a
 if has index .seen a x dupst .seen a some x
 dupst push .seen a x none i64
fn firstdup xs:list i64 -> opt i64
 ex firstdup [1 2 3 2 1] = some 2
 ex firstdup [1 2] = none i64
 .hit fold @dstep dupst empty i64 none i64 xs
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
 ex isok colval "1" 1 = false
 let f at split line "," c
 match f some x okor int x "bad" none err "bad"
fn sumcol s:str c:i64 -> res i64 str
 ex sumcol "1,2\n3,4" 1 = ok 6
 ex isok sumcol "1,x" 1 = false
 ex sumcol "" 0 = ok 0
 if == 0 len s ok 0
 let ns try mapr lam l colval l c split s "\n"
 ok sum ns
```
