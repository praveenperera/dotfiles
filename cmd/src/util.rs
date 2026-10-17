use eyre::{eyre, Result};
use rand::{
    distr::{Alphanumeric, SampleString as _, Uniform},
    RngExt,
};
use std::{fs, os::unix::fs::PermissionsExt as _};

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

/// Creates a Secure Note in the CLI vault with the given fields
///
/// Field values go through a JSON template on stdin because `op` documents argv as visible
/// to other processes
pub fn pass_create(sh: &Shell, secret_name: &str, fields: &[(&str, &str)]) -> Result<()> {
    let template = serde_json::json!({
        "title": secret_name,
        "category": "SECURE_NOTE",
        "fields": fields
            .iter()
            .map(|(label, value)| concealed_field(label, value))
            .collect::<Vec<_>>(),
    });

    cmd!(sh, "op item create --vault {VAULT} -")
        .stdin(template.to_string())
        .run()?;

    Ok(())
}

/// Sets a field on an existing item, adding it if it does not exist
///
/// Uses the documented `op item get | op item edit` template flow so the value is passed on
/// stdin instead of argv, where other processes could read it
pub fn pass_edit(sh: &Shell, secret_name: &str, label: &str, value: &str) -> Result<()> {
    let item = cmd!(
        sh,
        "op item get {secret_name} --vault {VAULT} --format=json"
    )
    .quiet()
    .read()?;

    let mut item: serde_json::Value = serde_json::from_str(&item)?;
    set_item_field(&mut item, label, value)?;

    cmd!(sh, "op item edit {secret_name} --vault {VAULT}")
        .stdin(item.to_string())
        .run()?;

    Ok(())
}

fn concealed_field(label: &str, value: &str) -> serde_json::Value {
    serde_json::json!({ "label": label, "type": "CONCEALED", "value": value })
}

fn set_item_field(item: &mut serde_json::Value, label: &str, value: &str) -> Result<()> {
    let fields = item
        .as_object_mut()
        .ok_or_else(|| eyre!("op item JSON is not an object"))?
        .entry("fields")
        .or_insert_with(|| serde_json::json!([]))
        .as_array_mut()
        .ok_or_else(|| eyre!("op item JSON `fields` is not an array"))?;

    let matching = fields
        .iter()
        .enumerate()
        .filter(|(_, field)| field["label"] == label)
        .map(|(index, _)| index)
        .collect::<Vec<_>>();

    match matching.as_slice() {
        [] => fields.push(concealed_field(label, value)),
        [index] => fields[*index]["value"] = serde_json::Value::from(value),
        _ => return Err(eyre!("item has more than one field labelled {label}")),
    }

    Ok(())
}

pub fn pass_read(sh: &xshell::Shell, secret_name: &str, key: &str) -> eyre::Result<String> {
    Ok(cmd!(sh, "op read op://{VAULT}/{secret_name}/{key}").read()?)
}

/// Parses `#rrggbb` or `#rgb`, with the `#` optional, into 0.0-1.0 components
pub fn hex_to_rgb(hex: &str) -> Result<(f32, f32, f32)> {
    let digits = hex.strip_prefix('#').unwrap_or(hex);
    if !digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(eyre!("invalid hex color {hex:?}: expected hex digits"));
    }

    let digits = match digits.len() {
        6 => digits.to_string(),
        3 => digits.chars().flat_map(|digit| [digit, digit]).collect(),
        _ => {
            return Err(eyre!(
                "invalid hex color {hex:?}: expected 3 or 6 hex digits"
            ))
        }
    };

    let num = u32::from_str_radix(&digits, 16)?;

    let r = (num >> 16) as u8;
    let g = (num >> 8) as u8;
    let b = num as u8;

    let r = r as f32 / 255.0;
    let g = g as f32 / 255.0;
    let b = b as f32 / 255.0;

    Ok((r, g, b))
}

/// Whether `tool` is an executable on the shell's PATH
///
/// Searches PATH directly because `command` is a shell builtin, so spawning `command -v`
/// fails on systems without a `/usr/bin/command` wrapper (e.g. Debian and Ubuntu)
pub fn has_tool(sh: &Shell, tool: &str) -> bool {
    let Some(path) = sh.var_os("PATH") else {
        return false;
    };

    std::env::split_paths(&path).any(|dir| {
        fs::metadata(dir.join(tool))
            .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
    })
}

#[cfg(test)]
mod tests {
    use std::{fs, os::unix::fs::PermissionsExt as _};

