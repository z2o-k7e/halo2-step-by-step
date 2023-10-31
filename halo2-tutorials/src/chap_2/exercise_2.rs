use std::marker::PhantomData;

/// chap2: chip
/// Prove knowing knowledge of three private inputs a, b, c
/// s.t: 
///     d = a^2 * b^2 * c
///     e = c + d
///     out = e^3

use halo2_proofs::{
    arithmetic::Field,
    plonk::{Advice, Column, Instance, Selector, ConstraintSystem, Error, Circuit, Constraints}, 
    circuit::{AssignedCell, Layouter, Value,SimpleFloorPlanner}, 
    poly::Rotation
};

/// Circuit design:
// / | ins   | a0    | a1    | s_mul | s_add | s_cub |
// / |  out  |-------|-------|-------|-------|-------|
// / |       |    a  |       |       |       |       |
// / |       |    b  |       |       |       |       |
// / |       |    c  |       |       |       |       |
// / |       |   a   |   b   |   1   |   0   |   0   |
// / |       |   ab  |   ab  |   1   |   0   |   0   |
// / |       | absq  |   c   |   1   |   0   |   0   |
// / |       |  d    |   c   |   0   |   1   |   0   |
// / |       |  e    |   out |   0   |   0   |   1   |


#[derive(Debug, Clone)]
struct SimpleConfig {
    advice: [Column<Advice>;2],
    instance: Column<Instance>,
    s_mul: Selector,
    s_add: Selector,
    s_cub: Selector,
}

#[derive(Clone)]
struct Number<F:Field>(AssignedCell<F,F>);

#[derive(Debug, Clone)]
struct SimpleChip<F:Field> {
    config: SimpleConfig,
    _marker: PhantomData<F>
}

impl <F:Field> SimpleChip<F> {
    pub fn construct(config: SimpleConfig) -> Self {
        SimpleChip { config, _marker: PhantomData }
    }
    pub fn configure(meta: &mut ConstraintSystem<F>) -> SimpleConfig {
        let advice = [meta.advice_column(),meta.advice_column()];
        let instance = meta.instance_column();
        let constant = meta.fixed_column();

        meta.enable_equality(instance);
        meta.enable_constant(constant);
        for c in &advice {
            meta.enable_equality(*c);
        }
        let s_mul = meta.selector();
        let s_add = meta.selector();
        let s_cub = meta.selector();

        meta.create_gate("mul_gate", |meta| {
            let lhs = meta.query_advice(advice[0], Rotation::cur());
            let rhs = meta.query_advice(advice[1], Rotation::cur());
            let out = meta.query_advice(advice[0], Rotation::next());
            let s_mul = meta.query_selector(s_mul);
            Constraints::with_selector(s_mul, vec![(lhs * rhs - out)])
        });

        meta.create_gate("add_gate", |meta| {
            let lhs = meta.query_advice(advice[0], Rotation::cur());
            let rhs = meta.query_advice(advice[1], Rotation::cur());
            let out = meta.query_advice(advice[0], Rotation::next());
            let s_add = meta.query_selector(s_add);
            Constraints::with_selector(s_add, vec![(lhs + rhs - out)])
        });

        meta.create_gate("cub_gate", |meta| {
            let lhs = meta.query_advice(advice[0], Rotation::cur());
            let out = meta.query_advice(advice[1], Rotation::cur());
            let s_cub = meta.query_selector(s_cub);
            Constraints::with_selector(s_cub, vec![(lhs.clone()*lhs.clone()*lhs - out)])
        });

        SimpleConfig {
            advice,
            instance,
            s_mul,
            s_add,
            s_cub
        }
    }

