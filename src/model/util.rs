pub trait OptionNaExt <T>{
    fn or_na<F>(&self, f: F) -> String
    where
        F: Fn(&T) -> String;
}

impl<T> OptionNaExt<T> for Option<T> {
    fn or_na<F>(&self, f: F) -> String
    where
        F: Fn(&T) -> String,
    {
        match self {
            Some(s) => f(s),
            None => "N/A".to_string(),
        }
    }
}
