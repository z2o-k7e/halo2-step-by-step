use ff::{PrimeField, PrimeFieldBits};
use halo2_proofs::{circuit::*, plonk::*, poly::Rotation};
use std::marker::PhantomData;

mod table;
use table::RangeTableConfig;


#[derive(Debug, Clone)]
struct DecomposeConfig<
    F: PrimeField + PrimeFieldBits,
    const LOOKUP_NUM_BITS: usize,  // 10 
    const LOOKUP_RANGE: usize,     // 1024
> {
    // You'll need an advice column to witness your running sum;
    running_sum: Column<Advice>,
    // A selector to constrain the running sum;
    q_decompose: Selector,
    // A selector to handle the final partial chunk
    q_partial_check: Selector,
    // And of course, the K-bit lookup table
    table: RangeTableConfig<F, LOOKUP_NUM_BITS, LOOKUP_RANGE>,
    _marker: PhantomData<F>,
}

impl<F: PrimeField + PrimeFieldBits, const LOOKUP_NUM_BITS: usize, const LOOKUP_RANGE: usize>
    DecomposeConfig<F, LOOKUP_NUM_BITS, LOOKUP_RANGE>
{
    fn configure(meta: &mut ConstraintSystem<F>, running_sum: Column<Advice>) -> Self {
        // Create the needed columns and internal configs.
        let q_decompose = meta.complex_selector();
        let q_partial_check = meta.complex_selector();
        let table = RangeTableConfig::configure(meta);

        meta.enable_equality(running_sum);

        // 
        // z_{i+1} = (z_i - c_i) / 2^K i.e.  `c_i = z_i - z_{i+1} * 2^K`.
        // Range-constrain each K-bit chunk  `c_i = z_i - z_{i+1} * 2^K` derived from the running sum.
        meta.lookup(|meta| {
            let q_decompose = meta.query_selector(q_decompose);

            // z_i
            let z_cur = meta.query_advice(running_sum, Rotation::cur());
            // z_{i+1}
            let z_next = meta.query_advice(running_sum, Rotation::next());
            // c_i = z_i - z_{i+1} * 2^K
            let chunk = z_cur - z_next * F::from(1u64 << LOOKUP_NUM_BITS);
            // println!("z_cur: {:?}, z_next: {:?} ,chunk: {:?}",z_cur, z_next ,chunk); // 0400

            // Lookup default value 0 when q_decompose = 0
            let not_q_decompose = Expression::Constant(F::ONE) - q_decompose.clone();
            let default_chunk = Expression::Constant(F::ZERO);

            vec![(
                q_decompose * chunk + not_q_decompose * default_chunk,
                table.value,
            )]
        });

        // Handle the final partial chunk.
        // 用于处理二进制数的最后一个部分块 (高位 chunk)
        // Shifted: 当我们到达 final chunk 且它的位数 < LOOKUP_NUM_BITS 时，
        // 需要 "shift"这个块, 以使其能够与完整的块进行交互或对比
        meta.create_gate("final partial chunk", |meta| {
            let q_partial_check = meta.query_selector(q_partial_check);

            // z_{C-1}
            let z_prev = meta.query_advice(running_sum, Rotation::prev());
            // z_C
            let z_cur = meta.query_advice(running_sum, Rotation::cur());
            // c_{C-1} = z_{C-1} - z_C * 2^K
            let final_chunk = z_prev - z_cur * F::from(1u64 << LOOKUP_NUM_BITS);

            // shifted_chunk final_chunk * 2^{K - num_bits}
            let shifted_chunk = meta.query_advice(running_sum, Rotation::next());

            // 2^{-num_bits}
            let inv_two_pow_s = meta.query_advice(running_sum, Rotation(2));

            let two_pow_k = F::from(1 << LOOKUP_NUM_BITS);
            let expr = final_chunk * two_pow_k * inv_two_pow_s - shifted_chunk;

            Constraints::with_selector(q_partial_check, [expr])
        });

        meta.lookup(|meta| {
            let q_partial_check = meta.query_selector(q_partial_check);
            let shifted = meta.query_advice(running_sum, Rotation::next());

            // Lookup default value 0 when q_partial_check = 0
            let not_q_partial_check = Expression::Constant(F::ONE) - q_partial_check.clone();
            let default_chunk = Expression::Constant(F::ZERO);

            vec![(
                q_partial_check * shifted + not_q_partial_check * default_chunk,
                table.value,
            )]
        });

        Self {
            running_sum,
            q_decompose,
            q_partial_check,
            table,
            _marker: PhantomData,
        }
    }

    fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        value: AssignedCell<Assigned<F>, F>,
        num_bits: usize,
    ) -> Result<(), Error> {
        // 8 % 3 = 2, 所以最后一个 chunk 只有 2 位， 不足 3 位
        let partial_len = num_bits % LOOKUP_NUM_BITS; // 8 % 3 = 2

        layouter.assign_region(
            || "Decompose value",
            |mut region| {
                let mut offset = 0;

                // 0. Copy in the witnessed `value` at offset = 0
                let mut z = value.copy_advice( // `9a` 
                    || "Copy in value for decomposition",
                    &mut region,
                    self.running_sum,
                    offset,
                )?;
                // println!("z: {:?}", z.value());  `9a` , raw num it self, the 1st of running sum.

                // Increase offset after copying `value`
                offset += 1;

                // 1. Compute the interstitial running sum values {z_1, ..., z_C}}
                //  计算该值的每个二进制块的累积和，从而确保分解是正确的
                let expected_vec_len = if partial_len > 0 {
                    1 + num_bits / LOOKUP_NUM_BITS // 64 / 10 +1 = 7 ; 8 / 3 + 1 = 3
                } else {
                    num_bits / LOOKUP_NUM_BITS
                };
                // println!("expected_vec_len {:?}", expected_vec_len); //  expected_vec_len: 3
                // println!("partial_len {:?}", partial_len); // partial_len: 2
                
                let running_sum: Vec<_> = value
                    .value()
                    .map(|&v| compute_running_sum::<_, LOOKUP_NUM_BITS>(v, num_bits)) // 0x9a, 8
                    .transpose_vec(expected_vec_len);
                
                // println!("running_sum {:?}", running_sum);
                /* running_sum : 
                    Rational(0x98, 0x08)  ,   0x98 / 0x08 = 0x13 = 19 (decimal)
                    Rational(0x80, 0x40)  ,   0x80 / 0x40 = 0x02 = 2 
                    Rational(0x00, 0x200) ,   0x00 / 0x200= 0x00 = 0 (循环到这里结束.)
                */

                // 2. Assign the `running sum` values
                for z_i in running_sum.into_iter() {
                    z = region.assign_advice(
                        || format!("assign z_{:?}", offset),
                        self.running_sum,
                        offset,
                        || z_i,
                    )?;
                    offset += 1;
                }

                // 3. Make sure to enable the relevant selector on each row of the running sum
                //    (but not on the row where z_C is witnessed)
                for offset in 0..(num_bits / LOOKUP_NUM_BITS) { // 8 / 3 =2
                    self.q_decompose.enable(&mut region, offset)?;
                }
                // println!("num_bits / LOOKUP_NUM_BITS {:?}", num_bits / LOOKUP_NUM_BITS);

                // 4. Constrain the final running sum `z_C` to be 0.
                region.constrain_constant(z.cell(), F::ZERO)?;

                // Handle partial chunk
                // println!("value.value(){:?}", value.value());
                if partial_len > 0 { //  8 % 3 = 2
                    // The final chunk, value.value():  Trivial(0x9a) i.e. 154
                    let final_chunk = value.value().map(|v| {
                        let v: Vec<_> = v
                            .evaluate()
                            .to_le_bits()
                            .iter()
                            .by_vals()
                            .take(num_bits)
                            .collect();
                        
                        //  println!("v .. {:?}", v) : [false, true, false, true, true, false, false, true]    
                        //     i.e. [01011001] <-  这个是低位在前, 高位在后. 因为 154 的二进制表示是 [10011010]
                        let final_chunk = &v[(num_bits - partial_len)..num_bits];
                        // final_chunk: [false, true]      ;      println!("final_chunk{:?}", final_chunk);
                        
                        Assigned::from(F::from(lebs2ip(final_chunk))) // 0x02
                    });
                    // final_chunk: 0x02,  i.e. `10` in binary format.
                    self.short_range_check(&mut region, offset - 1, final_chunk, partial_len)?;
                }
                Ok(())
            },
        )
    }

    /// Constrain `x` to be a partial_len word.
    /// 对给定的element进行约束，确保它是一个partial_len位的词（word）
    /// q_partial_check is enabled on the offset of the final running sum z_C.
    /// 本例: 约束 0x02 是一个 2 位的数.
    fn short_range_check(
        &self,
        region: &mut Region<'_, F>,
        offset: usize,
        element: Value<Assigned<F>>, // [01011001]
        partial_len: usize,
    ) -> Result<(), Error> {
        // Enable `q_partial_check`
        self.q_partial_check.enable(region, offset)?;

        // println!("LOOKUP_NUM_BITS, partial_len{:?} {:?}", LOOKUP_NUM_BITS, partial_len);  // 3, 2
        // Assign shifted `element * 2^{K - partial_len}`
        // 10|011|010

        // println!(";;element {:?} \n;;element.into_field() {:?}", element, element.into_field());   0x02 0x02
        // shifted 的取值: 0x02 * 2 , 向左补齐 从 `10` -> `100` 即 4 .
        let shifted = element.into_field() * F::from(1 << (LOOKUP_NUM_BITS - partial_len)); // 1 << 1 = 2
        // println!("shifted {:?}", shifted);  // 01 , i.e. 0x04

        region.assign_advice(
            || format!("element * 2^({}-{})", LOOKUP_NUM_BITS, partial_len),  // 3, 2
            self.running_sum,
            offset + 1,
            || shifted,
        )?;
        // GPT: 这部分代码确保我们有一个正确的 2^{−partial_len} 值，并将其存储在电路的适当位置，以供后续使用
        // Assign 2^{-partial_len} from a fixed column.
        let inv_two_pow_s = F::from(1 << partial_len).invert().unwrap();
        // println!("inv_two_pow_s {:?}", inv_two_pow_s);  0x30019b4f2bd06f9bad4b2e1e4b1c0000001
        region.assign_advice_from_constant(
            || format!("2^(-{})", partial_len),
            self.running_sum,
            offset + 2,
            inv_two_pow_s,
        )?;

        Ok(())
    }
}

