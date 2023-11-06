/// This helper uses a lookup table to check that the value witnessed in a given cell is
/// within a given range.
///
/// The lookup table is tagged by `num_bits` to give a strict range check.
///
/// ------------------
/// | private inputs |
/// ------------------
/// | value |  bit   | q_lookup  | table_n_bits | table_value |
/// -----------------------------------------------------------
/// |  v_0  |   0    |    0      |       1      |      0      |
/// |  v_1  |   1    |    1      |       1      |      1      |
/// |  ...  |  ...   |   1       |       2      |      2      |
/// |  ...  |  ...   |   1       |       2      |      3      |
/// |  ...  |  ...   |   1       |       3      |      4      |
/// |  ...  |  ...   |   1       |       3      |      5      |
/// |  ...  |  ...   |   1       |       3      |      6      |
/// |  ...  |  ...   |   ...     |       3      |      7      |
/// |  ...  |  ...   |   ...     |       4      |      8      |
/// |  ...  |  ...   |   ...     |      ...     |     ...     |
/// 
/// We use a K-bit lookup table, that is tagged 1..=K, where the tag `i` marks an `i`-bit value.
///
use halo2_proofs::{circuit::*, pasta::group::ff::PrimeField, plonk::*, poly::Rotation};

use super::table_3::*;

#[derive(Debug, Clone)]
struct RangeCheckConfig<F: PrimeField, const NUM_BITS: usize, const RANGE: usize> {
    value: Column<Advice>,
    bit: Column<Advice>,
    q_lookup: Selector,
    table: RangeCheckTable<F, NUM_BITS, RANGE>,
}

impl<F: PrimeField, const NUM_BITS: usize, const RANGE: usize>
    RangeCheckConfig<F, NUM_BITS, RANGE>
{
    fn configure(meta: &mut ConstraintSystem<F>) -> Self {
        //when to configure the colum, during config or circuit instance: configure time
        let value = meta.advice_column();
        let bit = meta.advice_column();
        let q_lookup = meta.complex_selector();
        let table = RangeCheckTable::configure(meta);

        meta.lookup(|meta| {
            let default_value = Expression::Constant(F::ZERO);
            let default_bit = Expression::Constant(F::ONE);
            let mut v = meta.query_advice(value, Rotation::cur());
            let mut b = meta.query_advice(bit, Rotation::cur());
            let q = meta.query_selector(q_lookup);
            let non_q = Expression::Constant(F::ONE) - q.clone();
            v = v * q.clone() + non_q.clone() * default_value;
            b = b * q + non_q * default_bit;
            vec![(b, table.n_bits), (v, table.value)]
        });

        RangeCheckConfig {
            value,
            bit,
            q_lookup,
            table,
        }
    }

    fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        bits: Vec<Value<F>>,
        values: &Vec<Value<Assigned<F>>>,
    ) -> Result<(), Error> {
        layouter.assign_region(
            || "bit&value region",
            |mut region| {
                for i in 0..bits.len() {
                    self.q_lookup.enable(&mut region, i)?;
                    region.assign_advice(|| "bit", self.bit, i, || bits[i])?;
                    region.assign_advice(|| "value", self.value, i, || values[i])?;
                }
                Ok(())
            },
        )
    }

    fn assign_table(&self, layouter: impl Layouter<F>) -> Result<(), Error> {
        self.table.load(layouter)
    }
}

#[derive(Debug, Default)]
struct MyCircuit<F: PrimeField, const NUM_BITS: usize, const RANGE: usize> {
    num_bits: Vec<u8>,
    values: Vec<Value<Assigned<F>>>,
}

impl<F: PrimeField, const NUM_BITS: usize, const RANGE: usize> Circuit<F>
    for MyCircuit<F, NUM_BITS, RANGE>
{
    type Config = RangeCheckConfig<F, NUM_BITS, RANGE>;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        MyCircuit::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        RangeCheckConfig::configure(meta)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        config.assign_table(layouter.namespace(|| "table"))?;
        let bits = self
            .num_bits
            .iter()
            .map(|v| Value::known(F::from(*v as u64)))
            .collect();
        config.assign(layouter.namespace(|| "value"), bits, &self.values)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use halo2_proofs::{dev::MockProver, pasta::Fp};

    use super::*;

    fn circuit() -> MyCircuit<Fp, 4, 15> {
        const NUM_BITS: usize = 4;
        let mut num_bits: Vec<u8> = vec![];
        let mut values: Vec<Value<Assigned<Fp>>> = vec![];
        for num_bit in 1u8..=NUM_BITS.try_into().unwrap() {
            for value in 1 << (num_bit - 1)..1 << num_bit {
                println!("value:{:?}, {:?}", num_bit, value);
                values.push(Value::known(Fp::from(value)).into());
                num_bits.push(num_bit);
            }
        }

        MyCircuit::<Fp, NUM_BITS, 15> { num_bits, values }
    }

    #[test]
    fn test_multi_cols_rangecheck_lookup() {
        let k = 5;
        let circuit = circuit();
        let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        prover.assert_satisfied();
    }

    #[cfg(feature = "dev-graph")]
    #[test]
    fn plot_multi_cols_rangecheck_lookup() {
        // Instantiate the circuit with the private inputs.
        let k = 4;
        let circuit = circuit();
        // Create the area you want to draw on.
        // Use SVGBackend if you want to render to .svg instead.
        use plotters::prelude::*;
        let root = BitMapBackend::new("./circuit_layouter_plots/chap_4_multi_cols_rangecheck_lookup.png", (1024, 768))
            .into_drawing_area();
        root.fill(&WHITE).unwrap();
        let root = root.titled("Lookup2 Circuit", ("sans-serif", 60)).unwrap();

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
