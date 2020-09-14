// Encountered a fatal error
// Print error message and exit the current process
#[macro_export]
macro_rules! exit {
    ($($arg:tt)*) => {
        {
            use bright::Colorful;
            eprint!("{}", "error: ".red().bold());
            eprintln!($($arg)*);
            std::process::exit(1)
        }
    };
}