    pub fn assign( 
        &self,
        mut layouter: impl Layouter<F>,
        a: Value<F>,
        b: Value<F>,
        c: F
        ) -> Result<Number<F>, Error> {
            let cells = layouter.assign_region(
                || "load private", 
            |mut region| {
                let a_cell = region.assign_advice(|| "private input a",  self.config.advice[0], 0, || a).map(Number)?;
                let b_cell = region.assign_advice(|| "private input b",  self.config.advice[0], 1, || b).map(Number)?;
                let c_cell = region.assign_advice_from_constant(|| "private input c",  self.config.advice[0], 2, c).map(Number)?;
                Ok((a_cell,b_cell, c_cell))
            }).unwrap();

            layouter.assign_region(|| "load witness", move |mut region|{
                let config = &self.config;
                let mut offset = 0;
                //load a,b
                let (a,b, c) = &cells;
                config.s_mul.enable(&mut region, offset)?;
                let a = a.0.copy_advice(|| "lhs", &mut region, self.config.advice[0], offset).map(Number)?;
                let b = b.0.copy_advice(|| "rhs", &mut region, self.config.advice[1], offset).map(Number)?;
                //fill ab,ab
                offset += 1;
                config.s_mul.enable(&mut region, offset)?;
                let value = a.0.value().copied() * b.0.value().copied();
                let ab_0 = region.assign_advice(|| "ab lhs", config.advice[0], offset, || value).map(Number)?;
                let ab_1 = ab_0.0.copy_advice(|| "ab rhs", &mut region, self.config.advice[1], offset).map(Number)?;
                //fill absq,c
                offset += 1;
                config.s_mul.enable(&mut region, offset)?;
                let value = ab_0.0.value().copied() * ab_1.0.value().copied();
                let absq =  region.assign_advice(|| "absq", config.advice[0], offset, || value).map(Number)?;
                let c = c.0.copy_advice(|| "c", &mut region, self.config.advice[1], offset).map(Number)?;
                //fill c, d
                offset += 1;
                config.s_add.enable(&mut region, offset)?;
                let value = absq.0.value().copied() * c.0.value().copied();
                let d =  region.assign_advice(|| "d", config.advice[0], offset, || value).map(Number)?;
                let c = c.0.copy_advice(|| "c", &mut region, self.config.advice[1], offset).map(Number)?;
                // fill e
                offset += 1;
                let value = d.0.value().copied() + c.0.value().copied();
                let e = region.assign_advice(|| "e", config.advice[0], offset, || value).map(Number)?;
                //fill out
                config.s_cub.enable(&mut region, offset)?;
                let value = e.0.value().copied() * e.0.value().copied() * e.0.value().copied();
                region.assign_advice(|| "out", config.advice[1], offset, || value).map(Number)
            })
    }

    fn expose_public(
        &self,
        mut layouter: impl Layouter<F>,
        out: Number<F>,
        row: usize
    )  -> Result<(), Error> {
        layouter.constrain_instance(out.0.cell(), self.config.instance, row)
    }
}

#[derive(Default)]
struct MyCircuit<F:Field> {
    c: F,
    a: Value<F>,
    b: Value<F>
}

impl <F:Field> Circuit<F> for MyCircuit<F> {
    type Config = SimpleConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        SimpleChip::configure(meta)
    }

    fn synthesize(&self, config: Self::Config, mut layouter: impl Layouter<F>) -> Result<(), Error> {
        //assign witness
        let chip = SimpleChip::construct(config);
        let out = chip.assign(layouter.namespace(|| "simple ship"), self.a, self.b, self.c)?;
        //expose public
        chip.expose_public(layouter, out, 0)
    }
}



#[cfg(test)]
mod tests {
    use halo2_proofs::{dev::MockProver, pasta::Fp};
    use super::*;

    fn circuit() -> (MyCircuit<Fp>, Fp) {
        // Prepare the private and public inputs to the circuit!
        let c = Fp::from(2);
        let a = Fp::from(2);
        let b = Fp::from(3);
        let e = c * a.square() * b.square() + c;
        let out = e.cube();
        println!("out=:{:?}",out);
    
        // Instantiate the circuit with the private inputs.
        (MyCircuit {
            c,
            a: Value::known(a),
            b: Value::known(b),
        }, out)
    }
    #[test]
    fn test_simple_ship() {
        // ANCHOR: test-circuit
        // The number of rows in our circuit cannot exceed 2^k. Since our example
        // circuit is very small, we can pick a very small value here.
        let k = 5;
        let (circuit, out) = circuit();
    
        // Arrange the public input. We expose the multiplication result in row 0
        // of the instance column, so we position it there in our public inputs.
        let mut public_inputs = vec![out];
    
        // Given the correct public input, our circuit will verify.
        let prover = MockProver::run(k, &circuit, vec![public_inputs.clone()]).unwrap();
        assert_eq!(prover.verify(), Ok(()));
    
        // If we try some other public input, the proof will fail!
        public_inputs[0] += Fp::one();
        let prover = MockProver::run(k, &circuit, vec![public_inputs]).unwrap();
        assert!(prover.verify().is_err());
        println!("simple_ship success!")
        // ANCHOR_END: test-circuit
    }

    #[cfg(feature = "dev-graph")]
    #[test]
    fn plot_chip_circuit(){
        // Instantiate the circuit with the private inputs.
        let (circuit, c) = circuit();
        // Create the area you want to draw on.
        // Use SVGBackend if you want to render to .svg instead.
        use plotters::prelude::*;
        let root = BitMapBackend::new("./images/simple_ship.png", (1024, 768)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let root = root
            .titled("Simple_ship Circuit chip", ("sans-serif", 60))
            .unwrap();

        halo2_proofs::dev::CircuitLayout::default()
            // You can optionally render only a section of the circuit.
            // .view_width(0..2)
            // .view_height(0..16)
            // You can hide labels, which can be useful with smaller areas.
            .show_labels(true)
            // Render the circuit onto your area!
            // The first argument is the size parameter for the circuit.
            .render(4, &circuit, &root)
            .unwrap();
    }
}