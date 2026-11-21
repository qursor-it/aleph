### vowels
```aleph
fn isvowel c:str -> bool
 ex isvowel "a" = true
 ex isvowel "z" = false
 or or or or == lower c "a" == lower c "e" == lower c "i" == lower c "o" == lower c "u"
fn vowels s:str -> i64
 ex vowels "Hello" = 2
 ex vowels "" = 0
 len filter @isvowel split s ""
```

### second
```aleph
fn second xs:list i64 -> opt i64
 ex second [3 1 4 4] = some 3
 ex second [7] = none i64
 let u sort uniq xs
 if < len u 2 none i64
 at u - len u 2
```

### rle
```aleph
fn rleclose cur:str n:i64 acc:str -> str
 ex rleclose "a" 2 "" = "a2"
 ex rleclose "" 0 "" = ""
 if == n 0 acc
 cat acc cat cur show n
fn rlego cs:list str cur:str n:i64 acc:str -> str
 ex rlego empty str "" 0 "" = ""
 ex rlego ["a" "a" "b"] "" 0 "" = "a2b1"
 if == len cs 0 rleclose cur n acc
 let c else at cs 0 ""
 let rest slice cs 1 len cs
 if == n 0 rlego rest c 1 acc
 if == c cur rlego rest cur + n 1 acc
 rlego rest c 1 cat acc cat cur show n
fn rle s:str -> str
 ex rle "aaab" = "a3b1"
 ex rle "" = ""
 rlego split s "" "" 0 ""
```

### balanced
```aleph
fn balgo cs:list str d:i64 -> bool
 ex balgo ["(" ")"] 0 = true
 ex balgo [")"] 0 = false
 if == len cs 0 == d 0
 let c else at cs 0 ""
 let d2 if == c "(" + d 1 if == c ")" - d 1 d
 if < d2 0 false
 balgo slice cs 1 len cs d2
fn balanced s:str -> bool
 ex balanced "(a(b))" = true
 ex balanced ")(" = false
 balgo split s "" 0
```

### mean
```aleph
fn mean xs:list f64 -> opt f64
 ex mean [1.0 2.0 3.0] = some 2.0
 ex mean empty f64 = none f64
 if == len xs 0 none f64
 some / fold lam a x + a x 0.0 xs tof len xs
```

### parsekv
```aleph
fn parse1 p:str -> res list str str
 ex parse1 "a=1" = ok ["a" "1"]
 ex isok parse1 "a1" = false
 let parts split p "="
 if != len parts 2 err "no ="
 ok parts
fn kvgo ps:list list str d:dict str i64 -> res dict str i64 str
 ex isok kvgo [["a" "1"]] dict str i64 = true
 ex isok kvgo [["a" "x"]] dict str i64 = false
 if == len ps 0 ok d
 let p else at ps 0 empty str
 let k else at p 0 ""
 let v else at p 1 ""
 let n try int v
 kvgo slice ps 1 len ps put d k n
fn parsekv s:str -> res dict str i64 str
 ex parsekv "a=1,b=2" = ok put put dict str i64 "a" 1 "b" 2
 ex isok parsekv "a1" = false
 let pairs try mapr @parse1 split s ","
 kvgo pairs dict str i64
```

### rotate
```aleph
fn rotate xs:list i64 k:i64 -> list i64
 ex rotate [1 2 3 4] 1 = [2 3 4 1]
 ex rotate [1 2 3] 3 = [1 2 3]
 if == len xs 0 empty i64
 let n % k len xs
 cat slice xs n len xs slice xs 0 n
```

### caesar
```aleph
fn shift1 c:str k:i64 -> str
 ex shift1 "a" 1 = "b"
 ex shift1 "z" 1 = "a"
 ex shift1 "A" 1 = "A"
 let o else ord c 0
 if or < o 97 > o 122 c
 let d + - o 97 k
 let e + % d 26 97
 else chr e c
fn caesar s:str k:i64 -> str
 ex caesar "abc" 1 = "bcd"
 ex caesar "xyz" 2 = "zab"
 join map lam c shift1 c k split s "" ""
```

