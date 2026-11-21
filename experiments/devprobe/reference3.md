### revwords
```aleph
fn revwords s:str -> str
 ex revwords "a b  c" = "c b a"
 join rev fields s " "
```

### longest
```aleph
fn longest xs:list str -> opt str
 ex longest ["ab" "abc" "x"] = some "abc"
 at sortby lam s neg count s xs 0
```

### ratio
```aleph
fn ratio xs:list i64 -> f64
 ex ratio [1 -1 2 0] = 0.5
 if == 0 len xs 0.0
 / tof len filter lam x > x 0 xs tof len xs
```

### flatten
```aleph
fn flatten xss:list list i64 -> list i64
 ex flatten [[1 2] [] [3]] = [1 2 3]
 fold @cat empty i64 xss
```

### sortpeople
```aleph
type person name:str age:i64
fn sortpeople ps:list person -> list str
 ex sortpeople [person "a" 30 person "b" 20] = ["b" "a"]
 map lam p .name p sortby lam p .age p ps
```

### inventory
```aleph
fn update d:dict str i64 u:str -> res dict str i64 str
 ex update dict str i64 "a:2" = ok put dict str i64 "a" 2
 let kv split u ":"
 if != 2 len kv err "bad"
 let k else at kv 0 ""
 let n else int else at kv 1 "" 0
 if not isok int else at kv 1 "" err "bad"
 ok put d k + n else get d k 0
fn inventory d:dict str i64 ups:list str -> res dict str i64 str
 ex inventory dict str i64 ["a:2" "a:-1"] = ok put dict str i64 "a" 1
 if == 0 len ups ok d
 let first try update d else at ups 0 ""
 inventory first slice ups 1 len ups
```

### dist
```aleph
fn dist x1:i64 y1:i64 x2:i64 y2:i64 -> f64
 ex dist 0 0 3 4 = 5.0
 let dx tof - x2 x1
 let dy tof - y2 y1
 sqrt + * dx dx * dy dy
```

### windows
```aleph
fn windows xs:list i64 k:i64 -> list i64
 ex windows [1 2 3 4] 2 = [3 5 7]
 map lam i sum slice xs i + i k range 0 + 1 - len xs k
```

### vowelfirst
```aleph
fn vowelfirst s:str -> list str
 ex vowelfirst "an apple by us" = ["an" "apple" "us"]
 filter lam w contains "aeiou" else at split w "" 0 "-" fields s
```

### interleave
```aleph
fn interleave a:list i64 b:list i64 -> list i64
 ex interleave [1 3] [2 4 6] = [1 2 3 4 6]
 if == 0 len a b
 if == 0 len b a
 cat [else at a 0 0 else at b 0 0] interleave slice a 1 len a slice b 1 len b
```

### countchars
```aleph
fn countchars s:str -> dict str i64
 ex countchars "aba" = put put dict str i64 "a" 2 "b" 1
 fold lam d c put d c + 1 else get d c 0 dict str i64 split s ""
```

### parsedate
```aleph
fn parsedate s:str -> res i64 str
 ex parsedate "2024-03-01" = ok 60
 let ps mapr @int split s "-"
 if not isok ps err "bad"
 let p else ps empty i64
 if != 3 len p err "bad"
 let m else at p 1 0
 ok + sum slice [31 28 31 30 31 30 31 31 30 31 30 31] 0 - m 1 else at p 2 0
```

### clampall
```aleph
fn clampall xs:list i64 lo:i64 hi:i64 -> list i64
 ex clampall [-5 3 12] 0 10 = [0 3 10]
 map lam x min hi max lo x xs
```

### isprime
```aleph
fn isprime n:i64 -> bool
 ex isprime 7 = true
 if < n 2 false
 == 0 len filter lam d == 0 % n d filter lam d <= * d d n range 2 n
```

### tally
```aleph
fn tally votes:list str -> list str
 ex tally ["b" "a" "b"] = ["b" "a"]
 let d fold lam m v put m v + 1 else get m v 0 dict str i64 votes
 sortby lam k neg else get d k 0 keys d
```

### stddev
```aleph
fn stddev xs:list f64 -> opt f64
 ex stddev empty f64 = none f64
 if == 0 len xs none f64
 let n tof len xs
 let m / fold lam a x + a x 0.0 xs n
 some sqrt / fold lam a x + a * - x m - x m 0.0 xs n
```

### snake
```aleph
fn snake s:str -> str
 ex snake "helloWorld" = "hello_world"
 join map lam c if and != c lower c contains "ABCDEFGHIJKLMNOPQRSTUVWXYZ" c cat "_" lower c c split s "" ""
```

### depth
```aleph
type tree | leaf | node l:tree r:tree
fn depth t:tree -> i64
 ex depth leaf = 0
 match t leaf 0 node l r + 1 max depth l depth r
```

### validip
```aleph
fn octet p:str -> bool
 ex octet "255" = true
 ex octet "01" = false
 let n else int p -1
 and and isok int p == p show n and >= n 0 <= n 255
fn validip s:str -> bool
 ex validip "192.168.0.1" = true
 let ps split s "."
 and == 4 len ps == 4 len filter @octet ps
```

### pascal
```aleph
fn pascal n:i64 -> list i64
 ex pascal 3 = [1 3 3 1]
 fold lam row i map2 lam a b + a b cat [0] row cat row [0] [1] range 0 n
```
