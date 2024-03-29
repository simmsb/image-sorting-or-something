use ndarray::prelude::*;
use ndarray::{Data, RemoveAxis, Zip};

use std::cmp::Ordering;
use std::ptr::copy_nonoverlapping;

// Type invariant: Each index appears exactly once
#[derive(Clone, Debug)]
pub struct Permutation {
    indices: Vec<usize>,
}

impl Permutation {
    /// Checks if the permutation is correct
    fn correct(&self) -> bool {
        let axis_len = self.indices.len();
        let mut seen = vec![false; axis_len];
        for &i in &self.indices {
            match seen.get_mut(i) {
                None => return false,
                Some(s) => {
                    if *s {
                        return false;
                    } else {
                        *s = true;
                    }
                }
            }
        }
        true
    }
}

pub trait SortArray {
    /// ***Panics*** if `axis` is out of bounds.
    fn identity(&self, axis: Axis) -> Permutation;
    fn sort_axis_by<F>(&self, axis: Axis, less_than: F) -> Permutation
    where
        F: FnMut(usize, usize) -> bool;
}

pub trait PermuteArray {
    type Elem;
    type Dim;
    fn permute_axis(self, axis: Axis, perm: &Permutation) -> Array<Self::Elem, Self::Dim>
    where
        Self::Elem: Clone,
        Self::Dim: RemoveAxis;
}

impl<A, S, D> SortArray for ArrayBase<S, D>
where
    S: Data<Elem = A>,
    D: Dimension,
{
    fn identity(&self, axis: Axis) -> Permutation {
        Permutation {
            indices: (0..self.len_of(axis)).collect(),
        }
    }

    fn sort_axis_by<F>(&self, axis: Axis, mut less_than: F) -> Permutation
    where
        F: FnMut(usize, usize) -> bool,
    {
        let mut perm = self.identity(axis);
        perm.indices.sort_by(move |&a, &b| {
            if less_than(a, b) {
                Ordering::Less
            } else if less_than(b, a) {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        });
        perm
    }
}

impl<A, D> PermuteArray for Array<A, D>
where
    D: Dimension,
{
    type Elem = A;
    type Dim = D;

    fn permute_axis(self, axis: Axis, perm: &Permutation) -> Array<A, D>
    where
        D: RemoveAxis,
    {
        let axis = axis;
        let axis_len = self.len_of(axis);
        assert_eq!(axis_len, perm.indices.len());
        debug_assert!(perm.correct());

        let mut v = Vec::with_capacity(self.len());
        let mut result;

        // panic-critical begin: we must not panic
        unsafe {
            v.set_len(self.len());
            result = Array::from_shape_vec_unchecked(self.dim(), v);
            for i in 0..axis_len {
                let perm_i = perm.indices[i];
                Zip::from(result.index_axis_mut(axis, perm_i))
                    .and(self.index_axis(axis, i))
                    .apply(|to, from| copy_nonoverlapping(from, to, 1));
            }
            // forget moved array elements but not its vec
            let mut old_storage = self.into_raw_vec();
            old_storage.set_len(0);
            // old_storage drops empty
        }
        // panic-critical end
        result
    }
}
