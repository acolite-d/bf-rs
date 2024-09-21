#[cfg(all(target_arch = "x86_64", target_os = "windows"))]
include!("x86_64_windows.rs");

#[cfg(all(target_arch = "x86_64", target_os = "linux"))]
include!("x86_64_linux.rs");

// #[cfg(all(target_arch = "riscv64", target_os = "linux"))]
// include!("riscv64_linux.rs");
