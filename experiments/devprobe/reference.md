### vowels
```aleph
fn vowels s:str -> i64
 ex vowels "Hello" = 2
 ex vowels "" = 0
 ex vowels "AEIOUxyz" = 5
 ex vowels "rhythm" = 0
 len filter lam c contains "aeiou" c split lower s ""
```

### second
```aleph
fn second xs:list i64 -> opt i64
 ex second [3 1 4 4] = some 3
 ex second [7] = none i64
 ex second [5 5 5] = none i64
 ex second [-1 -2 -3] = some -2
 at rev uniq xs 1
```

### rle
```aleph
fn rle s:str -> str
 ex rle "aaab" = "a3b1"
 ex rle "" = ""
 ex rle "abc" = "a1b1c1"
 ex rle "zzzzzzzzzzzz" = "z12"
 join map lam r cat else at r 0 "" show len r runs split s "" ""
fn runs cs:list str -> list list str
 ex runs ["a" "a" "b"] = [["a" "a"] ["b"]]
 fold lam acc c if == c else at else at rev acc 0 empty str 0 "" push slice acc 0 - len acc 1 push else at rev acc 0 empty str c push acc [c] [] cs
```

### balanced
```aleph
fn balanced s:str -> bool
 ex balanced "(a(b))" = true
 ex balanced ")(" = false
 ex balanced "" = true
 ex balanced "(()" = false
 ex balanced "x)y(" = false
 == 0 fold lam d c if < d 0 d if == c "(" + d 1 if == c ")" - d 1 d 0 split s ""
```

### mean
```aleph
fn mean xs:list f64 -> opt f64
 ex mean [1.0 2.0 3.0] = some 2.0
 ex mean empty f64 = none f64
 ex mean [5.0] = some 5.0
 ex mean [1.0 2.0] = some 1.5
 if == len xs 0 none f64 some / fold lam a x + a x 0.0 xs fold lam a x + a 1.0 0.0 xs
```

### parsekv
```aleph
fn parsekv s:str -> res dict str i64 str
 ex parsekv "a=1,b=2" = ok put put dict str i64 "a" 1 "b" 2
 ex isok parsekv "a1" = false
 ex parsekv "x=-5" = ok put dict str i64 "x" -5
 ex isok parsekv "a=x" = false
 let pairs try mapr @pair split s ","
 ok fold lam d p put d else at p 0 "" else int else at p 1 "" 0 dict str i64 pairs
fn pair p:str -> res list str str
 ex pair "a=1" = ok ["a" "1"]
 ex isok pair "a" = false
 let kv split p "="
 if != len kv 2 err "no =" if isok int else at kv 1 "" ok kv err "not an integer"
```

### rotate
```aleph
fn rotate xs:list i64 k:i64 -> list i64
 ex rotate [1 2 3 4] 1 = [2 3 4 1]
 ex rotate [1 2 3] 3 = [1 2 3]
 ex rotate [1 2 3] 5 = [3 1 2]
 ex rotate empty i64 2 = empty i64
 let n len xs
 if == n 0 xs map lam i else at xs % + i k n 0 range 0 n
```

### caesar
```aleph
fn caesar s:str k:i64 -> str
 ex caesar "abc" 1 = "bcd"
 ex caesar "xyz" 2 = "zab"
 ex caesar "Hi there!" 13 = "Hv gurer!"
 ex caesar "" 5 = ""
 join map lam c if contains "abcdefghijklmnopqrstuvwxyz" c else chr + 97 % + - else ord c 0 97 k 26 c c split s "" ""
```

### nestsum
```aleph
fn nestsum xss:list list i64 -> i64
 ex nestsum [[1 2] [3]] = 6
 ex nestsum empty list i64 = 0
 ex nestsum [[-1] [] [4 4]] = 7
 sum map @sum xss
```

### title
```aleph
fn title s:str -> str
 ex title "hello world" = "Hello World"
 ex title "" = ""
 ex title "aBC dEF" = "Abc Def"
 ex title "x" = "X"
 join map lam w cat upper join slice split w "" 0 1 "" lower join slice split w "" 1 count w "" split s " " " "
```

### median
```aleph
fn median xs:list i64 -> f64
 ex median [3 1 2] = 2.0
 ex median [4 1 2 3] = 2.5
 ex median [7] = 7.0
 ex median [1 1 2 100] = 1.5
 let ys sort xs
 let n len ys
 let a else at ys / - n 1 2 0
 let b else at ys / n 2 0
 / else float cat show + a b ".0" 0.0 2.0
```

### merge
```aleph
fn merge a:list i64 b:list i64 -> list i64
 ex merge [1 3 5] [2 4] = [1 2 3 4 5]
 ex merge empty i64 [1] = [1]
 ex merge [1 1] [1] = [1 1 1]
 ex merge [5] empty i64 = [5]
 sort fold @push a b
```

### dedupe
```aleph
fn dedupe xs:list str -> list str
 ex dedupe ["b" "a" "b"] = ["b" "a"]
 ex dedupe empty str = empty str
 ex dedupe ["x" "x" "x"] = ["x"]
 ex dedupe ["c" "b" "a" "b" "c"] = ["c" "b" "a"]
 fold lam acc x if has index acc x acc push acc x empty str xs
```

### digitsum
```aleph
fn digitsum n:i64 -> i64
 ex digitsum 123 = 6
 ex digitsum 0 = 0
 ex digitsum -45 = 9
 ex digitsum 1000000 = 1
 sum map lam c else int c 0 split show abs n ""
```

### profit
```aleph
fn profit prices:list i64 -> i64
 ex profit [7 1 5 3 6 4] = 5
 ex profit [7 6 4 3 1] = 0
 ex profit empty i64 = 0
 ex profit [2 4 1 10] = 9
 fold @max 0 map lam i - else at prices i 0 fold @min else at prices 0 0 slice prices 0 + i 1 range 0 len prices
```

### transpose
```aleph
fn transpose m:list list i64 -> list list i64
 ex transpose [[1 2 3] [4 5 6]] = [[1 4] [2 5] [3 6]]
 ex transpose [[1]] = [[1]]
 ex transpose [[1 2]] = [[1] [2]]
 ex transpose [[1] [2]] = [[1 2]]
 map lam j map lam row else at row j 0 m range 0 len else at m 0 empty i64
```

### binary
```aleph
fn binary n:i64 -> str
 ex binary 5 = "101"
 ex binary 0 = "0"
 ex binary 1 = "1"
 ex binary 255 = "11111111"
 if < n 2 show n cat binary / n 2 show % n 2
```

### anagram
```aleph
fn anagram a:str b:str -> bool
 ex anagram "listen" "silent" = true
 ex anagram "ab" "abb" = false
 ex anagram "" "" = true
 ex anagram "abc" "abd" = false
 == join sort split a "" "" join sort split b "" ""
```

### wordlen
```aleph
fn wordlen s:str -> i64
 ex wordlen "a bcd ef" = 3
 ex wordlen "" = 0
 ex wordlen "  x  " = 1
 ex wordlen "hello" = 5
 fold @max 0 map @count split s " "
```

### grade
```aleph
fn grade scores:list i64 -> list str
 ex grade [95 85 75 10] = ["A" "B" "C" "F"]
 ex grade empty i64 = empty str
 ex grade [90 89 70 69] = ["A" "B" "C" "F"]
 map lam x if >= x 90 "A" if >= x 80 "B" if >= x 70 "C" "F" scores
```
