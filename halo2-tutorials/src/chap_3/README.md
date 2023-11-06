# circuit_1.rs

Circuit design:
```bash
|  ins  |   a0    | seletor|
|-------|---------|--------|
|   a   | f(0)=a  |   1    |
|   b   | f(1)=b  |   1    |  
|  out  | f(2)    |   1    |  
|       | f(3)    |   1    |   
|       |  ...    |        |
|       | f(n-2)  |   1    |
|       | f(n-1)  |   0    |   
|       | f(n)=out|   0    |   
```

# circuit_2.rs

Circuit design:
```bash
| ins   | a0     |   a1   | seletor|
|-------|------- |------- |------- |
|   a   | f(0)=a | f(1)=b |    1   |
|   b   | f(2)=b | f(3)   |    1   |  
|  out  | f(4)   | f(5)   |    1   |   
|          ...            |        |
|       | f(2n/2) |f(2n/2+1)|   1  |
///
out = n % 2 == 0 ? f(2n/2) : f(2n/2 + 1)
```
