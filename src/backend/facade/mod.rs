#![cfg(all(not(target_os = "linux"), not(target_os = "windows")))]

// TODO: Provide a facade.
//#[cfg(not(feature = "platform-facade"))]
compile_error!("Platform is not supported.");
