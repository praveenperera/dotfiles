//! A small shell tokenizer that finds the commands a Bash tool call runs
//!
//! It is not a shell parser. It splits on `;`, `&`, `|`, newlines, parentheses,
//! and backticks, keeps quoted text in one word, drops redirections, comments,
//! and heredoc bodies, and reads into `$(...)`, backticks, and `sh -c '...'`
//! scripts. That is enough to tell a command that runs `gh run watch` from one
//! that only mentions it, such as `grep`, `echo`, or `git commit -m`

use std::path::Path;

/// Nested `$(...)` and `sh -c` scripts deeper than this are not read, so a
/// pathological command cannot overflow the stack
const MAX_DEPTH: usize = 8;

/// Words that only introduce or group the next command, such as `then` in
/// `if x; then gh run watch; fi`
const KEYWORDS: &[&str] = &[
    "if", "then", "else", "elif", "fi", "do", "done", "while", "until", "!", "{", "}",
];

/// A shell command line, split into the commands it runs
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Script {
    /// Each command's words, after `VAR=value` prefixes, keywords, and wrappers
    /// such as `timeout 600` are stripped, including the commands inside
    /// `$(...)`, backticks, and `sh -c` scripts
    pub commands: Vec<Vec<String>>,
    /// The source with heredoc bodies removed
    pub text: String,
}

impl Script {
    pub fn parse(source: &str) -> Self {
        parse_at(source, 0)
    }

    /// Whether any command satisfies `matches`
    pub fn runs(&self, matches: impl Fn(&Command) -> bool) -> bool {
        self.commands.iter().any(|words| matches(&Command(words)))
    }
}

/// One command's words, with helpers for matching its program and arguments
#[derive(Debug, Clone, Copy)]
pub struct Command<'a>(&'a [String]);

impl Command<'_> {
    /// The program's file name, so `~/.local/bin/cmd` reads as `cmd`
    pub fn program(&self) -> &str {
        self.0.first().map_or("", |word| file_name(word))
    }

    pub fn args(&self) -> impl Iterator<Item = &str> {
        self.0.iter().skip(1).map(String::as_str)
    }

    /// Whether the arguments start with `prefix`
    pub fn starts_with(&self, prefix: &[&str]) -> bool {
        self.0.len() > prefix.len()
            && self.0[1..]
                .iter()
                .zip(prefix)
                .all(|(arg, want)| arg == want)
    }
}

fn file_name(word: &str) -> &str {
    Path::new(word)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(word)
}

fn parse_at(source: &str, depth: usize) -> Script {
    let mut lexer = Lexer::new(source, depth);
    lexer.run();

    let mut commands = Vec::new();
    for words in lexer.segments {
        resolve(&words, depth, &mut commands);
    }

    commands.extend(lexer.nested);
    Script {
        commands,
        text: lexer.text,
    }
}

/// Strips what comes before the real program and reads into `sh -c` scripts
fn resolve(words: &[String], depth: usize, out: &mut Vec<Vec<String>>) {
    let mut rest = words;
    loop {
        match rest.first().map(String::as_str) {
            None => return,
            Some(word) if KEYWORDS.contains(&word) || is_assignment(word) => rest = &rest[1..],
            Some("timeout") => rest = skip_timeout(&rest[1..]),
            Some(_) => break,
        }
    }

    if let Some(script) = shell_script(rest) {
        if depth < MAX_DEPTH {
            out.extend(parse_at(script, depth + 1).commands);
        }
    }

    out.push(rest.to_vec());
}

fn is_assignment(word: &str) -> bool {
    let Some((name, _)) = word.split_once('=') else {
        return false;
    };

    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Skips `timeout`'s options and duration, as in `timeout -s KILL 600 gh ...`
fn skip_timeout(args: &[String]) -> &[String] {
    let mut rest = args;
    while let Some(arg) = rest.first() {
        match arg.as_str() {
            "-s" | "-k" | "--signal" | "--kill-after" => rest = rest.get(2..).unwrap_or_default(),
            "--" => {
                rest = &rest[1..];
                break;
            }
            arg if arg.starts_with('-') && arg.len() > 1 => rest = &rest[1..],
            _ => break,
        }
    }

    // the duration
    rest.get(1..).unwrap_or_default()
}

/// The script of `sh -c '...'`, `bash -lc "..."`, or `zsh -c ...`
fn shell_script(words: &[String]) -> Option<&str> {
    let program = file_name(words.first()?);
    if !matches!(program, "sh" | "bash" | "zsh") {
        return None;
    }

    let mut has_c = false;
    let mut args = words[1..].iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            // `-o pipefail` and `+o` take an option name
            "-o" | "+o" | "-O" | "+O" => {
                args.next();
            }
            arg if arg.starts_with("--") => {}
            arg if arg.starts_with('-') || arg.starts_with('+') => has_c |= arg.contains('c'),
            script => return has_c.then_some(script),
        }
    }

    None
}

