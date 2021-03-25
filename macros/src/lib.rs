/// Useful macros live here.

/// Like Rust's built-in `try!` macro or the `?` operator, but instead of
/// returning an `Err`, this macro will return a Boxed `Future<Item = ?, Error = <your-error>>.
#[macro_export]
macro_rules! try_future {
    ($f:expr) => {
        match $f {
            Ok(t) => t,
            Err(e) => return Box::new(::futures::future::err(e.into())),
        }
    };
}

/// "Throw" an error as a Boxed Future by returning it immediately.
#[macro_export]
macro_rules! throw_future {
    ($e:expr) => {
        return Box::new(::futures::future::err::<(), _>(($e).into()));
    };
}

/// Given an iterable of things that can be turned into strings, this
/// macro will produce Vec<String>.
#[macro_export]
macro_rules! strings {
    ($args:expr) => {
        $args
            .into_iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    };
}

/// Builds a path, yielding an instance of ::std::path::PathBuf.
///
/// # Example:
///
///   path!(::std::env::temp_dir(), "foo", "bar", "baz") -> "${TEMP_DIR}/foo/bar/baz"
///
///   path!(::std::env::temp_dir(), "foo", "bar"; extension => "baz") -> "${TEMP_DIR}/foo/bar.baz"
#[macro_export]
macro_rules! path {
    ($($part:expr),+) => {
        {
            #[cfg(windows)]
            let mut _first_component = true;
            let mut temp_path = ::std::path::PathBuf::new();
            $(
                let p = ::std::path::Path::new($part);
                #[cfg(windows)]
                {
                    match p.to_str() {
                        Some(inner) => {
                            if _first_component {
                                // If the first component is a drive like "C:\", don't
                                // replace any characters.
                                temp_path.push(p)
                            } else {
                                // Pushing a component mid-path that can be considered a
                                // drive like "C:\" will cause the path to be reset such
                                // that the "drive" component will be the root of the path.
                                let sanitized = inner.replace(":", "_");
                                let q = ::std::path::Path::new(&sanitized);
                                temp_path.push(q)
                            }
                        },
                        None => {
                            panic!("path!: component [{:?}] contains invalid unicode characters", $part);
                        }
                    };
                    _first_component = false;
                };
                #[cfg(not(windows))]
                temp_path.push(p);
            )*
            temp_path
        }
    };

    ($($part:expr),+ ; extension => $extension:expr) => {
        {
            let mut temp_path = path!($($part),+);
            temp_path.set_extension($extension);
            temp_path
        }
    }
}

#[macro_export]
/// Builds a PathBuf referencing the project `src` directory.
///
/// If additional path components are provided, they will be appended to the
/// resulting path.
macro_rules! src_path {
    () => {
        path!(env!("CARGO_MANIFEST_DIR"), "src")
    };

    ($($part:expr),+) => {
        path!(env!("CARGO_MANIFEST_DIR"), "src", $($part),+)
    }
}

#[macro_export]
/// Builds a PathBuf referencing the project `test` directory.
///
/// If additional path components are provided, they will be appended to the
/// resulting path.
macro_rules! test_path {
    () => {
       path!(env!("CARGO_MANIFEST_DIR"), "tests")
    };

    ($($part:expr),+) => {
       path!(env!("CARGO_MANIFEST_DIR"), "tests", $($part),+)
    }
}

#[macro_export]
/// Builds a PathBuf referencing the project `test_resources` directory.
///
/// If additional path components are provided, they will be appended to the
/// resulting path.
macro_rules! test_resources_path {
    () => {
       path!(env!("CARGO_MANIFEST_DIR"), "test_resources")
    };

    ($($part:expr),+) => {
       path!(env!("CARGO_MANIFEST_DIR"), "test_resources", $($part),+)
    }
}

#[macro_export]
/// Builds a PathBuf referencing the project `target` directory.
///
/// If additional path components are provided, they will be appended to the
/// resulting path.
macro_rules! target_path {
    () => {
       path!(env!("CARGO_MANIFEST_DIR"), "target")
    };

    ($($part:expr),+) => {
       path!(env!("CARGO_MANIFEST_DIR"), "target", $($part),+)
    }
}

#[macro_export]
/// Builds a PathBuf referencing the project debug binary.
macro_rules! debug_binary {
    () => {
        target_path!("debug", env!("CARGO_PKG_NAME"))
    };
}

#[macro_export]
/// Builds a PathBuf referencing the project release binary.
macro_rules! release_binary {
    () => {
        target_path!("release", env!("CARGO_PKG_NAME"))
    };
}
