pub mod native;
pub mod protocol;
pub mod session;
pub mod worker;
pub mod workspace;

pub type Result<T> = std::result::Result<T, &'static str>;
