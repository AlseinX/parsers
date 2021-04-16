use std::{
    fmt::{Debug, Display},
    marker::PhantomData,
    ops::{Add, BitOr, Deref, Not, Range},
};

use crate::pool::Pool;

mod set;
pub use set::*;

type ParserResult<O> = Result<(O, usize)>;
type Result<O> = std::result::Result<O, Error>;

#[derive(Debug)]
pub enum Error {
    Single(f64, usize),
    Add(Vec<Error>),
    Or(Vec<Error>),
    Succeed(Range<usize>),
    Hinted(Box<Error>, String),
}

impl Error {
    pub fn range(&self) -> Range<usize> {
        match self {
            &Error::Single(_, pos) => pos..pos + 1,
            Error::Add(l) => l[0].range().start..l[l.len() - 1].range().end,
            Error::Or(l) => l
                .iter()
                .max_by(|&x, &y| x.similarity().partial_cmp(&y.similarity()).unwrap())
                .unwrap()
                .range(),
            Error::Succeed(range) => range.clone(),
            Error::Hinted(inner, _) => inner.range(),
        }
    }

    pub fn similarity(&self) -> f64 {
        match self {
            &Error::Single(sim, _) => sim,
            Error::Add(l) => {
                l.iter().map(Self::similarity).sum::<f64>() / self.range().len() as f64
            }
            Error::Or(l) => l
                .iter()
                .map(Self::similarity)
                .max_by(|x, y| x.partial_cmp(y).unwrap())
                .unwrap(),
            Error::Succeed(_) => 1.0,
            Error::Hinted(inner, _) => inner.similarity(),
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Error::Hinted(_, s) = self {
            Display::fmt(s, f)
        } else {
            Debug::fmt(&self, f)
        }
    }
}

impl std::error::Error for Error {}

impl Add for Error {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Error::Add(mut l1), Error::Add(mut l2)) => Error::Add({
                l1.append(&mut l2);
                l1
            }),
            (Error::Add(mut l1), e2) => Error::Add({
                l1.push(e2);
                l1
            }),
            (e1, Error::Add(mut l2)) => Error::Add({
                l2.insert(0, e1);
                l2
            }),
            (e1, e2) => Error::Add(vec![e1, e2]),
        }
    }
}

impl BitOr for Error {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Error::Or(mut l1), Error::Or(mut l2)) => Error::Or({
                l1.append(&mut l2);
                l1
            }),
            (Error::Or(mut l1), e2) => Error::Or({
                l1.push(e2);
                l1
            }),
            (e1, Error::Or(mut l2)) => Error::Or({
                l2.insert(0, e1);
                l2
            }),
            (e1, e2) => Error::Or(vec![e1, e2]),
        }
    }
}

pub trait RawParser<I: Set + ?Sized> {
    type Output;
    fn parse(&self, input: &I, start: usize) -> ParserResult<Self::Output>;
}

impl<I: Set + ?Sized, O, F: Fn(&I, usize) -> ParserResult<O>> RawParser<I> for F {
    type Output = O;
    fn parse(&self, input: &I, start: usize) -> ParserResult<Self::Output> {
        (self)(input, start)
    }
}

pub struct Parser<'a, I: Set + ?Sized, R: RawParser<I> + ?Sized + 'a> {
    raw: &'a R,
    context: &'a ParserContext<'a>,
    _phantom: PhantomData<I>,
}

impl<'a, I: Set + ?Sized, R: RawParser<I> + ?Sized + 'a> Clone for Parser<'a, I, R> {
    fn clone(&self) -> Self {
        Parser {
            raw: self.raw,
            context: self.context,
            _phantom: PhantomData,
        }
    }
}

impl<'a, I: Set + ?Sized, R: RawParser<I> + ?Sized + 'a> Copy for Parser<'a, I, R> {}

pub struct Matcher<'a, I: Set + ?Sized, R: RawParser<I, Output = ()> + ?Sized + 'a>(
    Parser<'a, I, R>,
);

impl<'a, I: Set + ?Sized, R: RawParser<I, Output = ()> + ?Sized + 'a> Clone for Matcher<'a, I, R> {
    fn clone(&self) -> Self {
        Matcher(self.0.clone())
    }
}

impl<'a, I: Set + ?Sized, R: RawParser<I, Output = ()> + ?Sized + 'a> Copy for Matcher<'a, I, R> {}

