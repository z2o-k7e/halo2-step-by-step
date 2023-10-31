// Problem to prove:  a in [0, RANGE]
use halo2_proofs::{
    circuit::{AssignedCell, Layouter, SimpleFloorPlanner, Value},
    pasta::group::ff::PrimeField,
    plonk::*,
    poly::Rotation,
};

use super::table_2::*;

/// Circuit design:
/// | adv   | q_lookup|  table  |
/// |-------|---------|---------|
/// | a[0]  |    1    |    0    |
/// | a[1]  |    1    |    1    |
/// |  ...  |   ...   |   ...   |
/// | a[N]  |    1    |   N-1   |
/// |       |    0    |    N    |
/// |       |   ...   |   ...   |
/// |       |    0    |  RANGE  |

struct ACell<F: PrimeField>(AssignedCell<Assigned<F>, F>);
#[derive(Debug, Clone)]
struct RangeConfig<F: PrimeField, const RANGE: usize, const NUM: usize> {
    value: Column<Advice>,
    table: LookUpTable<F, RANGE>,
    q_lookup: Selector,
}

impl<F: PrimeField, const RANGE: usize, const NUM: usize> RangeConfig<F, RANGE, NUM> {
    fn configure(meta: &mut ConstraintSystem<F>, value: Column<Advice>) -> Self {
        let q_lookup = meta.complex_selector();
        let table = LookUpTable::<F, RANGE>::configure(meta);
        meta.lookup(|meta| {
            let q_lookup = meta.query_selector(q_lookup);
            let v = meta.query_advice(value, Rotation::cur());
            vec![(q_lookup * v, table.table)]
        });

        RangeConfig {
            value,
            table,
            q_lookup,
        }
    }

    fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        value: [Value<Assigned<F>>; NUM],
    ) -> Result<ACell<F>, Error> {
        layouter.assign_region(
            || "value to check",
            |mut region| {
                //instantiate a new region, so it's not ref
                self.q_lookup.enable(&mut region, 0)?;
                let mut cell = region
                    .assign_advice(|| "value", self.value, 0, || value[0])
                    .map(ACell);
                for i in 1..value.len() {
                    self.q_lookup.enable(&mut region, i)?;
                    cell = region
                        .assign_advice(|| "value", self.value, i, || value[i])
                        .map(ACell);
                }
                cell
            },
        )
    }
}

#[derive(Debug)]
struct MyCircuit<F: PrimeField, const RANGE: usize, const NUM: usize> {
    value: [Value<Assigned<F>>; NUM],
}

impl<F: PrimeField, const RANGE: usize, const NUM: usize> MyCircuit<F, RANGE, NUM> {
    fn default() -> Self {
        let mut values = vec![];
        for i in 0..NUM {
            values.push(Value::known(Assigned::from(F::from(i as u64))));
        }

        let values = values.try_into().unwrap();
        MyCircuit::<F, RANGE, NUM> { value: values }
    }
}

impl<F: PrimeField, const RANGE: usize, const NUM: usize> Circuit<F> for MyCircuit<F, RANGE, NUM> {
    type Config = RangeConfig<F, RANGE, NUM>;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        MyCircuit::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let advice = meta.advice_column();
        RangeConfig::configure(meta, advice)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        config
            .table
            .load(&mut layouter.namespace(|| "lookup col"))?;
        config.assign(layouter.namespace(|| "range check"), self.value)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use halo2_proofs::{dev::MockProver, pasta::Fp};

    use super::*;

    #[test]
    fn test_rangecheck_lookup() {
        const NUM: usize = 3;
        let mut values = vec![];
        for i in 0..NUM {
            values.push(Value::known(Assigned::from(Fp::from(i as u64))));
        }

        let circuit = MyCircuit::<Fp, 16, NUM> {
            value: values.clone().try_into().unwrap(),
        };
        let k = 5;

        let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        prover.assert_satisfied();

        values[1] = Value::known(Assigned::from(Fp::from(18 as u64)));
        let circuit = MyCircuit::<Fp, 16, NUM> {
            value: values.clone().try_into().unwrap(),
        };
        let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        assert!(prover.verify().is_err());
    }

    #[cfg(feature = "dev-graph")]
    #[test]
    fn plot_lookup_circuit() {
        // Instantiate the circuit with the private inputs.
        let circuit = MyCircuit::<Fp, 16, 5>::default();
        // Create the area you want to draw on.
        // Use SVGBackend if you want to render to .svg instead.
        use plotters::prelude::*;
        let root = BitMapBackend::new("./circuit_layouter_plots/lookup.png", (1024, 768))
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
            // Render the circuit onto your area!
            // The first argument is the size parameter for the circuit.
            .render(5, &circuit, &root)
            .unwrap();
    }
}
