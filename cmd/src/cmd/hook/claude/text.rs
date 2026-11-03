//! Line-based matchers for the checks that read raw command text instead of
//! the commands it runs, such as opt-in comments
//!
//! Each matches within one line, as the `grep` patterns they replace did

/// Whitespace within a line, as `[[:space:]]` matched in a `grep` line
fn is_space(c: char) -> bool {
    c != '\n' && c.is_whitespace()
}

fn trim_space(text: &str) -> &str {
    text.trim_start_matches(is_space)
}

/// Whether `text` holds an opt-in marker such as `# foreground-wait: <why>`,
/// given as `foreground-wait:`
pub fn has_marker(text: &str, marker: &str) -> bool {
    text.lines().any(|line| {
        line.match_indices('#')
            .any(|(at, _)| trim_space(&line[at + 1..]).starts_with(marker))
    })
}

/// Whether `text` mentions `homebased wait` anywhere, as a word followed by
/// whitespace or the end of a line
pub fn mentions_homebased_wait(text: &str) -> bool {
    text.lines().any(|line| {
        line.match_indices("homebased")
            .any(|(at, found)| wait_follows(&line[at + found.len()..], true))
    })
}

/// Whether a line of `text` starts with `homebased wait` and an argument
pub fn starts_line_with_homebased_wait(text: &str) -> bool {
    text.lines().any(|line| {
        trim_space(line)
            .strip_prefix("homebased")
            .is_some_and(|rest| wait_follows(rest, false))
    })
}

/// Whether `rest`, the text after `homebased`, is whitespace and then `wait`
/// followed by whitespace, or by the end of the line when `at_end` allows it
fn wait_follows(rest: &str, at_end: bool) -> bool {
    let after_space = trim_space(rest);
    if after_space.len() == rest.len() {
        return false;
    }

    match after_space.strip_prefix("wait") {
        Some("") => at_end,
        Some(after) => after.starts_with(is_space),
        None => false,
    }
}

/// Whether `text` redirects as `2>&1 > file`, which sends stderr to where stdout
/// pointed before, not to the file
pub fn redirects_stderr_first(text: &str) -> bool {
    text.lines().any(|line| {
        line.match_indices("2>&1").any(|(at, found)| {
            let rest = trim_space(&line[at + found.len()..]);
            rest.strip_prefix('>')
                .map(trim_space)
                .and_then(|target| target.chars().next())
                .is_some_and(|c| c != '&' && !is_space(c))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::{
        has_marker, mentions_homebased_wait, redirects_stderr_first,
        starts_line_with_homebased_wait,
    };

    #[test]
    fn matches_like_the_grep_patterns() {
        assert!(has_marker("x # foreground-wait: y", "foreground-wait:"));
        assert!(has_marker("x #foreground-wait:", "foreground-wait:"));
        assert!(has_marker("a\nb #\t long-lived: c", "long-lived:"));
        assert!(!has_marker("x # foreground-wait y", "foreground-wait:"));
        assert!(!has_marker("x #\nforeground-wait:", "foreground-wait:"));

        assert!(mentions_homebased_wait("homebased wait --next a"));
        assert!(mentions_homebased_wait("cd x && homebased  wait"));
        assert!(!mentions_homebased_wait("homebased waiter"));
        assert!(!mentions_homebased_wait("homebasedwait x"));
        assert!(!mentions_homebased_wait("homebased\nwait x"));

        assert!(starts_line_with_homebased_wait(
            "  homebased   wait --next a"
        ));
        assert!(starts_line_with_homebased_wait("cd x\nhomebased wait a"));
        assert!(!starts_line_with_homebased_wait("homebased wait"));
        assert!(!starts_line_with_homebased_wait("cd x && homebased wait a"));

        assert!(redirects_stderr_first("make 2>&1 > out"));
        assert!(redirects_stderr_first("make 2>&1>/dev/null"));
        assert!(!redirects_stderr_first("make > out 2>&1"));
        assert!(!redirects_stderr_first("make 2>&1 >&2"));
        assert!(!redirects_stderr_first("make 2>&1 >"));
    }
}
