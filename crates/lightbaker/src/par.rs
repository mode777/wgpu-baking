//! Data parallelism where the platform has threads: rayon's `par_iter` / `par_iter_mut` under `native`,
//! the same calls as plain sequential iterators otherwise (the browser has no threads without cross-origin
//! isolation, ADR-0003). Items are independent and `collect` keeps their order, so results are the same.

#[cfg(feature = "native")]
pub use rayon::prelude::*;

#[cfg(not(feature = "native"))]
pub use seq::*;

#[cfg(not(feature = "native"))]
mod seq {
    pub trait ParIter<T> {
        fn par_iter(&self) -> std::slice::Iter<'_, T>;
    }
    pub trait ParIterMut<T> {
        fn par_iter_mut(&mut self) -> std::slice::IterMut<'_, T>;
    }
    impl<T> ParIter<T> for [T] {
        fn par_iter(&self) -> std::slice::Iter<'_, T> {
            self.iter()
        }
    }
    impl<T> ParIterMut<T> for [T] {
        fn par_iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
            self.iter_mut()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_order() {
        let mut v: Vec<u32> = (0..1000).collect();
        let doubled: Vec<u32> = v.par_iter().zip(&v).map(|(a, b)| a + b).collect();
        assert_eq!(doubled, (0..1000).map(|i| 2 * i).collect::<Vec<_>>());
        let n: Vec<u32> = v.par_iter_mut().map(|x| *x + 1).collect();
        assert_eq!(n[999], 1000);
    }
}
