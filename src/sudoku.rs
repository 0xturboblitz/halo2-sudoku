use halo2_proofs::{arithmetic::FieldExt, circuit::*, plonk::*, poly::Rotation};
use std::marker::PhantomData;

// Fixed : fixed in the circuit
// Advice : witness values
// Instance : public inputs

#[derive(Debug, Clone)]
struct ACell<F: FieldExt>(AssignedCell<F, F>);

#[derive(Debug, Clone)]
struct SudokuConfig {
    always_enabled: Selector,
    only_first_enabled: Selector,
    first_column: Column<Advice>,
    instance: Column<Instance>,
}

#[derive(Debug, Clone)]
struct SudokuChip<F: FieldExt> {
    config: SudokuConfig,
    _marker: PhantomData<F>,
}

impl<F: FieldExt> SudokuChip<F> {
    pub fn construct(config: SudokuConfig) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    pub fn configure(
        meta: &mut ConstraintSystem<F>,
        first_column: Column<Advice>,
        instance: Column<Instance>,
    ) -> SudokuConfig {
        let [always_enabled, only_first_enabled] = [0; 2].map(|_| meta.selector());

        meta.enable_equality(first_column);
        meta.enable_equality(instance);

        meta.create_gate("sudoku_column", |meta| {
            //  first_column  | only_first_enabled
            //       1        |         1
            //       2        |
            //       3        |
            //       4        |
            //       5        |
            //       6        |
            //       7        |
            //       8        |
            //       9        |
            let only_first_enabled = meta.query_selector(only_first_enabled);
            let total_sum = Expression::Constant(F::from(45u64)); // the sum must be 45

            let a = meta.query_advice(first_column, Rotation::cur());
            let b = meta.query_advice(first_column, Rotation::next());
            let c = meta.query_advice(first_column, Rotation(2));
            let d = meta.query_advice(first_column, Rotation(3));
            let e = meta.query_advice(first_column, Rotation(4));
            let f = meta.query_advice(first_column, Rotation(5));
            let g = meta.query_advice(first_column, Rotation(6));
            let h = meta.query_advice(first_column, Rotation(7));
            let i = meta.query_advice(first_column, Rotation(8));

            print!("a: {:?}\n", a);
            print!("e: {:?}\n", e);
            print!("i: {:?}\n", i);
            print!("total_sum: {:?}\n", total_sum);
            vec![only_first_enabled * (a + b + c + d + e + f + g + h + i - total_sum)]

            // let mut sum: Expression<F> = Expression::Constant(F::zero());
            // for i in 0..9 {
            //     sum = sum + meta.query_advice(first_column, Rotation(i)); // Here increment
            // }

            // print!("sum: {:?}\n", sum);
            // print!("total_sum: {:?}\n", total_sum);

            // vec![only_first_enabled * (sum - total_sum)]
        });

        SudokuConfig {
            always_enabled,
            only_first_enabled,
            first_column,
            instance,
        }
    }

    pub fn assign(&self, mut layouter: impl Layouter<F>) -> Result<(), Error> {
        layouter.assign_region(
            || "entire table",
            |mut region| {
                self.config.only_first_enabled.enable(&mut region, 0)?; // enable the whole row

                // assign the whole advice column (first_column) with the values of instance column
                for row in 0..9 {
                    region.assign_advice_from_instance(
                        || format!("copy row {} from instance to advice", row),
                        self.config.instance,
                        row, // row in instance column
                        self.config.first_column,
                        row, // row in advice column
                    )?;
                }

                Ok(())
            },
        )
    }

    // pub fn expose_public(
    //     &self,
    //     mut layouter: impl Layouter<F>,
    //     cell: AssignedCell<F, F>,
    //     row: usize,
    // ) -> Result<(), Error> {
    //     layouter.constrain_instance(cell.cell(), self.config.instance, row)
    // }
}

#[derive(Default)]
struct MyCircuit<F>(PhantomData<F>);

impl<F: FieldExt> Circuit<F> for MyCircuit<F> {
    type Config = SudokuConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let first_column = meta.advice_column();
        let instance = meta.instance_column();

        print!("first_column: {:?}\n", first_column);
        print!("instance: {:?}\n", instance);

        SudokuChip::configure(meta, first_column, instance)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let chip = SudokuChip::construct(config);

        chip.assign(layouter.namespace(|| "entire table"))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::MyCircuit;
    use halo2_proofs::{dev::MockProver, pasta::Fp};
    use std::marker::PhantomData;

    #[test]
    fn sudoku_example() {
        let k = 5;

        let numbers = vec![1, 2, 3, 4, 5, 6, 7, 8, 9];
        let first_column: Vec<Fp> = numbers.into_iter().map(Fp::from).collect();

        let circuit = MyCircuit(PhantomData);

        let mut public_input = first_column;

        let prover = MockProver::run(k, &circuit, vec![public_input.clone()]).unwrap();
        prover.assert_satisfied();

        // public_input[1] += Fp::one();
        // let _prover = MockProver::run(k, &circuit, vec![public_input]).unwrap();
        // uncomment the following line and the assert will fail
        // _prover.assert_satisfied();
    }
}
