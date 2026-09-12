use syn::Error;

pub trait CombineSynErrors {
    fn combine(self) -> Option<syn::Error>;
}

impl<I> CombineSynErrors for I
where
    I: IntoIterator<Item = Error>,
{
    fn combine(self) -> Option<Error> {
        self.into_iter().reduce(|mut current, new| {
            Error::combine(&mut current, new);
            current
        })
    }
}
