# Aleph answers

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
 fold lam acc x if has index acc x acc push acc x empty i64 xs
```

### rle
```aleph
fn runlen c:str cs:list str -> i64
 ex runlen "a" ["a" "a" "b"] = 2
 if == len cs 0 0
 if == nth cs 0 c + 1 runlen c slice cs 1 len cs
 0
fn rlego cs:list str -> str
 ex rlego ["a" "a" "b"] = "a2b1"
 if == len cs 0 ""
 let c nth cs 0
 let n runlen c cs
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
 if == len xs 0 none str
 let best fold lam acc x if < .price x .price acc x acc nth xs 0 slice xs 1 len xs
 some .name best
```

### bylen
```aleph
fn bylen s:str -> dict i64 list str
 ex bylen "a bb c" = put put dict i64 list str 1 ["a" "c"] 2 ["bb"]
 ex bylen "" = dict i64 list str
 if == s "" dict i64 list str
 fold lam d w put d len w push else get d len w empty str w dict i64 list str fields s
```

### parseip
```aleph
fn parseip s:str -> res list i64 str
 ex parseip "10.0.0.1" = ok [10 0 0 1]
 ex isok parseip "1.2.3" = false
 let parts split s "."
 if != len parts 4 err "bad"
 let ns try mapr lam t okor int t "bad" parts
 if != 0 len filter lam n or < n 0 > n 255 ns err "bad"
 ok ns
```

### minmax
```aleph
fn minmax xs:list i64 -> opt list i64
 ex minmax [3 -1 2] = some [-1 3]
 ex minmax empty i64 = none list i64
 if == len xs 0 none list i64
 let mn fold lam p q if < q p q p nth xs 0 slice xs 1 len xs
 let mx fold lam r u if > u r u r nth xs 0 slice xs 1 len xs
 some [mn mx]
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
fn shiftc c:str k:i64 -> str
 ex shiftc "a" 1 = "b"
 ex shiftc "z" 1 = "a"
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
 len filter lam x == x w fields s
```

### hhmm
```aleph
fn hhmm s:str -> res i64 str
 ex hhmm "01:30" = ok 90
 ex isok hhmm "24:00" = false
 let ps split s ":"
 if != len ps 2 err "bad"
 let h else at ps 0 ""
 let m else at ps 1 ""
 if or != len h 2 != len m 2 err "bad"
 let hn try okor int h "bad"
 let mn try okor int m "bad"
 if or < hn 0 > hn 23 err "bad"
 if or < mn 0 > mn 59 err "bad"
 ok + * hn 60 mn
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
 match t tip empty i64 node l v r cat push inorder l v inorder r
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
fn fdgo xs:list i64 i:i64 -> opt i64
 ex fdgo [1 1] 1 = some 1
 if >= i len xs none i64
 let x nth xs i
 if has index slice xs 0 i x some x
 fdgo xs + i 1
fn firstdup xs:list i64 -> opt i64
 ex firstdup [1 2 3 2 1] = some 2
 ex firstdup [1 2] = none i64
 fdgo xs 1
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
 ex colval "1" 1 = err "bad"
 let fs split line ","
 if or < c 0 <= len fs c err "bad"
 okor int nth fs c "bad"
fn sumcol s:str c:i64 -> res i64 str
 ex sumcol "1,2\n3,4" 1 = ok 6
 ex isok sumcol "1,x" 1 = false
 if == s "" ok 0
 let vals try mapr lam line colval line c split s "\n"
 ok sum vals
```
