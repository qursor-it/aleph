### revwords
```aleph
fn revwords s:str -> str
 ex revwords "a b  c" = "c b a"
 ex revwords "" = ""
 join rev fields s " "
```
### longest
```aleph
fn longest xs:list str -> opt str
 ex longest ["ab" "abc" "x"] = some "abc"
 ex longest empty str = none str
 fold lam acc x if not has acc some x if > count x count else acc "" some x acc none str xs
```
### ratio
```aleph
fn ratio xs:list i64 -> f64
 ex ratio [1 -1 2 0] = 0.5
 ex ratio empty i64 = 0.0
 if == len xs 0 0.0 / tof fold lam a x + a if > x 0 1 0 0 xs tof len xs
```
### flatten
```aleph
fn flatten xss:list list i64 -> list i64
 ex flatten [[1 2] [] [3]] = [1 2 3]
 ex flatten empty list i64 = empty i64
 fold lam acc ys cat acc ys empty i64 xss
```
### sortpeople
```aleph
type person name:str age:i64
fn sortpeople ps:list person -> list str
 ex sortpeople [person "a" 30 person "b" 20] = ["b" "a"]
 ex sortpeople empty person = empty str
 map lam q .name q sortby lam p .age p ps
```
### inventory
```aleph
fn applyup d:dict str i64 u:str -> res dict str i64 str
 ex applyup dict str i64 "a:2" = ok put dict str i64 "a" 2
 let fs split u ":"
 if != len fs 2 err "bad"
 let nm else at fs 0 ""
 if == nm "" err "bad"
 let ds else at fs 1 ""
 let r int ds
 if not isok r err "bad"
 let n else r 0
 ok put d nm + else get d nm 0 n
fn inventory d:dict str i64 ups:list str -> res dict str i64 str
 ex inventory dict str i64 ["a:2" "a:-1"] = ok put dict str i64 "a" 1
 ex isok inventory dict str i64 ["a"] = false
 if == len ups 0 ok d
 let u else at ups 0 ""
 let r applyup d u
 match r ok d2 inventory d2 slice ups 1 len ups err e2 err "bad"
```
### dist
```aleph
fn dist x1:i64 y1:i64 x2:i64 y2:i64 -> f64
 ex dist 0 0 3 4 = 5.0
 ex dist 1 1 1 1 = 0.0
 let dx tof - x2 x1
 let dy tof - y2 y1
 sqrt + * dx dx * dy dy
```
### windows
```aleph
fn windows xs:list i64 k:i64 -> list i64
 ex windows [1 2 3 4] 2 = [3 5 7]
 ex windows [1] 2 = empty i64
 let n + 1 - len xs k
 if <= n 0 empty i64
 map lam i sum slice xs i + i k range 0 n
```
### vowelfirst
```aleph
fn vowelfirst s:str -> list str
 ex vowelfirst "an apple by us" = ["an" "apple" "us"]
 ex vowelfirst "" = empty str
 filter lam w if == w "" false contains "aeiou" else at split w "" 0 "" fields s
```
### interleave
```aleph
fn interleave a:list i64 b:list i64 -> list i64
 ex interleave [1 3] [2 4 6] = [1 2 3 4 6]
 ex interleave empty i64 [1] = [1]
 if == len a 0 b
 if == len b 0 a
 cat [else at a 0 0] cat [else at b 0 0] interleave slice a 1 len a slice b 1 len b
```
### countchars
```aleph
fn countchars s:str -> dict str i64
 ex countchars "aba" = put put dict str i64 "a" 2 "b" 1
 ex countchars "" = dict str i64
 fold lam d c put d c + 1 else get d c 0 dict str i64 split s ""
```
### parsedate
```aleph
fn parsedate s:str -> res i64 str
 ex parsedate "2024-01-01" = ok 1
 ex parsedate "2024-03-01" = ok 60
 let fs split s "-"
 if != len fs 3 err "bad"
 let ys else at fs 0 ""
 let ms else at fs 1 ""
 let ds else at fs 2 ""
 if or != count ys 4 or != count ms 2 != count ds 2 err "bad"
 if not isok int ys err "bad"
 if not isok int ms err "bad"
 if not isok int ds err "bad"
 let mo else int ms 0
 let dy else int ds 0
 if or < mo 1 > mo 12 err "bad"
 let mds [31 28 31 30 31 30 31 31 30 31 30 31]
 if or < dy 1 > dy else at mds - mo 1 0 err "bad"
 ok + sum slice mds 0 - mo 1 dy
```
### clampall
```aleph
fn clampall xs:list i64 lo:i64 hi:i64 -> list i64
 ex clampall [-5 3 12] 0 10 = [0 3 10]
 ex clampall empty i64 0 1 = empty i64
 map lam x max lo min hi x xs
```
### isprime
```aleph
fn isprime n:i64 -> bool
 ex isprime 7 = true
 ex isprime 1 = false
 if < n 2 false
 == 0 len filter lam d == 0 % n d range 2 n
```
### tally
```aleph
fn tally votes:list str -> list str
 ex tally ["b" "a" "b"] = ["b" "a"]
 ex tally empty str = empty str
 let cs fold lam d v put d v + 1 else get d v 0 dict str i64 votes
 sortby lam n neg else get cs n 0 sort keys cs
```
### stddev
```aleph
fn stddev xs:list f64 -> opt f64
 ex stddev [2.0 4.0 4.0 4.0 5.0 5.0 7.0 9.0] = some 2.0
 ex stddev empty f64 = none f64
 if == len xs 0 none f64
 let n tof len xs
 let m / fold lam a x + a x 0.0 xs n
 some sqrt / fold lam b y + b * - y m - y m 0.0 xs n
```
### snake
```aleph
fn snake s:str -> str
 ex snake "helloWorld" = "hello_world"
 ex snake "" = ""
 join map lam c if and == upper c c != lower c c cat "_" lower c c split s "" ""
```
### depth
```aleph
type tree | leaf | node l:tree r:tree
fn depth t:tree -> i64
 ex depth leaf = 0
 ex depth node leaf node leaf leaf = 2
 match t leaf 0 node l r + 1 max depth l depth r
```
### validip
```aleph
fn validfield f:str -> bool
 ex validfield "0" = true
 ex validfield "01" = false
 ex validfield "255" = true
 ex validfield "256" = false
 if == f "" false
 if not isok int f false
 if starts f "-" false
 if and > count f 1 == else at split f "" 0 "" "0" false
 let n else int f -1
 and >= n 0 <= n 255
fn validip s:str -> bool
 ex validip "192.168.0.1" = true
 ex validip "256.1.1.1" = false
 if or starts s "." ends s "." false
 if contains s ".." false
 let fs split s "."
 if != len fs 4 false
 and validfield else at fs 0 "" and validfield else at fs 1 "" and validfield else at fs 2 "" validfield else at fs 3 ""
```
### pascal
```aleph
fn pascal n:i64 -> list i64
 ex pascal 0 = [1]
 ex pascal 3 = [1 3 3 1]
 if == n 0 [1]
 let p pascal - n 1
 cat [1] cat map2 lam a b + a b slice p 0 - len p 1 slice p 1 len p [1]
```
