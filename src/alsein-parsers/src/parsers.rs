use std::{marker::PhantomData, ops::Index, process::Output};

pub trait ReadList<T>: Index<usize, Output = T> {
    fn len() -> usize;
}

#[derive(Clone)]
pub struct Parser<I: Clone, O, R: ReadList<I>, F: Fn(R, usize) -> Option<(O, usize)>> {
    delegate: F,
    _phantom: PhantomData<*const (I, O, R)>,
}

impl<I: Clone, O, R: ReadList<I>, F: Fn(R, usize) -> Option<(O, usize)>> Parser<I, O, R, F> {
    pub fn parse(&self, input: R) -> Option<O> {
        (self.delegate)(input, 0).map(|x| x.0)
    }
}

pub fn single<I: PartialEq + Clone, R: ReadList<I>>(
    value: I,
) -> Parser<I, I, R, impl Fn(R, usize) -> Option<(I, usize)>> {
    Parser {
        delegate: move |input, start| {
            if value == input[start] {
                Some((value.clone(), start + 1))
            } else {
                None
            }
        },
        _phantom: PhantomData,
    }
}
