use std::ops::Deref;

use crate::gc::ObjectTrait;

// In future have multiple representations (like rope) ?
pub type NString = smartstring::SmartString<smartstring::LazyCompact>;

unsafe impl ObjectTrait for NString {}

pub struct NSymbol(NString);

impl NSymbol {
    pub fn to_string(&self) -> &NString {
        &self.0
    }
    pub fn from_string(s: NString) -> Self {
        Self(s)
    }
}

unsafe impl ObjectTrait for NSymbol {}