### nestsum
```aleph
fn nestsum xss:list list i64 -> i64
 ex nestsum [[1 2] [3]] = 6
 ex nestsum empty list i64 = 0
 sum map @sum xss
```

### title
```aleph
fn capw w:str -> str
 ex capw "hello" = "Hello"
 ex capw "" = ""
 let cs split w ""
 if == len cs 0 w
 let hd else at cs 0 ""
 let tl slice cs 1 len cs
 cat upper hd lower join tl ""
fn title s:str -> str
 ex title "hello world" = "Hello World"
 ex title "" = ""
 join map @capw split s " " " "
```

### median
```aleph
fn median xs:list i64 -> f64
 ex median [3 1 2] = 2.0
 ex median [4 1 2 3] = 2.5
 let s sort xs
 let n len s
 if == % n 2 1 tof else at s / n 2 0
 let a tof else at s - / n 2 1 0
 let b tof else at s / n 2 0
 / + a b 2.0
```

### merge
```aleph
fn merge a:list i64 b:list i64 -> list i64
 ex merge [1 3 5] [2 4] = [1 2 3 4 5]
 ex merge empty i64 [1] = [1]
 sort cat a b
```

### dedupe
```aleph
fn addone acc:list str x:str -> list str
 ex addone ["a"] "b" = ["a" "b"]
 ex addone ["a"] "a" = ["a"]
 if has index acc x acc
 push acc x
fn dedupe xs:list str -> list str
 ex dedupe ["b" "a" "b"] = ["b" "a"]
 ex dedupe empty str = empty str
 fold @addone empty str xs
```

### digitsum
```aleph
fn dgo n:i64 -> i64
 ex dgo 123 = 6
 ex dgo 0 = 0
 if == n 0 0
 + % n 10 dgo / n 10
fn digitsum n:i64 -> i64
 ex digitsum 123 = 6
 ex digitsum 0 = 0
 dgo abs n
```

### profit
```aleph
fn pgo ps:list i64 mn:i64 best:i64 -> i64
 ex pgo empty i64 0 0 = 0
 ex pgo [5 2 8] 5 0 = 6
 if == len ps 0 best
 let p else at ps 0 0
 let d - p mn
 let b if > d best d best
 pgo slice ps 1 len ps min mn p b
fn profit prices:list i64 -> i64
 ex profit [7 1 5 3 6 4] = 5
 ex profit [7 6 4 3 1] = 0
 if == len prices 0 0
 let p0 else at prices 0 0
 pgo slice prices 1 len prices p0 0
```

### transpose
```aleph
fn col m:list list i64 j:i64 -> list i64
 ex col [[1 2] [3 4]] 0 = [1 3]
 ex col [[1 2] [3 4]] 1 = [2 4]
 map lam r else at r j 0 m
fn transpose m:list list i64 -> list list i64
 ex transpose [[1 2 3] [4 5 6]] = [[1 4] [2 5] [3 6]]
 ex transpose [[1]] = [[1]]
 if == len m 0 empty list i64
 let r0 else at m 0 empty i64
 map lam j col m j range 0 len r0
```

### binary
```aleph
fn bgo n:i64 -> str
 ex bgo 0 = ""
 ex bgo 5 = "101"
 if == n 0 ""
 cat bgo / n 2 show % n 2
fn binary n:i64 -> str
 ex binary 5 = "101"
 ex binary 0 = "0"
 if == n 0 "0"
 bgo n
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
fn wordlen s:str -> i64
 ex wordlen "a bcd ef" = 3
 ex wordlen "" = 0
 fold lam a x max a x 0 map lam w count w split s " "
```

### grade
```aleph
fn gr s:i64 -> str
 ex gr 95 = "A"
 ex gr 10 = "F"
 if >= s 90 "A"
 if >= s 80 "B"
 if >= s 70 "C"
 "F"
fn grade scores:list i64 -> list str
 ex grade [95 85 75 10] = ["A" "B" "C" "F"]
 ex grade empty i64 = empty str
 map @gr scores
```
