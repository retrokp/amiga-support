# amiga-support: Rust replacement for amiga.lib functions

Implementation of Amiga (m68k) amiga.lib functions for Rust.

This crate is intended to be used with the [amiga-sys](https://github.com/retrokp/amiga-sys) crate.

## Features

 - implementations for amiga.lib functions
 - the functions are marked as unsafe because they handle raw pointers, just like the original
   amiga.lib functions
 - no dependency to the Amiga Native Development Kit (NDK): no dependency to the NDK headers
   or amiga.lib
 - supports `no_std` (no dependency to `std` or `alloc`)
 - only cross-compiling for Amiga (no building on Amiga)
 - extra feature: a lazy developer who doesn't respond quickly to issues or pull requests

## Not supported

 - functions with variadic arguments: there's always a similar function available
   without variadic arguments (the replacement function's name usually ends with Args, List or A)
 - alib_stdio: functions duplicating libc functionality: printf(), fgetc(), etc.
 - debug.lib and ddebug.lib functions: KGetChar(), KPrintF(), DGetChar(), DPrintF(), etc.
 - no support for AmigaOS 4.0 or other derivatives, PowerPC or other non-m68k Amiga versions

## Related

 - [amiga-sys](https://github.com/retrokp/amiga-sys): bindings to Amiga system libraries
 - [amiga-rust](https://github.com/grahambates/amiga-rust): direct access to hardware
 - [amiga-debug Visual Studio Code Extension](https://github.com/BartmanAbyss/vscode-amiga-debug/tree/master): C/C++ and build tools for Amiga

## License

Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-0BSD">0BSD license</a> at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
