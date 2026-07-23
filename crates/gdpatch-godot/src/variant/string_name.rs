use std::borrow::Cow;

#[derive(Debug, Clone, Ord, PartialOrd, PartialEq, Eq, Hash, Default)]
pub struct StringName(pub Cow<'static, str>);

impl From<String> for StringName {
    fn from(value: String) -> Self {
        Self(Cow::Owned(value))
    }
}