/// A heredoc whose body starts after the current line
struct Heredoc {
    delimiter: String,
}

struct Lexer {
    chars: Vec<char>,
    at: usize,
    depth: usize,
    word: String,
    in_word: bool,
    words: Vec<String>,
    segments: Vec<Vec<String>>,
    /// Commands found inside `$(...)` and backticks
    nested: Vec<Vec<String>>,
    /// The next word is a redirection target, not an argument
    skip_word: bool,
    /// The next word is a heredoc delimiter
    heredoc_word: bool,
    heredocs: Vec<Heredoc>,
    text: String,
}

impl Lexer {
    fn new(source: &str, depth: usize) -> Self {
        Self {
            chars: source.chars().collect(),
            at: 0,
            depth,
            word: String::new(),
            in_word: false,
            words: Vec::new(),
            segments: Vec::new(),
            nested: Vec::new(),
            skip_word: false,
            heredoc_word: false,
            heredocs: Vec::new(),
            text: String::with_capacity(source.len()),
        }
    }

    fn peek(&self, offset: usize) -> Option<char> {
        self.chars.get(self.at + offset).copied()
    }

    /// Consumes `count` characters into the output text
    fn take(&mut self, count: usize) {
        let end = (self.at + count).min(self.chars.len());
        self.text.extend(&self.chars[self.at..end]);
        self.at = end;
    }

    fn run(&mut self) {
        while let Some(c) = self.peek(0) {
            match c {
                '\\' => {
                    // a backslash-newline joins lines, any other escape is literal
                    if let Some(next) = self.peek(1).filter(|&next| next != '\n') {
                        self.word.push(next);
                        self.in_word = true;
                    }
                    self.take(2);
                }
                '\'' => {
                    self.take(1);
                    while let Some(c) = self.peek(0) {
                        self.take(1);
                        if c == '\'' {
                            break;
                        }
                        self.word.push(c);
                    }
                    self.in_word = true;
                }
                '"' => {
                    self.take(1);
                    self.double_quoted();
                    self.in_word = true;
                }
                '`' => {
                    self.backticks();
                    self.in_word = true;
                }
                '$' if self.peek(1) == Some('(') => {
                    self.substitution();
                    self.in_word = true;
                }
                '#' if !self.in_word => {
                    while self.peek(0).is_some_and(|c| c != '\n') {
                        self.take(1);
                    }
                }
                ' ' | '\t' | '\r' => {
                    self.end_word();
                    self.take(1);
                }
                '\n' => {
                    self.end_segment();
                    self.take(1);
                    self.skip_heredoc_bodies();
                }
                '&' if self.peek(1) == Some('>') => {
                    self.end_word();
                    self.take(2);
                    if self.peek(0) == Some('>') {
                        self.take(1);
                    }
                    self.skip_word = true;
                }
                ';' | '&' | '|' | '(' | ')' => {
                    self.end_segment();
                    self.take(1);
                }
                '<' | '>' => self.redirection(c),
                _ => {
                    self.word.push(c);
                    self.in_word = true;
                    self.take(1);
                }
            }
        }

        self.end_segment();
    }

    fn end_word(&mut self) {
        if !self.in_word {
            return;
        }

        let word = std::mem::take(&mut self.word);
        self.in_word = false;

        if self.heredoc_word {
            self.heredoc_word = false;
            self.heredocs.push(Heredoc { delimiter: word });
        } else if self.skip_word {
            self.skip_word = false;
        } else {
            self.words.push(word);
        }
    }

    fn end_segment(&mut self) {
        self.end_word();
        self.skip_word = false;
        self.heredoc_word = false;

        if !self.words.is_empty() {
            self.segments.push(std::mem::take(&mut self.words));
        }
    }