// “小端二进制序列到整数”的转换，它将一个布尔值的数组（表示二进制位）转换为一个u64整数。
// little endian binary sequence 2 
fn lebs2ip(bits: &[bool]) -> u64 {
    assert!(bits.len() <= 64);
    bits.iter()
        .enumerate()
        .fold(0u64, |acc, (i, b)| acc + if *b { 1 << i } else { 0 })
}

// Function to compute the interstitial running sum values {z_1, ..., z_C}}
// value: 0x9a
// num_bits: 3
fn compute_running_sum<F: PrimeField + PrimeFieldBits, const LOOKUP_NUM_BITS: usize>(
    value: Assigned<F>,
    num_bits: usize,
) -> Vec<Assigned<F>> {
    let mut running_sum = vec![]; // empty running sum vec.
    let mut z = value; // 0x9a
    // println!("num_bits {:?}", num_bits); // 8

    // Get the little-endian bit representation of `value`.
    let value: Vec<_> = value
        .evaluate()
        .to_le_bits() // 转换为其小端二进制表示
        .iter()
        .by_vals()
        .take(num_bits) // 8 bit
        .collect();
    for chunk in value.chunks(LOOKUP_NUM_BITS) {
        let chunk = Assigned::from(F::from(lebs2ip(chunk)));
        // consider: 10|011|010,  chunk = 2, 3 ,2
        // println!("chunk: {:?}", chunk);

        // z_{i+1} = (z_i - c_i) * 2^{-K}:

        // num = 1 << 3 = 8
        // 1. Assigned::from() 会将 num => Assigned::Trivial(8) 即转化为一个"平凡的" Trivial(8)
        // 2. invert 接收一个 Trivial, 取逆的结果是: Self::Rational(F::ONE, *x) => (F::ONE, 8)
        //   println!("running_sum {:?}", running_sum);
        // 3. z = (z - chunk) * "取逆的结果": 
        /* running_sum : 
            Rational(0x98, 0x08)  ,   0x98 / 0x08 = 0x13 = 19 (decimal)
            Rational(0x80, 0x40)  ,   0x80 / 0x40 = 0x02 = 2 
            Rational(0x00, 0x200) ,   0x00 / 0x200= 0x00 = 0 (循环到这里结束.)
         */
        // cmd + click `*`, 观察: impl<F: Field> Mul for Assigned<F>
        // 可以知道, 如果其中一个操作数是 Trivial (z - chunk)，另一个是 Rational (0x01, 0x08)，
        // 那么乘以分子并保持分母不变。这实际上是乘以一个分数和一个整数的常规操作。
        // 所以下式: 0x98 (hex)  *  (0x01, 0x08) = Rational(0x98, 0x08)
        //    0x98 (hex)  *  (0x01, 0x08)
        z = (z - chunk) * Assigned::from(F::from(1u64 << LOOKUP_NUM_BITS)).invert();
        // println!("z :{:?}",z) ;
        // println!("Assigned::from(F::from(1u64 << LOOKUP_NUM_BITS)).invert(): {:?}", Assigned::from(F::from(1u64 << LOOKUP_NUM_BITS)).invert());
        running_sum.push(z);

    }
    running_sum
}

