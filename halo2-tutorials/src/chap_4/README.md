# circuit_1.rs

single columns lookup


Circuit design:

```rust
| adv   | q_lookup|  table  |
|-------|---------|---------|
| a[0]  |    1    |    0    |
| a[1]  |    1    |    1    |
|  ...  |   ...   |   ...   |
| a[N]  |    1    |   N-1   |
|       |    0    |    N    |
|       |   ...   |   ...   |
|       |    0    |  RANGE  |
```

Test:
```rust
$ cargo test -- --nocapture test_1_col_rangecheck_lookup
$ cargo test --features dev-graph -- --nocapture plot_1_col_rangecheck_lookup
```

# circuit_2.rs

multi-cols lookup.

Circuit design:

```rust
------------------
| private inputs |
------------------
| value |  bit   | q_lookup  | table_n_bits| table_value |
----------------------------------------------------------
|  v_0  |   0    |    0      |      1      |      0      |
|  v_1  |   1    |    1      |      1      |      1      |
|  ...  |  ...   |    1      |      2      |      2      |
|  ...  |  ...   |    1      |      2      |      3      |
|  ...  |  ...   |    1      |      3      |      4      |
|  ...  |  ...   |    1      |      3      |      5      |
|  ...  |  ...   |    1      |      3      |      6      |
|  ...  |  ...   |   ...     |      3      |      7      |
|  ...  |  ...   |   ...     |      4      |      8      |
|  ...  |  ...   |   ...     |     ...     |     ...     |
```

Test:
```rust
cargo test -- --nocapture test_multi_cols_rangecheck_lookup
cargo test --features dev-graph -- --nocapture plot_multi_cols_rangecheck_lookup
```


# circuit_3.rs

`circuit_3` can perform query operations on **different rows** based on the input vectors a and b to prove the relationship between them.

Circuit design:
```bash
| advice_a| advice_b| q_lookup| table_1 | table_2 |
|---------|---------|---------|---------|---------|
|    0    |    0    |    1    |    0    |    0    |
|    1    |    0    |    1    |    1    |    1    |
|    2    |    1    |    1    |    2    |    2    |
|    3    |    2    |    1    |    3    |    3    |
|         |    3    |    0    |    4    |    4    |
|         |         |   ...   |   ...   |   ...   |
|         |         |    0    |  RANGE  |  RANGE  |
```

Both need to be satisfied :
 - cur_a ∈ t1
 - next_b ∈ t2

```bash
$ cargo test -- --nocapture test_lookup_on_different_rows
$ cargo test --features dev-graph -- --nocapture plot_lookup_on_different_rows
```