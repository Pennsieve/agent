//! CLI-specific types live here.

use std::fmt;

use prettytable as pt;

/// Creates a data table suitable for CLI display, a la
///
/// ```rust,ignore
/// +-------------+------------+
/// | Title 1     | Title 2    |
/// +-------------+------------+
/// | Value 1     | Value 2    |
/// | Value three | Value four |
/// +-------------+------------+
/// ```
pub struct CliTable(pt::Table);

impl fmt::Display for CliTable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

pub fn cli_table<F, S>(titles: Option<Vec<S>>, build: F) -> CliTable
where
    F: Fn(&mut pt::Table) -> (),
    S: Into<String>,
{
    let mut table = pt::Table::new();
    table.set_format(*pt::format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    if let Some(titles) = titles {
        table.set_titles(pt::Row::new(
            titles
                .into_iter()
                .map(|s| pt::Cell::new(&s.into()))
                .collect(),
        ));
    }
    build(&mut table);
    CliTable(table)
}
