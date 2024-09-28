// MIT License
//
// Copyright (c) 2024 Mikael Forsberg (github.com/mkforsb)

#![expect(dead_code)]

use std::ops::{ControlFlow, RangeInclusive};

use bolero::ValueGenerator;

#[derive(Debug)]
pub struct WeightedUsizeGenerator {
    bounds: RangeInclusive<usize>,
    weight_fn: fn(usize) -> usize,
}

impl ValueGenerator for WeightedUsizeGenerator {
    type Output = usize;

    fn generate<D: bolero::Driver>(&self, driver: &mut D) -> Option<Self::Output> {
        let weights = (*self.bounds.start()..=*self.bounds.end())
            .map(self.weight_fn)
            .collect::<Vec<_>>();

        let random_offset = driver.gen_usize(
            std::ops::Bound::Included(&0),
            std::ops::Bound::Included(&(weights.iter().sum::<usize>() - 1)),
        )?;

        match weights.iter().try_fold((0usize, 0usize), |acc, x| {
            assert!(*x > 0, "weight_fn(x) must be greater than zero for all x");
            let (index, cumulative) = acc;

            if cumulative + x > random_offset {
                ControlFlow::Break(index)
            } else {
                ControlFlow::Continue((index + 1, cumulative + x))
            }
        }) {
            ControlFlow::Break(index) => Some(index + *self.bounds.start()),
            ControlFlow::Continue(_) => unreachable!(),
        }
    }
}

impl WeightedUsizeGenerator {
    pub fn new() -> Self {
        Self {
            bounds: 0..=usize::MAX,
            weight_fn: |_| 1,
        }
    }

    pub fn bounds(self, bounds: RangeInclusive<usize>) -> Self {
        Self { bounds, ..self }
    }

    pub fn weight_fn(self, weights: fn(usize) -> usize) -> Self {
        Self {
            weight_fn: weights,
            ..self
        }
    }
}
