use halo2_proofs::{arithmetic::FieldExt, circuit::*, pasta::Fp, plonk::*, poly::Rotation};
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

    advice: [Column<Advice>; 9],
    instance: [Column<Instance>; 9],
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

    pub fn configure(meta: &mut ConstraintSystem<F>) -> SudokuConfig {
        let [always_enabled, only_first_enabled] = [0; 2].map(|_| meta.selector());
        let advice = [0; 9].map(|_| meta.advice_column());
        let instance = [0; 9].map(|_| meta.instance_column());

        for adv in advice {
            meta.enable_equality(adv);
        }
        for inst in instance {
            meta.enable_equality(inst);
        }

        //  advice[0]  |   ...   |  advice[8]  | always_enabled | only_first_enabled
        //      5      |         |      7      |       1        |         1
        //      7      |         |      1      |       1        |         0
        //      1      |         |      2      |       1        |         0
        //      6      |         |      9      |       1        |         0
        //      2      |         |      3      |       1        |         0
        //      4      |         |      6      |       1        |         0
        //      3      |         |      4      |       1        |         0
        //      9      |         |      8      |       1        |         0
        //      8      |         |      5      |       1        |         0

        meta.create_gate("range check", |meta| {
            let only_first_enabled = meta.query_selector(only_first_enabled);
            let always_enabled = meta.query_selector(always_enabled);

            println!("applying range check");

            // let mut constraints = Vec::new();

            // for i in 0..9 {
            //     for j in 0..9 {
            //         let element = meta.query_advice(advice[i], Rotation(j));

            //         let range_check = |range: usize, value: Expression<F>| {
            //             (1..range).fold(value.clone(), |expr, k| {
            //                 expr * (Expression::Constant(F::from(k as u64)) - value.clone())
            //             })
            //         };

            //         constraints.push(only_first_enabled.clone() * range_check(10, element.clone()));
            //         // constraints.push(range_check(10, element.clone()));
            //     }
            // }

            // constraints
            // Constraints::with_selector(only_first_enabled, constraints)

            vec![
                only_first_enabled.clone()
                    * (Expression::Constant(F::from(5))
                        - meta.query_advice(advice[0], Rotation::cur())),
                only_first_enabled
                    * (Expression::Constant(F::from(7))
                        - meta.query_advice(advice[0], Rotation::next())),
            ]
        });

        // meta.create_gate("sudoku_column", |meta| {
        //     let only_first_enabled = meta.query_selector(only_first_enabled);

        //     meta.fi
        //     // let element = meta.query_advice(*col, Rotation(i));
        //     // for i in 0..9 {
        //     //     let a = meta.query_advice(advice[i], Rotation(0));
        //     //     println!("a: {:?}", a);
        //     // }

        //     // Compute the product of (x - number) for all numbers.
        //     let product = numbers
        //         .iter()
        //         .fold((Expression::Constant(F::one())), |acc, number| {
        //             acc * (Expression::variable(X) - number)
        //         });

        //     // The product is a polynomial of degree n if and only if the numbers are all distinct.
        //     // Therefore, the top degree coefficient of the product must be 1.
        //     let top_degree_coefficient = product.coefficients().last().unwrap();

        //     vec![
        //         only_first_enabled
        //             * (Expression::Constant(F::from(9)) - Expression::Constant(F::from(9))),
        //     ]
        // });

        SudokuConfig {
            always_enabled,
            only_first_enabled,
            advice,
            instance,
        }
    }

    pub fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        solution: &Vec<Vec<F>>,
    ) -> Result<(), Error> {
        layouter.assign_region(
            || "entire table",
            |mut region| {
                self.config.only_first_enabled.enable(&mut region, 0)?; // enable only first row
                for row in 0..9 {
                    self.config.always_enabled.enable(&mut region, row)?; // enable the whole column
                }

                // first, assign the public cells
                for row in 0..9 {
                    for col in 0..9 {
                        // if it's zero in solution, it must be public
                        if solution[row][col] != F::zero() {
                            continue;
                        }
                        region.assign_advice_from_instance(
                            || format!("copy row {} col {} from instance to advice", row, col),
                            self.config.instance[row],
                            col, // row in instance column
                            self.config.advice[row],
                            col, // row in advice column
                        )?;
                    }
                }

                // then, add the solution cells
                for row in 0..9 {
                    for col in 0..9 {
                        if solution[row][col] == F::zero() {
                            continue;
                        }
                        // println!(
                        //     "Assigning {:?} to advice column {}, row: {}",
                        //     solution[row][col], row, col,
                        // );
                        region.assign_advice(
                            || format!("copy row {} col {} from solution to advice", row, col),
                            self.config.advice[row],
                            col, // row in solution column
                            || Value::known(solution[row][col]),
                        )?;
                    }
                }
                Ok(())
            },
        )
    }
}

