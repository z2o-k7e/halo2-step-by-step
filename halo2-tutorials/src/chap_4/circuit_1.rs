/// A circuit to demonstrate we can do lookup on different rows in different columns
use std::marker::PhantomData;

use halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner, Value},
    pasta::group::ff::PrimeField,
    plonk::*,
    poly::Rotation,
};

#[derive(Clone)]
struct LookupConfig {
    a: Column<Advice>,
    b: Column<Advice>,
    s: Selector,
    t1: TableColumn,
    t2: TableColumn,
}

struct LookupChip<F: PrimeField> {
    config: LookupConfig,
    _marker: PhantomData<F>,
}

impl<F: PrimeField> LookupChip<F> {
    fn construct(config: LookupConfig) -> Self {
        LookupChip {
            config,
            _marker: PhantomData,
        }
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> LookupConfig {
        let a = meta.advice_column();
        let b = meta.advice_column();
        let s = meta.complex_selector();
        let t1 = meta.lookup_table_column();
        let t2 = meta.lookup_table_column();

        meta.enable_equality(a);
        meta.enable_equality(b);

        meta.lookup(|meta| {
            let cur_a = meta.query_advice(a, Rotation::cur());
            let next_b = meta.query_advice(b, Rotation::next());
            let s = meta.query_selector(s);
            // we'll assgin (0,0) in t1,t2 table
            // so the default condition for other rows without need to lookup will also satisfy this constriant
            vec![(s.clone() * cur_a, t1), (s * next_b, t2)]
        });

        LookupConfig { a, b, s, t1, t2 }
    }

    fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        a_arr: &Vec<Value<F>>,
        b_arr: &Vec<Value<F>>,
    ) -> Result<(), Error> {
        layouter.assign_region(
            || "a,b",
            |mut region| {
                for i in 0..a_arr.len() {
                    self.config.s.enable(&mut region, i)?;
                    region.assign_advice(|| "a col", self.config.a, i, || a_arr[i])?;
                }

                for i in 0..b_arr.len() {
                    region.assign_advice(|| "b col", self.config.b, i, || b_arr[i])?;
                }

                Ok(())
            },
        )?;

        layouter.assign_table(
            || "t1,t2",
            |mut table| {
                for i in 0..10 {
                    table.assign_cell(
                        || "t1",
                        self.config.t1,
                        i,
                        || Value::known(F::from(i as u64)),
                    )?;
                    table.assign_cell(
                        || "t2",
                        self.config.t2,
                        i,
                        || Value::known(F::from(i as u64)),
                    )?;
                }

                Ok(())
            },
        )?;

        Ok(())
    }
}

#[derive(Default)]
struct MyCircuit<F: PrimeField> {
    a: Vec<Value<F>>,
    b: Vec<Value<F>>,
}

impl<F: PrimeField> Circuit<F> for MyCircuit<F> {
    type Config = LookupConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        MyCircuit::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        LookupChip::configure(meta)
    }

    fn synthesize(&self, config: Self::Config, layouter: impl Layouter<F>) -> Result<(), Error> {
        let chip = LookupChip::<F>::construct(config);
        chip.assign(layouter, &self.a, &self.b)
    }
}

#[cfg(test)]
mod tests {
    use halo2_proofs::{dev::MockProver, pasta::Fp};

    use super::*;
    #[test]
    fn test_lookup_on_different_rows() {
        let k = 5;
        let a = [0, 1, 2, 3];
        let b = [0, 0, 1, 2, 3];
        let a = a.map(|v| Value::known(Fp::from(v))).to_vec();
        let b = b.map(|v| Value::known(Fp::from(v))).to_vec();

        let circuit = MyCircuit { a, b };
        let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        prover.assert_satisfied();
    }
}