    fn redirection(&mut self, c: char) {
        // a word of digits right before the operator is a file descriptor, as in `2>`
        if self.in_word && self.word.chars().all(|c| c.is_ascii_digit()) {
            self.word.clear();
            self.in_word = false;
        }
        self.end_word();

        // `<(...)` and `>(...)` run a command
        if self.peek(1) == Some('(') {
            self.take(1);
            self.substitution_body();
            return;
        }

        if c == '<' && self.peek(1) == Some('<') {
            if self.peek(2) == Some('<') {
                self.take(3);
                self.skip_word = true;
            } else {
                self.take(2);
                if self.peek(0) == Some('-') {
                    self.take(1);
                }
                self.heredoc_word = true;
            }
            return;
        }

        self.take(1);
        if matches!(self.peek(0), Some('>' | '&' | '|')) || (c == '<' && self.peek(0) == Some('>'))
        {
            self.take(1);
        }
        self.skip_word = true;
    }

    fn double_quoted(&mut self) {
        while let Some(c) = self.peek(0) {
            match c {
                '"' => {
                    self.take(1);
                    return;
                }
                '\\' => {
                    if let Some(next) = self.peek(1) {
                        if !matches!(next, '"' | '\\' | '$' | '`' | '\n') {
                            self.word.push('\\');
                        }
                        if next != '\n' {
                            self.word.push(next);
                        }
                    }
                    self.take(2);
                }
                '$' if self.peek(1) == Some('(') => self.substitution(),
                '`' => self.backticks(),
                _ => {
                    self.word.push(c);
                    self.take(1);
                }
            }
        }
    }

    /// Reads `$(...)` at the cursor, or skips `$((...))` arithmetic, keeping its
    /// text in the current word
    fn substitution(&mut self) {
        let start = self.at;
        let arithmetic = self.peek(2) == Some('(');
        self.take(1);

        if arithmetic {
            self.closing_paren();
        } else {
            self.substitution_body();
        }

        self.word.extend(&self.chars[start..self.at]);
    }

    /// Parses the commands of a substitution whose `(` is at the cursor
    fn substitution_body(&mut self) {
        let start = self.at + 1;
        let closed = self.closing_paren();
        let end = if closed { self.at - 1 } else { self.at };
        self.nest(start, end);
    }

