/// A circuit to demonstrate we can do lookup on different rows in different columns
use std::marker::PhantomData;

use halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner, Value},
    pasta::group::ff::PrimeField,
    plonk::*,
    poly::Rotation,
};

/// Circuit design:
/// | advice_a| advice_b| q_lookup| table_1 | table_2 |
/// |---------|---------|---------|---------|---------|
/// |    0    |    0    |    1    |    0    |    0    |
/// |    1    |    0    |    1    |    1    |    1    |
/// |    2    |    1    |    1    |    2    |    2    |
/// |    3    |    2    |    1    |    3    |    3    |
/// |         |    3    |    0    |    4    |    4    |
/// |         |         |   ...   |   ...   |   ...   |
/// |         |         |    0    |  RANGE  |  RANGE  |
/// - cur_a ∈ t1
/// - next_b ∈ t2

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
            // we'll assgin (0, 0) in t1, t2 table
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
        let a = [0, 1, 2, 3, 4];
        let b = [0, 0, 1, 2, 3, 4];
        let a = a.map(|v| Value::known(Fp::from(v))).to_vec();
        let b = b.map(|v| Value::known(Fp::from(v))).to_vec();

        let circuit = MyCircuit { a, b };
        let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        prover.assert_satisfied();
    }

    #[cfg(feature = "dev-graph")]
    #[test]
    fn plot_lookup_on_different_rows() {
        let k = 5;
        let a = [0, 1, 2, 3, 4];
        let b = [0, 0, 1, 2, 3, 4];
        let a = a.map(|v| Value::known(Fp::from(v))).to_vec();
        let b = b.map(|v| Value::known(Fp::from(v))).to_vec();
        let circuit = MyCircuit { a, b };

        // Create the area you want to draw on.
        // Use SVGBackend if you want to render to .svg instead.
        use plotters::prelude::*;
        let root = BitMapBackend::new("./circuit_layouter_plots/chap_4_lookup_on_different_rows.png", (1024, 768))
            .into_drawing_area();
        root.fill(&WHITE).unwrap();
        let root = root
            .titled("Simple Lookup Circuit", ("sans-serif", 60))
            .unwrap();

        halo2_proofs::dev::CircuitLayout::default()
            // You can optionally render only a section of the circuit.
            // .view_width(0..2)
            // .view_height(0..16)
            // You can hide labels, which can be useful with smaller areas.
            .show_labels(true)
            .mark_equality_cells(true)
            .show_equality_constraints(true)
            // Render the circuit onto your area!
            // The first argument is the size parameter for the circuit.
            .render(5, &circuit, &root)
            .unwrap();
    }
}
