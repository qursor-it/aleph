### merge
```aleph
fn mergego a:list i64 b:list i64 i:i64 j:i64 -> list i64
 ex mergego [1 3] [2] 0 0 = [1 2 3]
 ex mergego empty i64 empty i64 0 0 = empty i64
 if == i len a slice b j len b
 if == j len b slice a i len a
 let x else at a i 0
 let y else at b j 0
 if <= x y cat [x] mergego a b + i 1 j cat [y] mergego a b i + j 1
fn merge a:list i64 b:list i64 -> list i64
 ex merge [1 4] [2 3 5] = [1 2 3 4 5]
 ex merge empty i64 empty i64 = empty i64
 mergego a b 0 0
```

### transpose
```aleph
fn col m:list list i64 j:i64 i:i64 -> list i64
 ex col [[1 2] [3 4]] 0 0 = [1 3]
 ex col [[1 2] [3 4]] 1 0 = [2 4]
 if == i len m empty i64
 cat [else at else at m i empty i64 j 0] col m j + i 1
fn transposego m:list list i64 j:i64 cols:i64 -> list list i64
 ex transposego [[1 2] [3 4]] 0 2 = [[1 3] [2 4]]
 if == j cols empty list i64
 cat [col m j 0] transposego m + j 1 cols
fn transpose m:list list i64 -> list list i64
 ex transpose [[1 2 3] [4 5 6]] = [[1 4] [2 5] [3 6]]
 ex transpose empty list i64 = empty list i64
 if == len m 0 empty list i64
 transposego m 0 len else at m 0 empty i64
```
