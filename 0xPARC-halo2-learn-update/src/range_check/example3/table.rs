use std::marker::PhantomData;
use ff::{Field, PrimeField};
use halo2_proofs::{
    circuit::{Layouter, Value},
    plonk::{ConstraintSystem, Error, TableColumn},
};

/// A lookup table of values up to RANGE
/// e.g. RANGE = 256, values = [0..255]
/// This table is tagged by an index `k`, where `k` is the number of bits of the element in the `value` column.
#[derive(Debug, Clone)]
pub(super) struct RangeTableConfig<F: PrimeField, const NUM_BITS: usize, const RANGE: usize> {
    pub(super) num_bits: TableColumn, // tag for our table.
    pub(super) value: TableColumn,
    _marker: PhantomData<F>,
}

impl<F: PrimeField, const NUM_BITS: usize, const RANGE: usize> RangeTableConfig<F, NUM_BITS, RANGE> {
    pub(super) fn configure(meta: &mut ConstraintSystem<F>) -> Self {
        // 确保RANGE等于 2 的 NUM_BITS 次方，这是为了确保指定的范围与期望的位数相匹配
        //   "1" 左移一位 NUM_BITS 位, 即变大 1 的 2^NUM_BITS 倍
        assert_eq!(1 << NUM_BITS, RANGE);  
        
        // 为 num_bits 和 value 定义查找表列。这两个列将在查找表中用于存储数据。
        let num_bits = meta.lookup_table_column();
        let value = meta.lookup_table_column();

        Self {
            num_bits,
            value,
            _marker: PhantomData,
        }
    }

    pub(super) fn load(&self, layouter: &mut impl Layouter<F>) -> Result<(), Error> {
        layouter.assign_table(
            || "load range-check table",
            |mut table| {
                let mut offset = 0;

                // Assign (num_bits = 1, value = 0), 2 列都是 lookup columns.
                // 这部分是赋值首行, 为 num_bits 和 value 分配了其首个值，即 1 和 0, 方便下面累加
                {
                    table.assign_cell(
                        || "assign num_bits",
                        self.num_bits,
                        offset,
                        || Value::known(F::ONE),
                    )?;
                    table.assign_cell(
                        || "assign value",
                        self.value,
                        offset,
                        || Value::known(F::ZERO),
                    )?;

                    offset += 1;
                }

                // (1 << (num_bits_ - 1))..(1 << num_bits_) : 在给定的 NUM_BITS 下的 min & max value.
                //   num_bits_ 标识了 value 所占的位数,比如 213
                //   value_ 则是实际赋值(约束)到电路里的实际 Private value
                for num_bits_ in 1..=NUM_BITS {
                    for value_ in (1 << (num_bits_ - 1))..(1 << num_bits_) {
                        table.assign_cell(
                            || "assign num_bits",
                            self.num_bits,
                            offset,
                            || Value::known(F::from(num_bits_ as u64)),
                        )?;
                        table.assign_cell(
                            || "assign value",
                            self.value,
                            offset,
                            || Value::known(F::from(value_ as u64)),
                        )?;
                        offset += 1;
                    }
                }
                Ok(())
            },
        )
    }
}
