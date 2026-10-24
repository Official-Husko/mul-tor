//! Random tokens some protocols need (boundaries, upload ids, file names).

use rand::RngExt;

/// Random string of `len` characters drawn from `alphabet`.
pub fn random_string(len: usize, alphabet: &str) -> String {
    let chars: Vec<char> = alphabet.chars().collect();
    let mut rng = rand::rng();
    (0..len)
        .map(|_| chars[rng.random_range(0..chars.len())])
        .collect()
}

pub const LOWER_ALNUM: &str = "abcdefghijklmnopqrstuvwxyz0123456789";
pub const MIXED_ALNUM: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_strings_use_only_the_alphabet() {
        let s = random_string(64, LOWER_ALNUM);
        assert_eq!(s.len(), 64);
        assert!(s.chars().all(|c| LOWER_ALNUM.contains(c)));
    }
}
