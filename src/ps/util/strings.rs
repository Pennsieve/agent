/// Random string utilities
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

/// Generates an alphanumeric string of the given length.
#[allow(dead_code)]
pub fn random_alphanum(length: usize) -> String {
    let mut rng = thread_rng();
    rng.sample_iter(&Alphanumeric)
        .take(length)
        .collect::<String>()
}

/// Adds a 6 character alphanumeric suffix to the input string.
#[allow(dead_code)]
pub fn random_suffix<S>(input: S) -> String
where
    S: Into<String>,
{
    format!(
        "{input}-{suffix}",
        input = input.into(),
        suffix = random_alphanum(6)
    )
}

/// Tests if the given string looks like a dataset node ID
/// (e.g. starts with "N:dataset".
pub fn looks_like_dataset_node_id(dataset_ident: &str) -> bool {
    dataset_ident.to_lowercase().starts_with("n:dataset")
}

/// Tests if the given string looks like a package node ID
/// (e.g. starts with "N:package" or "N:collection")
pub fn looks_like_package_node_id(package_ident: &str) -> bool {
    let p = package_ident.to_lowercase();
    p.starts_with("n:package") || p.starts_with("n:collection")
}
