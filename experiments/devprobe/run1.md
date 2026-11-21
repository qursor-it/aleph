### vowels
```aleph
fn vowel c:str -> bool
 ex vowel "a" = true
 ex vowel "z" = false
 contains "aeiouAEIOU" c
fn vowels s:str -> i64
 ex vowels "Hello" = 2
 ex vowels "" = 0
 len filter @vowel split s ""
```

### second
```aleph
fn second xs:list i64 -> opt i64
 ex second [3 1 4 4] = some 3
 ex second [7] = none i64
 ex second empty i64 = none i64
 let u uniq sort xs
 if < len u 2 none i64 at rev u 1
```

### rle
```aleph
type rlst c:str n:i64 out:list str
fn rlstep a:rlst x:str -> rlst
 ex .n rlstep rlst "a" 1 empty str "b" = 2
 ex .out rlstep rlst "a" 1 empty str "b" = ["a1"]
 if == .n a 0 rlst x 1 .out a if == .c a x rlst .c a + .n a 1 .out a rlst x 1 push .out a cat .c a show .n a
fn rle s:str -> str
 ex rle "aaab" = "a3b1"
 ex rle "" = ""
 let a fold @rlstep rlst "" 0 empty str split s ""
 if == .n a 0 "" join push .out a cat .c a show .n a ""
```

### balanced
```aleph
fn bstep d:i64 x:str -> i64
 ex bstep 0 "(" = 1
 ex bstep 0 ")" = -1
 ex bstep -1 "(" = -1
 if == d -1 -1 if == x "(" + d 1 if == x ")" - d 1 d
fn balanced s:str -> bool
 ex balanced "(a(b))" = true
 ex balanced ")(" = false
 ex balanced "" = true
 == fold @bstep 0 split s "" 0
```

### mean
```aleph
fn tof n:i64 -> f64
 ex tof 0 = 0.0
 ex tof 12 = 12.0
 else float cat show n ".0" 0.0
fn mean xs:list f64 -> opt f64
 ex mean [1.0 2.0 3.0] = some 2.0
 ex mean empty f64 = none f64
 if == len xs 0 none f64 some / fold lam a x + a x 0.0 xs tof len xs
```

### parsekv
```aleph
type kv k:str v:i64
fn parsepair p:str -> res kv str
 ex parsepair "a=1" = ok kv "a" 1
 ex isok parsepair "a1" = false
 let parts split p "="
 let key else at parts 0 ""
 let num try int else at parts 1 ""
 if != len parts 2 err "no =" ok kv key num
fn addkv d:dict str i64 q:kv -> dict str i64
 ex get addkv dict str i64 kv "a" 1 "a" = some 1
 put d .k q .v q
fn parsekv s:str -> res dict str i64 str
 ex parsekv "a=1,b=2" = ok put put dict str i64 "a" 1 "b" 2
 ex isok parsekv "a1" = false
 ex parsekv "" = ok dict str i64
 if == count s 0 ok dict str i64 ok fold @addkv dict str i64 try mapr @parsepair split s ","
```

### rotate
```aleph
fn rotn xs:list i64 n:i64 -> list i64
 ex rotn [1 2 3 4] 1 = [2 3 4 1]
 cat slice xs n len xs slice xs 0 n
fn rotate xs:list i64 k:i64 -> list i64
 ex rotate [1 2 3 4] 1 = [2 3 4 1]
 ex rotate [1 2 3] 3 = [1 2 3]
 ex rotate empty i64 5 = empty i64
 if == len xs 0 xs rotn xs % k len xs
```

### caesar
```aleph
fn rotc c:str k:i64 -> str
 ex rotc "a" 1 = "b"
 ex rotc "z" 1 = "a"
 ex rotc "A" 1 = "A"
 let o else ord c 0
 if and >= o 97 <= o 122 else chr + 97 % + - o 97 k 26 c
fn caesar s:str k:i64 -> str
 ex caesar "abc" 1 = "bcd"
 ex caesar "xyz" 2 = "zab"
 join map lam c rotc c k split s "" ""
```

### nestsum
```aleph
fn nestsum xss:list list i64 -> i64
 ex nestsum [[1 2] [3]] = 6
 ex nestsum empty list i64 = 0
 sum map lam r sum r xss
```

### title
```aleph
fn capw w:str -> str
 ex capw "hELLO" = "Hello"
 ex capw "" = ""
 let cs split lower w ""
 cat upper else at cs 0 "" join slice cs 1 len cs ""
fn title s:str -> str
 ex title "hello world" = "Hello World"
 ex title "" = ""
 join map lam w capw w split s " " " "
```

