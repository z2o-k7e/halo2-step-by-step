参考链接: https://learn.z2o-k7e.world/halo2/chap-0/index.html
1. selector 约束错误
2. region 分配

录制视频:
 - hey guys! welcome to the halo2-monster world!
 - In this world, there are so many bugs and underconstraints waiting for you , to fix it.
 - with the process of our lessons , there will be more and more practice you need face, try to follow the steps with us!


## Usage:

```bash
# 查看测试说明
$ cargo run

# 正式开始练习 (watch 模式: 每次 ctrl + S 保存都会自动运行 test 检查是否通过)
$ cargo run watch
```

```bash
wacth 运行时命令
- hint: 解题提示
- solution: 考察要点讲解
- clear: 清屏
```


### 对每个 exercise 的测试流程:
1. 写好错误区域 or fill-in-blank area
2. 写好错误原因 和 考查要点
3. 给出修改方法
4. 绘制 Circuit Layouter diagram
   1. 图片路径, 图片名称 (circuit_layouter_plots/plot_chap_1_exercise_1)
   2. 2 个测试函数名称修改


#### exercise_1

错误区域: fn configure(): line 128 ~ 131
错误原因: 在创建 custom gate "mul_gate" 时, 未指定 selector 作用
修改方法: 添加 selector

```rust
    /// let out = meta.query_advice(advice[0], Rotation::next());
    let s_mul = meta.query_selector(s_mul);
    vec![s_mul * (lhs * rhs - out)]
}
```

Test & plot it.
```bash
cargo test --features chap_1_exercise_1 -- --nocapture test_chap_1_exercise_1
cargo test --features chap_1_exercise_1,dev-graph -- --nocapture plot_chap_1_exercise_1
```


#### exercise_2

fill-in-blank area: `line 147:  let ab = ____ `
考察要点: 在 synthesize 如何向电路中填入 witness
修改方法: 添加 selector


```rust
  let ab = mul(&config,layouter.namespace(|| "a*b"), a, b)?;
```

Test & plot it.
```bash
cargo test --features chap_1_exercise_2 -- --nocapture test_chap_1_exercise_2
cargo test --features chap_1_exercise_2,dev-graph -- --nocapture plot_chap_1_exercise_2
```


#### exercise_3

错误区域: fn configure(): line 127 ~ 130
错误原因: 在创建 custom gate "mul_gate" 时, custom gate 的形状未覆盖 region assignment 形状
修改方法: 添加 selector
参考链接: https://learn.z2o-k7e.world/halo2/chap-0/index.html

修改方法: 按照 custom_gate 的创建方式去布局 region

```rust
     let lhs = meta.query_advice(advice[0], Rotation::cur());
     let rhs = meta.query_advice(advice[1], Rotation::cur());
     let out = meta.query_advice(advice[0], Rotation::next());
```

Test & plot it.
```bash
cargo test --features chap_1_exercise_3 -- --nocapture test_chap_1_exercise_3
cargo test --features chap_1_exercise_3,dev-graph -- --nocapture plot_chap_1_exercise_3
```

#### exercise_4

错误区域: fn configure(): line 150 ~ 154
错误原因: offset 多增加了一行, 导致 custom gate 无法覆盖 assign region
修改方法: 注释/删除掉 line 151 的 offset += 1;
参考链接: https://learn.z2o-k7e.world/halo2/chap-2/index.html

```rust
    // fill out
    // offset += 1;
    config.s_cub.enable(&mut region, offset)?;
    let value = e.0.value().copied() * e.0.value().copied() * e.0.value().copied();
    region.assign_advice(|| "out", config.advice[1], offset, || value).map(Number)
```

Test & plot it.
```bash
cargo test --features chap_2_exercise_4 -- --nocapture test_chap_2_exercise_4
cargo test --features chap_2_exercise_4,dev-graph -- --nocapture plot_chap_2_exercise_4
```



#### exercise_5


参考链接: https://learn.z2o-k7e.world/halo2/chap-2/index.html

参考答案:
```rust
    pub fn configure(meta: &mut ConstraintSystem<F>) -> SimpleConfig {
        let advice = [meta.advice_column(), meta.advice_column(), meta.advice_column()];
        let instance = meta.instance_column();
        let constant = meta.fixed_column();

        meta.enable_equality(instance);
        meta.enable_constant(constant);
        for c in &advice {
            meta.enable_equality(*c);
        }
        let s_cpx = meta.selector();

        meta.create_gate("complex_gate", |meta| {
            let l = meta.query_advice(advice[0], Rotation::cur());
            let r = meta.query_advice(advice[1], Rotation::cur());
            let c = meta.query_advice(advice[2], Rotation::cur());
            let out = meta.query_advice(advice[0], Rotation::next());

            let s_cpx = meta.query_selector(s_cpx);

            let e = (l.clone() * r.clone()) * (l * r ) * c.clone() + c;
            let e_cub = e.clone() * e.clone() * e.clone() ;
            Constraints::with_selector(s_cpx, vec![e_cub - out])
        });

        SimpleConfig {
            advice,
            instance,
            s_cpx
        }
    }

    pub fn assign( 
        &self,
        mut layouter: impl Layouter<F>,
        a: Value<F>,
        b: Value<F>,
        c: F
        ) -> Result<Number<F>, Error> {
            layouter.assign_region(
                || "load private & witness", 
            |mut region| {
                let mut offset = 0;
                let config = &self.config;
                config.s_cpx.enable(&mut region, offset)?;  // Attention the positon of s_cpx to offset.

                let a_cell = region.assign_advice(|| "private input a",  self.config.advice[0], offset, || a).map(Number)?;
                let b_cell = region.assign_advice(|| "private input b",  self.config.advice[1], offset, || b).map(Number)?;
                let c_cell = region.assign_advice_from_constant(|| "private input c",  self.config.advice[2], offset, c).map(Number)?;
                offset += 1;
                let e: Value<F> = 
                    (a_cell.0.value().copied() * b_cell.0.value().copied())   // a * b    = ab
                    * (a_cell.0.value().copied() * b_cell.0.value().copied()) // ab * ab  = absq
                    * c_cell.0.value().copied()                               // absq * c = d
                    + c_cell.0.value().copied();                              // d + c    = e
                let e_cub = e * e * e;                              // e_cub    = e^3
                region.assign_advice(|| "out", config.advice[0], offset, || e_cub).map(Number)
            })
    }

```

Test & plot it.
```bash
cargo test --features chap_2_exercise_5 -- --nocapture test_chap_2_exercise_5
cargo test --features chap_2_exercise_5,dev-graph -- --nocapture plot_chap_2_exercise_5
```