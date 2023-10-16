references:
 - [Jason Morton halo2 codes](https://github.com/jasonmorton/halo2-examples/blob/master/src/fibonacci/example1.rs)
 - [ZCash halo2 books](https://zcash.github.io/halo2/user/simple-example.html#define-a-chip-implementation)
 - [trapdoor-tech halo2 book](https://trapdoor-tech.github.io/halo2-book-chinese/user/simple-example.html)
 - [icemelon/HaiCheng Shen](https://github.com/icemelon/halo2-examples/blob/master/src/fibonacci/example3.rs)
 - [0xPARC halo2](https://learn.0xparc.org/)

# Halo2 Examples

This repo includes a few simple examples to illustrate how to write circuit in Halo2.

## Instruction

Compile the repo

```
cargo build
```

Run examples
```
cargo test -- --nocapture test_example1
cargo test -- --nocapture test_example2
cargo test -- --nocapture test_example3
```

Plot the circuit layout
```
cargo test --all-features -- --nocapture plot
cargo test --all-features -- --nocapture print

cargo test --all-features -- --nocapture plot_fibo1
cargo test --all-features -- --nocapture plot_fibo2
cargo test --all-features -- --nocapture plot_fibo3

cargo test --release --all-features print_range_check_1
cargo test --release --all-features print_range_check_2
cargo test --release --all-features print_range_check_3

# decompose
cargo test --release --all-features print_decompose_range_check_1
```


# halo2-learn
