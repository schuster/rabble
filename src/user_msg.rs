use std::fmt::Debug;
use errors::Result;

pub trait UserMsg: Debug + Clone + PartialEq + Send {
    fn to_bytes(self) -> Result<Vec<u8>>;
    fn from_bytes(Vec<u8>) -> Result<Self>;
}