impl<'a, I: Set, R: RawParser<I, Output = ()>> Deref for Matcher<'a, I, R> {
    type Target = Parser<'a, I, R>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub type ParserDyn<'a, I, O> = Parser<'a, I, dyn RawParser<I, Output = O> + 'a>;

impl<'a, I: Set + ?Sized, R: RawParser<I> + ?Sized> Parser<'a, I, R> {
    pub fn parse(&self, input: &I) -> Result<<R as RawParser<I>>::Output> {
        Ok(self.raw.parse(&input, 0)?.0)
    }

    pub fn map<T>(
        self,
        f: impl Fn(<R as RawParser<I>>::Output) -> T,
    ) -> Parser<'a, I, impl RawParser<I, Output = T> + 'a> {
        self.context.new_parser(move |input: &I, start| {
            self.raw.parse(input, start).map(|(v, end)| (f(v), end))
        })
    }
}

#[derive(Clone, Copy)]
pub struct Discard<'a, I: Set + ?Sized, R: RawParser<I> + ?Sized + 'a>(Parser<'a, I, R>);

impl<'a, I: Set + ?Sized, R: RawParser<I> + ?Sized> Not for Parser<'a, I, R> {
    type Output = Matcher<'a, I, Discard<'a, I, R>>;

    fn not(self) -> Self::Output {
        Matcher(self.context.new_parser(Discard(self)))
    }
}

impl<'a, I: Set + ?Sized, R: RawParser<I> + ?Sized + 'a> RawParser<I> for Discard<'a, I, R> {
    type Output = ();
    fn parse(&self, input: &I, start: usize) -> ParserResult<Self::Output> {
        self.0.raw.parse(input, start).map(|(_, end)| ((), end))
    }
}

impl<'a, O, I: Set, R: RawParser<I, Output = O>> Parser<'a, I, R> {
    pub fn into_dyn(self) -> ParserDyn<'a, I, O> {
        Parser {
            raw: self.raw,
            context: self.context,
            _phantom: PhantomData,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Or<
    'a,
    I: Set + ?Sized,
    O,
    R1: RawParser<I, Output = O> + ?Sized + 'a,
    R2: RawParser<I, Output = O> + ?Sized + 'a,
>(Parser<'a, I, R1>, Parser<'a, I, R2>);

impl<
        'a,
        O: 'a,
        I: Set + ?Sized,
        R1: RawParser<I, Output = O> + ?Sized + 'a,
        R2: RawParser<I, Output = O> + ?Sized + 'a,
    > BitOr<Parser<'a, I, R2>> for Parser<'a, I, R1>
{
    type Output = Parser<'a, I, Or<'a, I, O, R1, R2>>;

    fn bitor(self, rhs: Parser<'a, I, R2>) -> Self::Output {
        self.context.new_parser(Or(self, rhs))
    }
}

impl<
        'a,
        I: Set + ?Sized,
        O,
        R1: RawParser<I, Output = O> + ?Sized + 'a,
        R2: RawParser<I, Output = O> + ?Sized + 'a,
    > RawParser<I> for Or<'a, I, O, R1, R2>
{
    type Output = O;
    fn parse(&self, input: &I, start: usize) -> ParserResult<Self::Output> {
        match self.0.raw.parse(input, start) {
            Ok(r) => Ok(r),
            Err(e1) => match self.1.raw.parse(input, start) {
                Ok(r) => Ok(r),
                Err(e2) => Err(e1 | e2),
            },
        }
    }
}
#[derive(Clone, Copy)]
pub struct AddPP<
    'a,
    I: Set + ?Sized,
    R1: RawParser<I> + ?Sized + 'a,
    R2: RawParser<I> + ?Sized + 'a,
>(Parser<'a, I, R1>, Parser<'a, I, R2>);

impl<'a, I: Set + ?Sized, R1: RawParser<I> + ?Sized + 'a, R2: RawParser<I> + ?Sized + 'a>
    Add<Parser<'a, I, R2>> for Parser<'a, I, R1>
{
    type Output = Parser<'a, I, AddPP<'a, I, R1, R2>>;

    fn add(self, rhs: Parser<'a, I, R2>) -> Self::Output {
        self.context.new_parser(AddPP(self, rhs))
    }
}

impl<'a, I: Set + ?Sized, R1: RawParser<I> + ?Sized + 'a, R2: RawParser<I> + ?Sized + 'a>
    RawParser<I> for AddPP<'a, I, R1, R2>
{
    type Output = (R1::Output, R2::Output);
    fn parse(&self, input: &I, start: usize) -> ParserResult<Self::Output> {
        match self.0.raw.parse(input, start) {
            Ok((r1, end1)) => match self.1.raw.parse(input, end1) {
                Ok((r2, end2)) => Ok(((r1, r2), end2)),
                Err(e2) => Err(Error::Succeed(start..end1) + e2),
            },
            Err(e1) => {
                let start = e1.range().end;
                match self.1.raw.parse(input, start) {
                    Ok((_, end)) => Err(e1 + Error::Succeed(start..end)),
                    Err(e2) => Err(e1 + e2),
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
pub struct AddPM<
    'a,
    I: Set + ?Sized,
    R1: RawParser<I> + ?Sized + 'a,
    R2: RawParser<I, Output = ()> + ?Sized + 'a,
>(AddPP<'a, I, R1, R2>);

impl<
        'a,
        I: Set + ?Sized,
        R1: RawParser<I> + ?Sized + 'a,
        R2: RawParser<I, Output = ()> + ?Sized + 'a,
    > Add<Matcher<'a, I, R2>> for Parser<'a, I, R1>
{
    type Output = Parser<'a, I, AddPM<'a, I, R1, R2>>;

    fn add(self, rhs: Matcher<'a, I, R2>) -> Self::Output {
        self.context.new_parser(AddPM(AddPP(self, rhs.0)))
    }
}

impl<
        'a,
        I: Set + ?Sized,
        R1: RawParser<I> + ?Sized + 'a,
        R2: RawParser<I, Output = ()> + ?Sized + 'a,
    > RawParser<I> for AddPM<'a, I, R1, R2>
{
    type Output = R1::Output;
    fn parse(&self, input: &I, start: usize) -> ParserResult<Self::Output> {
        self.0
            .parse(input, start)
            .map(|((result, _), end)| (result, end))
    }
}

#[derive(Clone, Copy)]
pub struct AddMP<
    'a,
    I: Set + ?Sized,
    R1: RawParser<I, Output = ()> + ?Sized + 'a,
    R2: RawParser<I> + ?Sized + 'a,
>(AddPP<'a, I, R1, R2>);

impl<
        'a,
        I: Set + ?Sized,
        R1: RawParser<I, Output = ()> + ?Sized + 'a,
        R2: RawParser<I> + ?Sized + 'a,
    > Add<Parser<'a, I, R2>> for Matcher<'a, I, R1>
{
    type Output = Parser<'a, I, AddMP<'a, I, R1, R2>>;

    fn add(self, rhs: Parser<'a, I, R2>) -> Self::Output {
        self.0.context.new_parser(AddMP(AddPP(self.0, rhs)))
    }
}

impl<
        'a,
        I: Set + ?Sized,
        R1: RawParser<I, Output = ()> + ?Sized + 'a,
        R2: RawParser<I> + ?Sized + 'a,
    > RawParser<I> for AddMP<'a, I, R1, R2>
{
    type Output = R2::Output;
    fn parse(&self, input: &I, start: usize) -> ParserResult<Self::Output> {
        self.0
            .parse(input, start)
            .map(|((_, result), end)| (result, end))
    }
}
pub struct ParserContext<'a> {
    pool: Pool<'a>,
}

impl Default for ParserContext<'_> {
    fn default() -> Self {
        Self { pool: Pool::new() }
    }
}

impl<'a> ParserContext<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_parser<I: Set + ?Sized, R: RawParser<I> + 'a>(&'a self, raw: R) -> Parser<'a, I, R> {
        Parser::<'a, I, R> {
            raw: self.pool.add(Box::new(raw)),
            context: self,
            _phantom: PhantomData,
        }
    }

    pub fn single<E: PartialEq + Clone, I: Set<Output = E>>(
        &self,
        value: E,
    ) -> Parser<I, impl RawParser<I, Output = E>> {
        self.new_parser(move |input: &I, start| {
            if &value == input.get(start) {
                Ok((value.clone(), start + 1))
            } else {
                Err(Error::Single(1.0, start))
            }
        })
    }
}

#[allow(dead_code)]
#[allow(unused_variables)]
mod test {
    use std::marker::PhantomData;

    use super::{Parser, ParserContext, RawParser, Set};

    #[derive(Default)]
    struct TestParser<'a, I: Set<Output = char>> {
        context: ParserContext<'a>,
        _phantom: PhantomData<I>,
    }

    impl<I: Set<Output = char>> TestParser<'_, I> {
        fn a(&self) -> Parser<I, impl RawParser<I, Output = char>> {
            self.context.single('a')
        }
    }

    pub fn _test() {
        let chars = "abcd".chars().collect::<Vec<_>>();
        let parser = TestParser {
            context: ParserContext::new(),
            _phantom: PhantomData,
        };
        let a = parser.a();
        let b = parser.a();
        let c = a + !b;
        let d = c.into_dyn();
        let x = c.parse(&chars).unwrap();
    }
}
