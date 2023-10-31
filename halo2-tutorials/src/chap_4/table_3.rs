use std::marker::PhantomData;

use halo2_proofs::{circuit::*, pasta::group::ff::PrimeField, plonk::*};

#[derive(Debug, Clone)]
pub struct RangeCheckTable<F: PrimeField, const NUM_BITS: usize, const RANGE: usize> {
    pub n_bits: TableColumn,
    pub value: TableColumn,
    _marker: PhantomData<F>,
}

impl<F: PrimeField, const NUM_BITS: usize, const RANGE: usize> RangeCheckTable<F, NUM_BITS, RANGE> {
    pub fn configure(meta: &mut ConstraintSystem<F>) -> Self {
        let n_bits = meta.lookup_table_column();
        let value = meta.lookup_table_column();
        RangeCheckTable {
            n_bits,
            value,
            _marker: PhantomData,
        }
    }

    pub fn load(&self, mut layouter: impl Layouter<F>) -> Result<(), Error> {
        layouter.assign_table(
            || "table",
            |mut table| {
                let mut offset = 0;
                //Assign bit=1, value=0
                table.assign_cell(
                    || "n_bits cell",
                    self.n_bits,
                    offset,
                    || Value::known(F::from(1 as u64)),
                )?;
                table.assign_cell(
                    || "value cell",
                    self.value,
                    offset,
                    || Value::known(F::from(0 as u64)),
                )?;
                offset += 1;

                for n_bits in 1..=NUM_BITS {
                    for value in 1 << (n_bits - 1)..1 << n_bits {
                        table.assign_cell(
                            || "n_bits cell",
                            self.n_bits,
                            offset,
                            || Value::known(F::from(n_bits as u64)),
                        )?;
                        table.assign_cell(
                            || "value cell",
                            self.value,
                            offset,
                            || Value::known(F::from(value as u64)),
                        )?;
                        offset += 1;
                    }
                }
                Ok(())
            },
        )
    }
}
