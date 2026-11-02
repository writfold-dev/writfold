//! Native effects. Successful effects are constructed only after observed IO.
#![forbid(unsafe_code)]

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
compile_error!("writfold-runtime supports Linux/WSL x86_64 only");

pub mod fs;