    use super::{
        has_tool, hex_to_rgb, random_alpha, random_alpha_lower, random_alpha_numeric,
        random_alpha_numeric_lower, random_ascii, random_base32, random_pin, set_item_field,
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

    #[test]
    fn has_tool_finds_only_executables_on_the_shell_path() {
        let bin = tempfile::tempdir().unwrap();
        fs::write(bin.path().join("present"), "").unwrap();
        fs::set_permissions(
            bin.path().join("present"),
            fs::Permissions::from_mode(0o755),
        )
        .unwrap();
        fs::write(bin.path().join("not-executable"), "").unwrap();

        let sh = xshell::Shell::new().unwrap();
        sh.set_var("PATH", bin.path());

        assert!(has_tool(&sh, "present"));
        assert!(!has_tool(&sh, "not-executable"));
        assert!(!has_tool(&sh, "missing"));
    }

    #[test]
    fn hex_to_rgb_expands_shorthand() {
        assert_rgb_close(hex_to_rgb("#f80").unwrap(), hex_to_rgb("#ff8800").unwrap());
        assert_rgb_close(hex_to_rgb("369").unwrap(), hex_to_rgb("336699").unwrap());
    }

    #[test]
    fn hex_to_rgb_rejects_wrong_lengths() {
        for hex in [
            "",
            "#",
            "#f",
            "#ffff",
            "#fffff",
            "#fffffff",
            "#ffffffff",
            "##ffffff",
            "+fffff",
        ] {
            assert!(hex_to_rgb(hex).is_err(), "{hex:?} should be rejected");
        }
    }

    #[test]
    fn set_item_field_updates_in_place_or_appends_and_keeps_other_fields() {
        let mut item = serde_json::json!({
            "title": "cmd_secrets",
            "fields": [
                { "id": "notesPlain", "label": "notesPlain", "type": "STRING", "value": "" },
                { "id": "abc", "label": "ln.yaml", "type": "STRING", "value": "old" },
            ],
        });

        set_item_field(&mut item, "ln.yaml", "new").unwrap();
        set_item_field(&mut item, "sq.yaml", "added").unwrap();

        let fields = item["fields"].as_array().unwrap();
        assert_eq!(fields.len(), 3);
        assert_eq!(fields[1]["id"], "abc");
        assert_eq!(fields[1]["type"], "STRING");
        assert_eq!(fields[1]["value"], "new");
        assert_eq!(fields[2]["label"], "sq.yaml");
        assert_eq!(fields[2]["value"], "added");
        assert_eq!(item["title"], "cmd_secrets");
    }

    fn assert_random_output(generate: impl Fn(usize) -> String, valid: impl Fn(char) -> bool) {
        for length in [32, 0] {
            let output = generate(length);

            assert_eq!(output.len(), length);
            assert!(output.chars().all(&valid));
        }
    }

    #[test]
    fn random_alpha_returns_expected_lengths_and_characters() {
        assert_random_output(random_alpha, |character| character.is_ascii_alphabetic());
    }

    #[test]
    fn random_alpha_lower_returns_expected_lengths_and_characters() {
        assert_random_output(random_alpha_lower, |character| {
            character.is_ascii_lowercase()
        });
    }

    #[test]
    fn random_alpha_numeric_returns_expected_lengths_and_characters() {
        assert_random_output(random_alpha_numeric, |character| {
            character.is_ascii_alphanumeric()
        });
    }

    #[test]
    fn random_alpha_numeric_lower_returns_expected_lengths_and_characters() {
        assert_random_output(random_alpha_numeric_lower, |character| {
            character.is_ascii_lowercase() || character.is_ascii_digit()
        });
    }

    #[test]
    fn random_pin_returns_expected_lengths_and_characters() {
        assert_random_output(random_pin, |character| character.is_ascii_digit());
    }

    #[test]
    fn random_base32_returns_expected_lengths_and_characters() {
        // the charset omits 0, 1, i, and l, because they are easy to confuse
        let base32_charset = "23456789abcdefghjkmnopqrstuvwxyz";
        assert_random_output(random_base32, |character| {
            base32_charset.contains(character)
        });
    }

    #[test]
    fn random_ascii_returns_expected_lengths_and_characters() {
        let ascii_charset =
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abcdefghijklmnopqrstuvwxyz!@#%^&|-_=+*";
        assert_random_output(random_ascii, |character| ascii_charset.contains(character));
    }
}
