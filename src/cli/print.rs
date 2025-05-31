
macro_rules! quiz_command_prefix {
    () => ({
        use std::io::Write;
        print!("Quiz command> ");
        std::io::stdout().flush().expect("Output flush failed");
    });
}

macro_rules! quiz_print {
    ($($arg:tt)*) => ({
        println!();
        println!($($arg)*);
        quiz_command_prefix!();
    })
}