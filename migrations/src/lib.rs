use std::borrow::Cow;
use std::str;

use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "migrations/sql/"]
struct SqlFiles;

pub struct Migrations;

impl Migrations {
    /// Get a vector of all (<filename>, <content>) tuples migrations to run,
    /// in the order they are meant to be applied.
    pub fn get_all() -> impl Iterator<Item = (String, Cow<'static, str>)> {
        let mut file_names: Vec<String> = SqlFiles::iter().map(|s| s.into_owned()).collect();
        file_names.sort();
        file_names.into_iter().map(|filename| {
            let contents: Cow<[u8]> = SqlFiles::get(filename.as_ref())
                .expect(&format!("MIGRATION FILE ~ MISSING: {}", filename));
            let decode_failure = format!("MIGRATION FILE ~ BAD UTF-8 CHARACTERS: {}", filename);
            let text: Cow<str> = match contents {
                Cow::Borrowed(bytes) => str::from_utf8(bytes).expect(&decode_failure).into(),
                Cow::Owned(bytes) => String::from_utf8(bytes).expect(&decode_failure).into(),
            };
            (filename, text)
        })
    }
}
