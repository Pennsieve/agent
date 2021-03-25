//! Collection of functions for receiving user input

use getch::Getch;
use rustyline::error::ReadlineError;
use rustyline::Editor;
use std::io::{self, Write};
use std::result::Result;

// Given a y/n prompt, get a boolean response from the user
pub fn confirm<S: Into<String>>(prompt: S) -> io::Result<bool> {
    print!("{} (y/n) ", prompt.into());
    io::stdout().flush()?;

    Getch::new().getch().map_err(Into::into).and_then(|ch| {
        let mut ch = Into::<char>::into(ch);
        ch.make_ascii_lowercase();

        println!(); // add newline
        if vec!['y', 'n'].contains(&ch) {
            Ok(ch == 'y')
        } else {
            confirm("Please answer 'y' or 'n'")
        }
    })
}

/// Get the user's response to the given prompt
pub fn user_input<S: Into<String>>(prompt: S) -> Result<String, ReadlineError> {
    Editor::<()>::new().readline(&format!("{} ", prompt.into()))
}

/// Get the user's response to the given prompt, will default to the
/// provided default value if no response is given.
pub fn user_input_with_default<S: Into<String>>(
    prompt: S,
    default: S,
) -> Result<String, ReadlineError> {
    let default: String = default.into();

    let prompt = format!("{} [{}] ", prompt.into(), default.clone());
    let result = user_input(prompt)?;

    if result.is_empty() {
        Ok(default)
    } else {
        Ok(result.to_string())
    }
}