### median
```aleph
fn tof n:i64 -> f64
 ex tof 0 = 0.0
 ex tof 12 = 12.0
 else float cat show n ".0" 0.0
fn ati xs:list i64 i:i64 -> i64
 ex ati [5 6] 1 = 6
 else at xs i 0
fn medeven xs:list i64 h:i64 -> f64
 ex medeven [1 2 3 4] 2 = 2.5
 / tof + ati xs - h 1 ati xs h 2.0
fn median xs:list i64 -> f64
 ex median [3 1 2] = 2.0
 ex median [4 1 2 3] = 2.5
 let s sort xs
 let h / len s 2
 if == % len s 2 1 tof ati s h medeven s h
```

### merge
```aleph
fn hd xs:list i64 -> i64
 ex hd [5 6] = 5
 else at xs 0 0
fn tl xs:list i64 -> list i64
 ex tl [5 6] = [6]
 slice xs 1 len xs
fn merge a:list i64 b:list i64 -> list i64
 ex merge [1 3 5] [2 4] = [1 2 3 4 5]
 ex merge empty i64 [1] = [1]
 if == len a 0 b if == len b 0 a if <= hd a hd b push merge tl a b hd a push merge a tl b hd b
```

### dedupe
```aleph
fn dedx acc:list str x:str -> list str
 ex dedx ["b"] "b" = ["b"]
 ex dedx ["b"] "a" = ["b" "a"]
 if has index acc x acc push acc x
fn dedupe xs:list str -> list str
 ex dedupe ["b" "a" "b"] = ["b" "a"]
 ex dedupe empty str = empty str
 fold @dedx empty str xs
```

### digitsum
```aleph
fn dsum m:i64 acc:i64 -> i64
 ex dsum 123 0 = 6
 ex dsum 0 0 = 0
 if == m 0 acc dsum / m 10 + acc % m 10
fn digitsum n:i64 -> i64
 ex digitsum 123 = 6
 ex digitsum 0 = 0
 ex digitsum 45 = 9
 dsum abs n 0
```

### profit
```aleph
type st lo:i64 best:i64
fn hd xs:list i64 -> i64
 ex hd [5 6] = 5
 else at xs 0 0
fn tl xs:list i64 -> list i64
 ex tl [5 6] = [6]
 slice xs 1 len xs
fn pstep a:st p:i64 -> st
 ex .best pstep st 10 0 15 = 5
 ex .best pstep st 10 0 5 = 0
 st min .lo a p max .best a - p .lo a
fn profit prices:list i64 -> i64
 ex profit [7 1 5 3 6 4] = 5
 ex profit [7 6 4 3 1] = 0
 ex profit empty i64 = 0
 if == len prices 0 0 .best fold @pstep st hd prices 0 tl prices
```

### transpose
```aleph
fn ati xs:list i64 i:i64 -> i64
 ex ati [5 6] 1 = 6
 else at xs i 0
fn hdr m:list list i64 -> list i64
 ex hdr [[1 2] [3 4]] = [1 2]
 else at m 0 empty i64
fn colj m:list list i64 j:i64 -> list i64
 ex colj [[1 2 3] [4 5 6]] 0 = [1 4]
 map lam r ati r j m
fn transpose m:list list i64 -> list list i64
 ex transpose [[1 2 3] [4 5 6]] = [[1 4] [2 5] [3 6]]
 ex transpose [[1]] = [[1]]
 ex transpose empty list i64 = empty list i64
 if == len m 0 empty list i64 map lam j colj m j range 0 len hdr m
```

### binary
```aleph
fn binary n:i64 -> str
 ex binary 5 = "101"
 ex binary 0 = "0"
 ex binary 1 = "1"
 if < n 2 show n cat binary / n 2 show % n 2
```

### anagram
```aleph
fn anagram a:str b:str -> bool
 ex anagram "listen" "silent" = true
 ex anagram "ab" "abb" = false
 == join sort split a "" "" join sort split b "" ""
```

### wordlen
```aleph
fn maxlen acc:i64 w:str -> i64
 ex maxlen 2 "abc" = 3
 ex maxlen 5 "ab" = 5
 max acc count w
fn wordlen s:str -> i64
 ex wordlen "a bcd ef" = 3
 ex wordlen "" = 0
 fold @maxlen 0 split s " "
```

### grade
```aleph
fn letter n:i64 -> str
 ex letter 95 = "A"
 ex letter 85 = "B"
 ex letter 75 = "C"
 ex letter 10 = "F"
 if >= n 90 "A" if >= n 80 "B" if >= n 70 "C" "F"
fn grade scores:list i64 -> list str
 ex grade [95 85 75 10] = ["A" "B" "C" "F"]
 ex grade empty i64 = empty str
 map @letter scores
```
