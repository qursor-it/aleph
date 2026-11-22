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
fn dedgo xs:list i64 i:i64 acc:list i64 -> list i64
 ex dedgo [3 1 3 2 1] 0 empty i64 = [3 1 2]
 if == i len xs acc
 let x nth xs i
 let acc2 if has index acc x acc push acc x
 dedgo xs + i 1 acc2
fn dedupe xs:list i64 -> list i64
 ex dedupe [3 1 3 2 1] = [3 1 2]
 ex dedupe empty i64 = empty i64
 dedgo xs 0 empty i64
```

### rle
```aleph
fn rlgo cs:list str i:i64 cur:str n:i64 acc:str -> str
 ex rlgo ["a" "a" "b"] 0 "" 0 "" = "a2b1"
 if and == i len cs == n 0 acc
 if == i len cs cat acc cat cur show n
 let c nth cs i
 let same if == n 0 true == c cur
 let n1 if same + n 1 1
 let acc1 if or == n 0 same acc cat acc cat cur show n
 let cur1 if == n 0 c cur
 rlgo cs + i 1 cur1 n1 acc1
fn rle s:str -> str
 ex rle "aaab" = "a3b1"
 ex rle "" = ""
 rlgo split s "" 0 "" 0 ""
```

### dot
```aleph
fn dotgo a:list i64 b:list i64 i:i64 acc:i64 -> i64
 ex dotgo [1 2 3] [4 5 6] 0 0 = 32
 if == i len a acc
 dotgo a b + i 1 + acc * nth a i nth b i
fn dot a:list i64 b:list i64 -> i64
 ex dot [1 2 3] [4 5 6] = 32
 ex dot empty i64 empty i64 = 0
 dotgo a b 0 0
```

### cheapest
```aleph
type item name:str price:i64
fn chmin xs:list item i:i64 best:i64 -> i64
 ex chmin [item "a" 5 item "b" 3] 0 5 = 3
 if == i len xs best
 chmin xs + i 1 min best .price nth xs i
fn chname xs:list item i:i64 p:i64 -> str
 ex chname [item "a" 5 item "b" 3] 0 3 = "b"
 if == i len xs ""
 let it nth xs i
 if == .price it p .name it
 chname xs + i 1 p
fn cheapest xs:list item -> opt str
 ex cheapest [item "a" 5 item "b" 3] = some "b"
 ex cheapest empty item = none str
 if == len xs 0 none str
 let p chmin xs 0 .price nth xs 0
 some chname xs 0 p
```

### bylen
```aleph
fn blgo ws:list str i:i64 d:dict i64 list str -> dict i64 list str
 ex blgo ["a" "bb" "c"] 0 dict i64 list str = put put dict i64 list str 1 ["a" "c"] 2 ["bb"]
 if == i len ws d
 let w nth ws i
 let k len w
 let old else get d k empty str
 blgo ws + i 1 put d k push old w
fn bylen s:str -> dict i64 list str
 ex bylen "a bb c" = put put dict i64 list str 1 ["a" "c"] 2 ["bb"]
 ex bylen "" = dict i64 list str
 blgo fields s 0 dict i64 list str
```

### parseip
```aleph
fn ipok xs:list i64 i:i64 -> bool
 ex ipok [10 0 0 1] 0 = true
 ex ipok [256] 0 = false
 if == i len xs true
 let x nth xs i
 if or < x 0 > x 255 false
 ipok xs + i 1
fn parseip s:str -> res list i64 str
 ex parseip "10.0.0.1" = ok [10 0 0 1]
 ex isok parseip "1.2.3" = false
 let ps split s "."
 if != len ps 4 err "bad"
 let ns try mapr lam t okor int t "bad" ps
 if not ipok ns 0 err "bad"
 ok ns
```

### minmax
```aleph
fn mmgo xs:list i64 i:i64 lo:i64 hi:i64 -> list i64
 ex mmgo [3 -1 2] 1 3 3 = [-1 3]
 if == i len xs [lo hi]
 mmgo xs + i 1 min lo nth xs i max hi nth xs i
fn minmax xs:list i64 -> opt list i64
 ex minmax [3 -1 2] = some [-1 3]
 ex minmax empty i64 = none list i64
 if == len xs 0 none list i64
 some mmgo xs 1 nth xs 0 nth xs 0
```

### digitsum
```aleph
fn dsgo n:i64 acc:i64 -> i64
 ex dsgo 1234 0 = 10
 ex dsgo 0 0 = 0
 if == n 0 acc
 dsgo / n 10 + acc % n 10
fn digitsum n:i64 -> i64
 ex digitsum 1234 = 10
 ex digitsum 0 = 0
 dsgo n 0
```

### caesar
```aleph
fn csgo cs:list str i:i64 k:i64 acc:str -> str
 ex csgo ["a" "b" "z"] 0 1 "" = "bca"
 if == i len cs acc
 let c nth cs i
 let o else ord c 0
 let lo and >= o 97 <= o 122
 let c2 if lo else chr + 97 % + - o 97 k 26 c c
 csgo cs + i 1 k cat acc c2
fn caesar s:str k:i64 -> str
 ex caesar "abz" 1 = "bca"
 ex caesar "" 3 = ""
 csgo split s "" 0 k ""
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
 let h2 nth ps 0
 let m2 nth ps 1
 if or != len h2 2 != len m2 2 err "bad"
 let h try okor int h2 "bad"
 let m try okor int m2 "bad"
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
fn dgago m:list list i64 i:i64 acc:list i64 -> list i64
 ex dgago [[1 2] [3 4]] 0 empty i64 = [1 4]
 if == i len m acc
 let row nth m i
 dgago m + i 1 push acc nth row i
fn diag m:list list i64 -> list i64
 ex diag [[1 2] [3 4]] = [1 4]
 ex diag empty list i64 = empty i64
 dgago m 0 empty i64
```

### prefixes
```aleph
fn pfgo cs:list str i:i64 cur:str acc:list str -> list str
 ex pfgo ["a" "b" "c"] 0 "" empty str = ["a" "ab" "abc"]
 if == i len cs acc
 let nxt cat cur nth cs i
 pfgo cs + i 1 nxt push acc nxt
fn prefixes s:str -> list str
 ex prefixes "abc" = ["a" "ab" "abc"]
 ex prefixes "" = empty str
 pfgo split s "" 0 "" empty str
```

### firstdup
```aleph
fn fdgo xs:list i64 i:i64 seen:list i64 -> opt i64
 ex fdgo [1 2 3 2 1] 0 empty i64 = some 2
 if == i len xs none i64
 let x nth xs i
 if has index seen x some x
 fdgo xs + i 1 push seen x
fn firstdup xs:list i64 -> opt i64
 ex firstdup [1 2 3 2 1] = some 2
 ex firstdup [1 2] = none i64
 fdgo xs 0 empty i64
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
 == show sort split a "" show sort split b ""
```

### sumcol
```aleph
fn scgo ls:list str i:i64 c:i64 acc:i64 -> res i64 str
 ex scgo ["1,2" "3,4"] 0 1 0 = ok 6
 if == i len ls ok acc
 let fs split nth ls i ","
 if or < c 0 <= len fs c err "bad"
 let v try okor int nth fs c "bad"
 scgo ls + i 1 c + acc v
fn sumcol s:str c:i64 -> res i64 str
 ex sumcol "1,2\n3,4" 1 = ok 6
 ex isok sumcol "1,x" 1 = false
 if == s "" ok 0
 scgo split s "\n" 0 c 0
```
