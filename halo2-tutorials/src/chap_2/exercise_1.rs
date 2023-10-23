/// chap2: custom gates
/// Prove knowing knowledge of three private inputs a , b, c
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
// / |       |   c  |        |       |       |       |
// / |       |   a  |   b   |   1   |   0   |   0   |
// / |       |   ab  |       |   0   |   0   |   0   |
// / |       |   ab  |   ab  |   1   |   0   |   0   |
// / |       | absq  |       |   0   |   0   |   0   |
// / |       |  absq |   c   |   1   |   0   |   0   |
// / |       |  d    |       |   0   |   0   |   0   |
// / |       |  c    |  d    |   0   |   1   |   0   |
// / |       |  e    |       |   0   |   0   |   0   |
// / |       |  e    |  out  |   0   |   0   |   1   |


#[derive(Debug, Clone)]
struct CircuitConfig {
    advice: [Column<Advice>;2],
    instance: Column<Instance>,
    s_mul: Selector,
    s_add: Selector,
    s_cub: Selector,
}

#[derive(Clone)]
struct Number<F:Field>(AssignedCell<F,F>);

#[derive(Default)]
struct MyCircuit<F:Field> {
    c: F,
    a: Value<F>,
    b: Value<F>
}

fn load_private<F:Field>( 
    config: &CircuitConfig,
    mut layouter: impl Layouter<F>,
    value: Value<F>) -> Result<Number<F>, Error> {
    layouter.assign_region(
        || "load private", 
    |mut region| {
        region.assign_advice(
            || "private input", 
            config.advice[0], 
            0, 
            || value
        ).map(Number)
    })
}

fn load_constant<F:Field>( 
    config: &CircuitConfig,
    mut layouter: impl Layouter<F>,
    c: F
) -> Result<Number<F>, Error> {
    layouter.assign_region(
        || "load private", 
    |mut region| {
        region.assign_advice_from_constant(
            || "private input", 
            config.advice[0], 
            0, 
            c
        ).map(Number)
    })
}

fn mul<F:Field>(
    config: &CircuitConfig,
    mut layouter: impl Layouter<F>,
    a: Number<F>,
    b: Number<F>,
) -> Result<Number<F>, Error> {
    layouter.assign_region(
        || "mul", 
    |mut region| {
        config.s_mul.enable(&mut region, 0)?;
        a.0.copy_advice(|| "lhs", &mut region, config.advice[0], 0)?;
        b.0.copy_advice(|| "rhs", &mut region, config.advice[1], 0)?;

        let value = a.0.value().copied() * b.0.value().copied();
        region.assign_advice(|| "out=lhs*rhs", config.advice[0], 1, || value)
        .map(Number)
    })
}

fn add<F:Field>(
    config: &CircuitConfig,
    mut layouter: impl Layouter<F>,
    a: Number<F>,
    b: Number<F>,
) -> Result<Number<F>, Error> {
    layouter.assign_region(
        || "add", 
    |mut region| {
        config.s_add.enable(&mut region, 0)?;
        a.0.copy_advice(|| "lhs", &mut region, config.advice[0], 0)?;
        b.0.copy_advice(|| "rhs", &mut region, config.advice[1], 0)?;

        let value = a.0.value().copied() + b.0.value().copied();
        region.assign_advice(|| "out=lhs+rhs", config.advice[0], 1, || value)
        .map(Number)
    })
}

fn cub<F:Field>(
    config: &CircuitConfig,
    mut layouter: impl Layouter<F>,
    a: Number<F>,
) -> Result<Number<F>, Error> {
    layouter.assign_region(
        || "cub", 
    |mut region| {
        config.s_cub.enable(&mut region, 0)?;
        a.0.copy_advice(|| "lhs", &mut region, config.advice[0], 0)?;
        let value = a.0.value().copied()*a.0.value().copied()*a.0.value().copied();
        region.assign_advice(|| "out=lhs^3", config.advice[1], 0, || value)
        .map(Number)
    })
}

impl <F:Field> Circuit<F> for MyCircuit<F> {
    type Config = CircuitConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
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

        CircuitConfig {
            advice,
            instance,
            s_mul,
            s_add,
            s_cub
        }
    }

    fn synthesize(&self, config: Self::Config, mut layouter: impl Layouter<F>) -> Result<(), Error> {
        let a = load_private(&config,layouter.namespace(|| "load a"), self.a)?;
        let b = load_private(&config,layouter.namespace(|| "load b"), self.b)?;
        let c = load_constant(&config,layouter.namespace(|| "load constant"), self.c)?;


        let ab = mul(&config,layouter.namespace(|| "a*b"), a, b)?;
        let absq = mul(&config,layouter.namespace(|| "ab*ab"), ab.clone(), ab)?;
        let d = mul(&config, layouter.namespace(|| "absq*c"), absq, c.clone())?;

        let e = add(&config, layouter.namespace(|| "absq + c"), d, c)?;
        let out = cub(&config, layouter.namespace(|| "e^3"), e)?;

        //expose public
        layouter.namespace(|| "expose out").constrain_instance(out.0.cell(), config.instance, 0)
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
    fn test_simple_3gates() {
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
        println!("simple_3gates success!")
        // ANCHOR_END: test-circuit
    }

    #[cfg(feature = "dev-graph")]
    #[test]
    fn plot_3gates_circuit(){
        // Instantiate the circuit with the private inputs.
        let (circuit, c) = circuit();
        // Create the area you want to draw on.
        // Use SVGBackend if you want to render to .svg instead.
        use plotters::prelude::*;
        let root = BitMapBackend::new("./images/simple_3gates.png", (1024, 768)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let root = root
            .titled("Simple_3gates Circuit without chip", ("sans-serif", 60))
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