#[derive(Default)]
struct MyCircuit<F> {
    solution: Vec<Vec<F>>,
}

impl<F: FieldExt> Circuit<F> for MyCircuit<F> {
    type Config = SudokuConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        SudokuChip::configure(meta)
    }

    fn synthesize(&self, config: Self::Config, layouter: impl Layouter<F>) -> Result<(), Error> {
        let chip = SudokuChip::construct(config);
        chip.assign(layouter, &self.solution)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::MyCircuit;
    use halo2_proofs::{dev::MockProver, pasta::Fp};

    #[test]
    fn sudoku_example() {
        let k = 5;

        let public_grid = vec![
            vec![0, 0, 1, 0, 0, 4, 0, 9, 0],
            vec![4, 0, 0, 0, 0, 0, 1, 0, 7],
            vec![0, 8, 0, 7, 0, 0, 0, 0, 4],
            vec![9, 0, 0, 0, 1, 0, 8, 0, 0],
            vec![0, 0, 0, 8, 0, 7, 0, 0, 0],
            vec![0, 0, 8, 0, 6, 0, 0, 0, 1],
            vec![8, 0, 0, 0, 0, 5, 0, 1, 0],
            vec![6, 0, 5, 0, 0, 0, 0, 0, 9],
            vec![0, 1, 0, 9, 0, 0, 4, 0, 0],
        ];

        let solution = vec![
            vec![5, 7, 0, 6, 2, 0, 3, 0, 8],
            vec![0, 2, 6, 3, 8, 9, 0, 5, 0],
            vec![3, 0, 9, 0, 5, 1, 2, 6, 0],
            vec![0, 5, 7, 4, 0, 2, 0, 3, 6],
            vec![1, 6, 3, 0, 8, 0, 5, 4, 2],
            vec![2, 4, 0, 5, 0, 3, 9, 7, 0],
            vec![0, 9, 4, 2, 7, 0, 6, 0, 3],
            vec![0, 3, 0, 1, 4, 8, 7, 2, 0],
            vec![7, 0, 2, 0, 3, 6, 0, 8, 5],
        ];

        let mut public_input: Vec<Vec<Fp>> = convert_sudoku_to_fp(public_grid);
        let private_input = convert_sudoku_to_fp(solution);

        let circuit = MyCircuit {
            solution: private_input.clone(),
        };

        let prover = MockProver::run(k, &circuit, public_input.clone()).unwrap();
        prover.assert_satisfied();

        public_input[0][0] += Fp::one();
        let _prover = MockProver::run(k, &circuit, public_input).unwrap();
        // uncomment the following line and the assert will fail
        // _prover.assert_satisfied();
    }

    fn convert_sudoku_to_fp(sudoku: Vec<Vec<u64>>) -> Vec<Vec<Fp>> {
        sudoku
            .into_iter()
            .map(|row| row.into_iter().map(Fp::from).collect())
            .collect()
    }
}

// let mut sum = 0;

// for col in &advice[0..9] {
//     // for each column, let's check the column is valid
//     let mut num_set = HashSet::new();
//     for i in 0..9 {
//         let element = meta.query_advice(*col, Rotation(i));
//         num_set.insert(element);
//     }

//     if num_set.len() != 9 {
//         continue;
//     }

//     for i in 1..=9 {
//         if !num_set.contains(&Fp::from(i)) {
//             // break also from the outer loop
//             break;
//         }
//     }
//     sum += 1;
// }

// let mut sum: Expression<F> = Expression::Constant(F::zero());
// for i in 0..9 {
//     sum = sum + meta.query_advice(first_column, Rotation(i)); // Here increment
// }

// print!("sum: {:?}\n", sum);
// print!("total_sum: {:?}\n", total_sum);
