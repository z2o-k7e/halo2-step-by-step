// Problem to prove: f(n) = f(n-1) + f(n-2), f(0)= a, f(1) = b

use std::marker::PhantomData;

use halo2_proofs::{
    circuit::{Layouter, AssignedCell, SimpleFloorPlanner}, 
    arithmetic::Field, 
    plonk::{Advice, Column, Selector, Instance, ConstraintSystem, Error, Circuit}, 
    poly::Rotation
};

/// Circuit design:
/// |  ins  |   a0    | seletor|
/// |-------|---------|--------|
/// |   a   | f(0)=a  |   1    | 
/// |   b   | f(1)=b  |   1    |  
/// |  out  | f(2)    |   1    |  
/// |       | f(3)    |   1    |   
/// |       |  ...    |        |
/// |       | f(n-2)  |   1    | 
/// |       | f(n-1)  |   0    |   
/// |       | f(n)=out|   0    |    

#[derive(Debug, Clone)]
struct FiboChipConfig {
    advice: Column<Advice> ,
    instance: Column<Instance>,
    selector: Selector,
}

#[derive(Debug, Clone)]
struct FiboChip<F:Field>{
    config: FiboChipConfig,
    _marker: PhantomData<F>
}

#[derive(Debug, Clone)]
struct ACell<F:Field> (AssignedCell<F,F>);

impl <F:Field> FiboChip<F> {
    fn construct(config: FiboChipConfig) -> Self {
        FiboChip {
            config,
            _marker: PhantomData,
        }
    }

    fn configure(
        meta: &mut ConstraintSystem<F>,
        advice: Column<Advice> ,
        instance: Column<Instance>,
    ) -> FiboChipConfig {
        let selector = meta.selector();
        meta.enable_equality(advice);
        meta.enable_equality(instance);

        meta.create_gate(
            "fibo gate", 
            |meta| {
                let cur_row = meta.query_advice(advice, Rotation::cur());
                let next_row = meta.query_advice(advice, Rotation::next());
                let third_row = meta.query_advice(advice, Rotation(2)); 
                let s = meta.query_selector(selector);
                vec![s * (cur_row + next_row - third_row)]
            }
        );

        FiboChipConfig {
            advice,
            instance,
            selector
        }
    }

    fn assign_witness(
        &self,
        mut layouter: impl Layouter<F>,
        nrow: usize
    ) -> Result<ACell<F>, Error> {
        layouter.assign_region(
            || "fibo", 
            |mut region| {
                let instance = self.config.instance;
                let advice = self.config.advice;
                let selector =  self.config.selector;
                selector.enable(&mut region, 0)?;
                selector.enable(&mut region, 1)?;
                let mut f_pre = region.assign_advice_from_instance(
                    || "f0",instance , 0, advice, 0).map(ACell)?;
                let mut f_cur = region.assign_advice_from_instance(
                    || "f1", instance, 1, advice, 1).map(ACell)?;
                for i in 2..nrow{
                    if i < nrow -2 {
                        selector.enable(&mut region, i)?;
                    }
                    let value = f_pre.0.value().copied() +  f_cur.0.value();
                    let f_next = region.assign_advice(
                        || "fn", advice, i, || value).map(ACell)?;
                    f_pre = f_cur;
                    f_cur = f_next;

                }
                Ok(f_cur)
            }
        )
    }
}


#[derive(Debug, Clone,Default)]
struct FiboCircuit<F:Field> {
    nrow: usize,
    _marker: PhantomData<F>
}

impl <F:Field> Circuit<F> for FiboCircuit<F> {
    type Config = FiboChipConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        FiboCircuit::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let advice = meta.advice_column();
        let instance = meta.instance_column();
        FiboChip::configure(meta, advice, instance)
        
    }
    fn synthesize(&self, config: Self::Config, mut layouter: impl Layouter<F>) -> Result<(), Error> {
        let chip = FiboChip::construct(config);
        let out = FiboChip::assign_witness(&chip, layouter.namespace(|| "fibo table"), self.nrow)?;
        //expose public
        layouter.namespace(|| "out").constrain_instance(out.0.cell(), chip.config.instance, 2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use halo2_proofs::{pasta::Fp, dev::MockProver};

    #[test]
    fn test_fibo(){
        let f0 = Fp::from(1);
        let f1 = Fp::from(1);
        let out = Fp::from(55);
        let circuit = FiboCircuit {
            nrow: 10, 
            _marker: PhantomData
        };

        let k = 4;
        let public_inputs = vec![f0, f1, out];
        let prover = MockProver::run(k, &circuit, vec![public_inputs.clone()]).unwrap();
        prover.assert_satisfied();
    }

    #[cfg(feature = "dev-graph")]
    #[test]
    fn plot_fibo_circuit(){
        // Instantiate the circuit with the private inputs.
        let circuit =  FiboCircuit::<Fp>{nrow: 10, _marker: PhantomData};
        // Create the area you want to draw on.
        // Use SVGBackend if you want to render to .svg instead.
        use plotters::prelude::*;
        let root = BitMapBackend::new("./circuit_layouter_plots/fibo_1.png", (1024, 768)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let root = root
            .titled("Fibo Circuit", ("sans-serif", 60))
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
            .render(4, &circuit, &root)
            .unwrap();
    }
}