    /// Moves past the `)` that closes the `(` at the cursor, minding quotes, and
    /// returns whether there was one
    fn closing_paren(&mut self) -> bool {
        let mut depth = 0usize;
        while let Some(c) = self.peek(0) {
            self.take(1);
            match c {
                '(' => depth += 1,
                ')' => {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return true;
                    }
                }
                '\\' => self.take(1),
                '\'' => {
                    while self.peek(0).is_some_and(|c| c != '\'') {
                        self.take(1);
                    }
                    self.take(1);
                }
                '"' => {
                    while let Some(c) = self.peek(0) {
                        self.take(1);
                        match c {
                            '\\' => self.take(1),
                            '"' => break,
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }

        false
    }

    /// Reads a backtick substitution at the cursor, keeping its text in the
    /// current word
    fn backticks(&mut self) {
        let open = self.at;
        self.take(1);
        let start = self.at;
        while let Some(c) = self.peek(0) {
            if c == '`' {
                break;
            }
            self.take(if c == '\\' { 2 } else { 1 });
        }

        let end = self.at;
        self.take(1);
        self.nest(start, end);
        self.word.extend(&self.chars[open..self.at]);
    }

    fn nest(&mut self, start: usize, end: usize) {
        if self.depth >= MAX_DEPTH || start >= end {
            return;
        }

        let inner = self.chars[start..end].iter().collect::<String>();
        self.nested
            .extend(parse_at(&inner, self.depth + 1).commands);
    }

    /// Skips the bodies of heredocs opened on the line that just ended
    fn skip_heredoc_bodies(&mut self) {
        for heredoc in std::mem::take(&mut self.heredocs) {
            while self.at < self.chars.len() {
                let line_end = self.chars[self.at..]
                    .iter()
                    .position(|&c| c == '\n')
                    .map_or(self.chars.len(), |offset| self.at + offset);

                let line = self.chars[self.at..line_end].iter().collect::<String>();
                self.at = (line_end + 1).min(self.chars.len());

                // like the shell, but also tolerate trailing whitespace and any indent
                if line.trim() == heredoc.delimiter {
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Script;

    fn commands(source: &str) -> Vec<String> {
        Script::parse(source)
            .commands
            .iter()
            .map(|words| words.join(" "))
            .collect()
    }

    #[test]
    fn splits_commands() {
        let cases: &[(&str, &[&str])] = &[
            ("gh run watch 1", &["gh run watch 1"]),
            ("a; b && c || d | e & f", &["a", "b", "c", "d", "e", "f"]),
            ("a\nb", &["a", "b"]),
            ("(a) ; { b; }", &["a", "b"]),
            ("FOO=1 BAR=2 cargo test", &["cargo test"]),
            ("timeout 600 gh run watch 1", &["gh run watch 1"]),
            ("timeout -s KILL -k 5 600 gh run watch", &["gh run watch"]),
            ("timeout --foreground 5m make", &["make"]),
            ("if x; then y; else z; fi", &["x", "y", "z"]),
            (
                "while true; do gh pr checks; done",
                &["true", "gh pr checks"],
            ),
            ("! grep x f", &["grep x f"]),
            ("echo 'a; b' \"c && d\"", &["echo a; b c && d"]),
            ("echo a\\ b", &["echo a b"]),
            ("cmd ship \\\n  watch-pr 5", &["cmd ship watch-pr 5"]),
            ("echo ok # gh run watch", &["echo ok"]),
            ("echo a#b", &["echo a#b"]),
            ("make 2>&1 >out.log", &["make"]),
            ("make &> out.log", &["make"]),
            ("make > out 2>&1 | tail", &["make", "tail"]),
            ("cat < in > out", &["cat"]),
            ("cat <<<\"gh run watch\"", &["cat"]),
        ];

        for (source, want) in cases {
            assert_eq!(commands(source), *want, "{source:?}");
        }
    }

    #[test]
    fn reads_into_substitutions_and_shells() {
        let cases: &[(&str, &[&str])] = &[
            (
                "echo `gh run watch 1`",
                &["echo `gh run watch 1`", "gh run watch 1"],
            ),
            (
                "echo \"$(gh run watch 1)\"",
                &["echo $(gh run watch 1)", "gh run watch 1"],
            ),
            ("x=$(a | b) c", &["c", "a", "b"]),
            ("echo $((1 + 2))", &["echo $((1 + 2))"]),
            ("echo `a`.txt sleep", &["echo `a`.txt sleep", "a"]),
            ("diff <(gh run view 1) b", &["diff b", "gh run view 1"]),
            (
                "bash -c 'gh run watch 1'",
                &["gh run watch 1", "bash -c gh run watch 1"],
            ),
            (
                "sh -lc \"sleep 1; gh pr view\"",
                &["sleep 1", "gh pr view", "sh -lc sleep 1; gh pr view"],
            ),
            ("bash -ec 'x' arg0", &["x", "bash -ec x arg0"]),
            ("bash -o pipefail -c 'x'", &["x", "bash -o pipefail -c x"]),
            ("bash scripts/release.sh", &["bash scripts/release.sh"]),
            ("bash -l -c 'x'", &["x", "bash -l -c x"]),
        ];

        for (source, want) in cases {
            assert_eq!(commands(source), *want, "{source:?}");
        }
    }

    #[test]
    fn skips_heredoc_bodies() {
        let cases: &[(&str, &[&str])] = &[
            ("cat <<EOF\ngh run watch\nEOF\nls", &["cat", "ls"]),
            ("cat <<'EOF' > f\ngh run watch\nEOF", &["cat"]),
            ("cat <<-\"END\"\n\tgh run watch\n\tEND\nls", &["cat", "ls"]),
            ("cat << EOF | sh\nx\nEOF", &["cat", "sh"]),
            ("a <<A; b <<B\n1\nA\n2\nB\nc", &["a", "b", "c"]),
            // an unterminated body runs to the end
            ("cat <<EOF\ngh run watch", &["cat"]),
        ];

        for (source, want) in cases {
            assert_eq!(commands(source), *want, "{source:?}");
        }

        let script = Script::parse("cat <<EOF\n# foreground-wait: x\nEOF\nls");
        assert_eq!(script.text, "cat <<EOF\nls");
    }

    #[test]
    fn survives_unbalanced_and_deep_input() {
        for source in [
            "echo '", "echo \"", "echo $(", "echo `", "a <<", "a >", "))((", "\\",
        ] {
            let _ = Script::parse(source);
        }

        let deep = "$(".repeat(10_000);
        let _ = Script::parse(&deep);

        let shells = "bash -c '".repeat(200);
        let _ = Script::parse(&shells);
    }
}
