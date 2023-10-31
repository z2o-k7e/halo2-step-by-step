use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::Field,
    circuit::{AssignedCell, Layouter, SimpleFloorPlanner},
    plonk::*,
    poly::Rotation,
};

/// Circuit design:
/// | ins   | a0     |   a1   | seletor|
/// |-------|------- |------- |------- |
/// |   a   | f(0)=a | f(1)=b |    1   |
/// |   b   | f(2)=b | f(3)   |    1   |  
/// |  out  | f(4)   | f(5)   |    1   |   
/// |          ...            |        |
/// |       | f(2n/2) |f(2n/2+1)|   1  |
///
/// out = n % 2 == 0 ? f(2n/2) : f(2n/2 + 1)

#[derive(Clone, Debug)]
struct FiboChipConfig {
    advice: [Column<Advice>; 2],
    selector: Selector,
    instance: Column<Instance>,
}

#[derive(Clone, Debug)]
struct FiboChip<F: Field> {
    config: FiboChipConfig,
    _marker: PhantomData<F>,
}

struct ACell<F: Field>(AssignedCell<F, F>);

impl<F: Field> FiboChip<F> {
    fn construct(config: FiboChipConfig) -> Self {
        FiboChip {
            config,
            _marker: PhantomData,
        }
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> FiboChipConfig {
        let instance = meta.instance_column();
        let selector = meta.selector();
        let advice = [meta.advice_column(), meta.advice_column()];
        meta.enable_equality(instance);
        for col in &advice {
            meta.enable_equality(*col);
        }

        meta.create_gate("fibo gate", |meta| {
            let s = meta.query_selector(selector);
            let cur_left = meta.query_advice(advice[0], Rotation::cur());
            let cur_right = meta.query_advice(advice[1], Rotation::cur());
            let next_left = meta.query_advice(advice[0], Rotation::next());
            let next_right = meta.query_advice(advice[1], Rotation::next());
            Constraints::with_selector(
                s,
                vec![
                    (cur_left + cur_right.clone() - next_left.clone()),
                    (cur_right + next_left - next_right),
                ],
            )
        });
        FiboChipConfig {
            advice,
            selector,
            instance,
        }
    }

    fn assign(&self, mut layouter: impl Layouter<F>, nrow: usize) -> Result<ACell<F>, Error> {
        layouter.assign_region(
            || "fibo region",
            |mut region| {
                let left_advice = self.config.advice[0];
                let right_advice = self.config.advice[1];
                let instance = self.config.instance;
                let s = self.config.selector;

                let mut prev_left = region
                    .assign_advice_from_instance(|| "f0", instance, 0, left_advice, 0)
                    .map(ACell)?;
                let mut prev_right = region
                    .assign_advice_from_instance(|| "f1", instance, 1, right_advice, 0)
                    .map(ACell)?;

                for i in 1..=nrow / 2 {
                    s.enable(&mut region, i - 1)?;
                    let value = prev_left.0.value().copied() + prev_right.0.value().copied();
                    let cur_left = region
                        .assign_advice(|| "f left", left_advice, i, || value)
                        .map(ACell)?;
                    let value = prev_right.0.value().copied() + cur_left.0.value().copied();
                    let cur_right = region
                        .assign_advice(|| "f right", right_advice, i, || value)
                        .map(ACell)?;
                    prev_left = cur_left;
                    prev_right = cur_right;
                }

                if nrow % 2 == 0 {
                    Ok(prev_left)
                } else {
                    Ok(prev_right)
                }
            },
        )
    }
}

#[derive(Debug, Default)]
struct FiboCircuit<F: Field> {
    nrow: usize,
    _marker: PhantomData<F>,
}

impl<F: Field> Circuit<F> for FiboCircuit<F> {
    type Config = FiboChipConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        FiboCircuit::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        FiboChip::configure(meta)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let chip = FiboChip::<F>::construct(config);
        let out = chip.assign(layouter.namespace(|| "fibo layouter"), self.nrow)?;
        //expose public
        layouter
            .namespace(|| "out")
            .constrain_instance(out.0.cell(), chip.config.instance, 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use halo2_proofs::{dev::MockProver, pasta::Fp};

    fn fib(n: u64) -> u64 {
        match n {
            0 => 1,
            1 => 1,
            _ => fib(n - 1) + fib(n - 2),
        }
    }

    #[test]
    fn test_fibo2() {
        let f0 = Fp::from(1);
        let f1 = Fp::from(1);
        let n = 11;
        let out = Fp::from(fib(n));
        let circuit = FiboCircuit {
            nrow: n as usize,
            _marker: PhantomData,
        };

        let k = 4;
        let public_inputs = vec![f0, f1, out];
        let prover = MockProver::run(k, &circuit, vec![public_inputs.clone()]).unwrap();
        prover.assert_satisfied();
    }

    #[cfg(feature = "dev-graph")]
    #[test]
    fn plot_fibo2_circuit() {
        // Instantiate the circuit with the private inputs.
        let circuit = FiboCircuit::<Fp> {
            nrow: 20,
            _marker: PhantomData,
        };
        // Create the area you want to draw on.
        // Use SVGBackend if you want to render to .svg instead.
        use plotters::prelude::*;
        let root = BitMapBackend::new("./images/fibo2.png", (1024, 768)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let root = root.titled("Fibo Circuit", ("sans-serif", 60)).unwrap();

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
            .render(4, &circuit, &root)
            .unwrap();
    }
}
