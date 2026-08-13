use eyre::Result;
use rand::{
    distr::{Alphanumeric, SampleString as _, Uniform},
    RngExt,
};
use xshell::{cmd, Shell};

pub const VAULT: &str = "CLI";

fn random_with_charset(length: usize, charset: &[u8]) -> String {
    if charset.is_empty() {
        return String::new();
    }

    let mut rng = rand::rng();
    let dist = Uniform::new(0, charset.len()).unwrap_or_else(|_| unreachable!());
    (0..length)
        .map(|_| charset[rng.sample(dist)] as char)
        .collect()
}

pub fn random_ascii(length: usize) -> String {
    random_with_charset(
        length,
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abcdefghijklmnopqrstuvwxyz!@#%^&|-_=+*",
    )
}

pub fn random_alpha(length: usize) -> String {
    random_with_charset(
        length,
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz",
    )
}

pub fn random_base32(length: usize) -> String {
    random_with_charset(length, b"23456789abcdefghjkmnopqrstuvwxyz")
}

pub fn random_pin(length: usize) -> String {
    random_with_charset(length, b"0123456789")
}

pub fn random_alpha_numeric(length: usize) -> String {
    Alphanumeric.sample_string(&mut rand::rng(), length)
}

pub fn random_alpha_lower(length: usize) -> String {
    random_with_charset(length, b"abcdefghijklmnopqrstuvwxyz")
}

pub fn random_alpha_numeric_lower(length: usize) -> String {
    random_with_charset(length, b"abcdefghijklmnopqrstuvwxyz0123456789")
}

pub fn pass_edit(
    sh: &xshell::Shell,
    secret_name: &str,
    key: &str,
    password: &str,
) -> eyre::Result<()> {
    // add password to item
    Ok(cmd!(
        sh,
        "op item edit {secret_name} {key}={password} --vault {VAULT}"
    )
    .run()?)
}

pub fn pass_read(sh: &xshell::Shell, secret_name: &str, key: &str) -> eyre::Result<String> {
    Ok(cmd!(sh, "op read op://{VAULT}/{secret_name}/{key}").read()?)
}

pub fn hex_to_rgb(hex: &str) -> Result<(f32, f32, f32), std::num::ParseIntError> {
    let hex = hex.trim_start_matches('#');
    let num = u32::from_str_radix(hex, 16)?;

    let r = (num >> 16) as u8;
    let g = (num >> 8) as u8;
    let b = num as u8;

    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;

    Ok((r, g, b))
}

pub fn has_tool(sh: &Shell, tool: &str) -> bool {
    cmd!(sh, "command -v {tool}").quiet().output().is_ok()
}

#[cfg(test)]
mod tests {
    use super::{
        hex_to_rgb, random_alpha, random_alpha_lower, random_alpha_numeric,
        random_alpha_numeric_lower, random_ascii, random_base32, random_pin,
    };

    const FLOAT_EPSILON: f32 = 1e-6;

    fn assert_rgb_close(actual: (f32, f32, f32), expected: (f32, f32, f32)) {
        assert!((actual.0 - expected.0).abs() < FLOAT_EPSILON);
        assert!((actual.1 - expected.1).abs() < FLOAT_EPSILON);
        assert!((actual.2 - expected.2).abs() < FLOAT_EPSILON);
    }

    #[test]
    fn hex_to_rgb_parses_hex_with_or_without_hash() {
        let expected = (
            0x33 as f32 / 255.0,
            0x66 as f32 / 255.0,
            0x99 as f32 / 255.0,
        );

        assert_rgb_close(hex_to_rgb("#336699").unwrap(), expected);
        assert_rgb_close(hex_to_rgb("336699").unwrap(), expected);
    }

    #[test]
    fn hex_to_rgb_parses_known_values_and_rejects_invalid_hex() {
        assert_rgb_close(hex_to_rgb("#000000").unwrap(), (0.0, 0.0, 0.0));
        assert_rgb_close(hex_to_rgb("#ffffff").unwrap(), (1.0, 1.0, 1.0));
        assert!(hex_to_rgb("#not-hex").is_err());
    }

    fn assert_random_output<F>(generate: fn(usize) -> String, valid: F)
    where
        F: Fn(char) -> bool,
    {
        for length in [32, 0] {
            let output = generate(length);

            assert_eq!(output.len(), length);
            assert!(output.chars().all(&valid));
        }
    }

    #[test]
    fn random_generators_return_expected_lengths_and_characters() {
        assert_random_output(random_alpha, |character| character.is_ascii_alphabetic());
        assert_random_output(random_alpha_lower, |character| {
            character.is_ascii_lowercase()
        });
        assert_random_output(random_alpha_numeric, |character| {
            character.is_ascii_alphanumeric()
        });
        assert_random_output(random_alpha_numeric_lower, |character| {
            character.is_ascii_lowercase() || character.is_ascii_digit()
        });
        assert_random_output(random_pin, |character| character.is_ascii_digit());

        // the charset omits 0, 1, i, and l, because they are easy to confuse
        let base32_charset = "23456789abcdefghjkmnopqrstuvwxyz";
        assert_random_output(random_base32, |character| {
            base32_charset.contains(character)
        });

        let ascii_charset =
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abcdefghijklmnopqrstuvwxyz!@#%^&|-_=+*";
        assert_random_output(random_ascii, |character| ascii_charset.contains(character));
    }
}
