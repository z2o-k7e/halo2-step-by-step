use std::marker::PhantomData;

use halo2_proofs::{circuit::*, pasta::group::ff::PrimeField, plonk::*};

#[derive(Debug, Clone)]
pub(crate) struct LookUpTable<F: PrimeField, const RANGE: usize> {
    pub(crate) table: TableColumn,
    _maker: PhantomData<F>,
}

impl<F: PrimeField, const RANGE: usize> LookUpTable<F, RANGE> {
    pub fn configure(meta: &mut ConstraintSystem<F>) -> Self {
        let table = meta.lookup_table_column();
        Self {
            table,
            _maker: PhantomData,
        }
    }

    pub fn load(&self, layouter: &mut impl Layouter<F>) -> Result<(), Error> {
        layouter.assign_table(
            || "load range lookup table",
            |mut table| {
                for value in 0..RANGE {
                    table.assign_cell(
                        || "table cell",
                        self.table,
                        value,
                        || Value::known(F::from(value as u64)),
                    )?;
                }
                Ok(())
            },
        )
    }
}