#[cfg(test)]
mod tests {
    use halo2_proofs::{circuit::floor_planner::V1, dev::MockProver, pasta::Fp};
    use rand;

    use super::*;

    struct MyCircuit<F: PrimeField, const NUM_BITS: usize, const RANGE: usize> {
        value: Value<Assigned<F>>,
        num_bits: usize,
    }

    impl<F: PrimeField + PrimeFieldBits, const NUM_BITS: usize, const RANGE: usize> Circuit<F>
        for MyCircuit<F, NUM_BITS, RANGE>
    {   // DecomposeConfig<F, 10, 1024>
        type Config = DecomposeConfig<F, NUM_BITS, RANGE>; // <F, LOOKUP_NUM_BITS, LOOKUP_RANGE>
        type FloorPlanner = V1;

        fn without_witnesses(&self) -> Self {
            Self {
                value: Value::unknown(),
                num_bits: self.num_bits,
            }
        }

        fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
            // Fixed column for constants
            let constants = meta.fixed_column();
            meta.enable_constant(constants);

            let value = meta.advice_column();
            DecomposeConfig::configure(meta, value)
        }

        fn synthesize(
            &self,
            config: Self::Config,
            mut layouter: impl Layouter<F>,
        ) -> Result<(), Error> {
            config.table.load(&mut layouter)?;

            // Witness the value somewhere
            // `self.value`  is  `9a` , is the raw num itself.
            let value = layouter.assign_region(
                || "Witness value",
                |mut region| {
                    region.assign_advice(|| "Witness value", config.running_sum, 0, || self.value)
                },
            )?;
            // println!("synthesize value : {:?}", value.value()); // 0x9a.

            config.assign(
                layouter.namespace(|| "Decompose value"),
                value,    // value 0x9a.
                self.num_bits, // 8, the len of binary form of the num `154`.
            )?;

            Ok(())
        }
    }

    #[test]
    fn test_decompose_3() {
        // 本例中, K (NUM_BITS) 为 10 (即分解为大小为 10 的块, 查找表的大小为 2^10 )
        let k = 11;
        // i.e. `K` in fomula, const NUM_BITS: usize = 10;
        // const RANGE: usize = 1024; // 10-bit value
        const NUM_BITS: usize = 3; // LOOKUP_NUM_BITS
        const RANGE: usize = 8; // 10-bit value // LOOKUP_RANGE

        // Random u64 value
        // let value: u64 = rand::random();
        let value = 154; // hex is `9A`
        let value = Value::known(Assigned::from(Fp::from(value)));
        // println!("test value  {:?}", value); // 9a
        let circuit = MyCircuit::<Fp, NUM_BITS, RANGE> {
            value,
            num_bits: 8, // `154` : 10011010 是 8 位
        };

        let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        prover.assert_satisfied();
    }

    // fn test_decompose_3_should_fail() {
    //     let k = 11;
    //     const NUM_BITS: usize = 10;
    //     const RANGE: usize = 1024; // 10-bit value

    //     let value = 18446744073709551617;
    //     let value = Value::known(Assigned::from(Fp::from(value)));

    //     // Out-of-range `value = 8`
    //     let circuit = MyCircuit::<Fp, NUM_BITS, RANGE> {
    //         value,
    //         num_bits: 64,
    //     };
    //     let prover = MockProver::run(k, &circuit, vec![]).unwrap();
    //     match prover.verify() {
    //         Err(e) => {
    //             println!("Error successfully achieved!");
    //         }
    //         _ => assert_eq!(1, 0),
    //     }
    // }
    #[cfg(feature = "dev-graph")]
    #[test]
    fn print_decompose_3() {
        use plotters::prelude::*;

        let root = BitMapBackend::new("decompose-layout.png", (1024, 3096)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let root = root
            .titled("Decompose Range Check Layout", ("sans-serif", 60))
            .unwrap();

        let circuit = MyCircuit::<Fp, 10, 1024> {
            value: Value::unknown(),
            num_bits: 64,
        };
        halo2_proofs::dev::CircuitLayout::default()
            .render(11, &circuit, &root)
            .unwrap();
    }
}