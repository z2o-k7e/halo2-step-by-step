use std::marker::PhantomData;
use ff::PrimeField;

use halo2_proofs::{
    circuit::{Layouter, Value},
    plonk::{ConstraintSystem, Error, TableColumn},
};

// Lookup Table for Range Check
/// A lookup table of values up to LOOKUP_RANGE
/// e.g. LOOKUP_RANGE = 256, values = [0..255]
/// This table is tagged by an index `k`, where `k` is the number of bits of the element in the `value` column.
#[derive(Debug, Clone)]
pub(super) struct RangeTableConfig<F: PrimeField, const LOOKUP_NUM_BITS: usize, const LOOKUP_RANGE: usize> {
    pub(super) num_bits: TableColumn,
    pub(super) value: TableColumn,
    _marker: PhantomData<F>,
}

impl<F: PrimeField, const LOOKUP_NUM_BITS: usize, const LOOKUP_RANGE: usize> RangeTableConfig<F, LOOKUP_NUM_BITS, LOOKUP_RANGE> {
    pub(super) fn configure(meta: &mut ConstraintSystem<F>) -> Self {
        println!("RangeTableConfig - configure");
        assert_eq!(1 << LOOKUP_NUM_BITS, LOOKUP_RANGE);

        let num_bits = meta.lookup_table_column();
        let value = meta.lookup_table_column();

        Self {
            num_bits,
            value,
            _marker: PhantomData,
        }
    }

    pub(super) fn load(&self, layouter: &mut impl Layouter<F>) -> Result<(), Error> {
        println!("RangeTableConfig - load");
        layouter.assign_table(
            || "load range-check table",
            |mut table| {
                let mut offset = 0;

                // Assign (num_bits = 1, value = 0)
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

                // 这个范围用于确保在给定的 `num_bits` 下，他所能表示的数字的值在预期的最小和最大之间
                // when (lookup) LOOKUP_NUM_BITS = 3 ; LOOKUP_RANGE = 8 时:
                // - num_bits: [1,2,2,3,3,3,3]
                // - value   : [1,2,3,4,5,6,7]
                // 可以看到这个查找表是蛮小的..
                for num_bits in 1..=LOOKUP_NUM_BITS {
                    for value in (1 << (num_bits - 1))..(1 << num_bits) {
                        table.assign_cell(
                            || "assign num_bits",
                            self.num_bits,
                            offset,
                            || Value::known(F::from(num_bits as u64)),
                        )?;
                        table.assign_cell(
                            || "assign value",
                            self.value,
                            offset,
                            || Value::known(F::from(value as u64)),
                        )?;
                        offset += 1;
                        // println!("num_bits,  value {:?} {:?} ",num_bits,  value);
                    }
                }
                Ok(())
            },
        )
    }
}