use std::{
    marker::PhantomData,
    mem,
    ops::{Deref, Index},
};

pub trait Set: 'static {
    type Output;
    fn len(&self) -> usize;
    fn get<'a>(&'a self, idx: usize) -> &'a Self::Output;
}

impl<T: 'static> Set for [T] {
    type Output = T;

    fn len(&self) -> usize {
        self.len()
    }

    fn get<'a>(&'a self, idx: usize) -> &'a Self::Output {
        &self[idx]
    }
}

impl<S: Set + ?Sized, D: Deref<Target = S> + 'static> Set for D {
    type Output = S::Output;

    fn len(&self) -> usize {
        self.deref().len()
    }

    fn get<'a>(&'a self, idx: usize) -> &'a Self::Output {
        self.deref().get(idx)
    }
}
