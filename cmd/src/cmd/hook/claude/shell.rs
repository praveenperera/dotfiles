//! A small shell tokenizer that finds the commands a Bash tool call runs
//!
//! It is not a shell parser. It splits on `;`, `&`, `|`, newlines, parentheses,
//! and backticks, keeps quoted text in one word, drops redirections, comments,
//! and heredoc bodies, and reads into `$(...)`, backticks, `eval`, `sh -c '...'`
//! scripts, and heredocs a shell reads its commands from, as in
//! `bash -s <<'EOF'`. Callers can also read the command a `homebased task` or
//! `ssh` call runs elsewhere. That is enough to tell a command that runs `gh run watch`
//! from one that only mentions it, such as `grep`, `echo`, or `git commit -m`

use std::borrow::Cow;
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
    /// The heredoc body each command reads on stdin, by index into `commands`
    stdin: Vec<Option<String>>,
    /// The source with heredoc bodies removed
    pub text: String,
}

impl Script {
    pub fn parse(source: &str) -> Self {
        parse_at(source, 0, None)
    }

    fn push(&mut self, words: Vec<String>, stdin: Option<&str>) {
        self.commands.push(words);
        self.stdin.push(stdin.map(str::to_owned));
    }

    fn append(&mut self, other: Script) {
        self.commands.extend(other.commands);
        self.stdin.extend(other.stdin);
    }

    /// Whether any command satisfies `matches`
    pub fn runs(&self, matches: impl Fn(&Command) -> bool) -> bool {
        self.iter().any(|command| matches(&command))
    }

    /// Every command the script runs
    pub fn iter(&self) -> impl Iterator<Item = Command<'_>> {
        self.commands.iter().enumerate().map(|(at, words)| {
            let stdin = self.stdin.get(at).and_then(Option::as_deref);
            Command(words, stdin)
        })
    }
}

/// One command's words and the heredoc body on its stdin, with helpers for
/// matching its program and arguments
#[derive(Debug, Clone, Copy)]
pub struct Command<'a>(&'a [String], Option<&'a str>);

impl Command<'_> {
    /// The program's file name, so `~/.local/bin/cmd` reads as `cmd`
    pub fn program(&self) -> &str {
        self.0.first().map_or("", |word| file_name(word))
    }

    pub fn args(&self) -> impl Iterator<Item = &str> {
        self.0.iter().skip(1).map(String::as_str)
    }

    /// The words after the first `--`, read as the command a program such as
    /// `homebased task run -- sh -c '...'` runs without a shell
    pub fn trailing_command(&self) -> Option<Script> {
        let at = self.0.iter().position(|word| word == "--")?;
        let quoted = self.0[at + 1..]
            .iter()
            .map(|word| format!("'{}'", word.replace('\'', r"'\''")))
            .collect::<Vec<_>>()
            .join(" ");

        Some(Script::parse(&quoted))
    }

    /// The script `ssh` runs on the remote host: the words after the
    /// destination, joined with spaces as ssh sends them to the remote shell,
    /// so `ssh code pkill -f x` and `ssh code 'pkill -f x'` read the same, with
    /// a heredoc on ssh's stdin passed on to the remote command, or run as the
    /// script when there is no command
    pub fn remote_command(&self) -> Option<Script> {
        if self.program() != "ssh" {
            return None;
        }

        // OpenSSH reads options again after the destination, as in
        // `ssh code -p 2222 uptime`, unless a `--` before it ended them
        let (rest, terminated) = skip_ssh_options(&self.0[1..]);
        let rest = rest.get(1..)?;
        let rest = match terminated {
            true => rest,
            false => skip_ssh_options(rest).0,
        };

        if rest.is_empty() {
            return self.1.map(Script::parse);
        }

        Some(parse_at(&rest.join(" "), 0, self.1))
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

/// The file name of a program word, so `/bin/kill` reads as `kill`
pub fn file_name(word: &str) -> &str {
    Path::new(word)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(word)
}

/// Parses `source`, whose commands read `inherited` on stdin unless a heredoc
/// of their own replaces it
fn parse_at(source: &str, depth: usize, inherited: Option<&str>) -> Script {
    let mut lexer = Lexer::new(source, depth);
    lexer.run();

    let mut script = Script::default();
    for (at, words) in lexer.segments.iter().enumerate() {
        let stdin = lexer.heredoc_of(at).or(inherited);
        resolve(words, stdin, depth, &mut script);
    }

    script.append(std::mem::take(&mut lexer.nested));
    script.text = lexer.text;
    script
}

/// Strips what comes before the real program and reads into the scripts of
/// `sh -c`, `eval`, and a shell reading its commands from a heredoc
fn resolve(words: &[String], stdin: Option<&str>, depth: usize, out: &mut Script) {
    let mut rest = words;
    loop {
        match rest.first().map(String::as_str) {
            None => return,
            Some(word) if KEYWORDS.contains(&word) || is_assignment(word) => rest = &rest[1..],
            // wrappers that run the rest of the line, by name or by path
            Some(word) => match file_name(word) {
                "timeout" => rest = skip_timeout(&rest[1..]),
                "env" => rest = skip_options(&rest[1..], ENV_VALUE_FLAGS),
                "sudo" => rest = skip_options(&rest[1..], SUDO_VALUE_FLAGS),
                "doas" => rest = skip_options(&rest[1..], &["-a", "-C", "-u"]),
                "nice" => rest = skip_options(&rest[1..], &["-n", "--adjustment"]),
                "ionice" => rest = skip_options(&rest[1..], IONICE_VALUE_FLAGS),
                "stdbuf" => rest = skip_options(&rest[1..], STDBUF_VALUE_FLAGS),
                "setsid" | "builtin" => rest = skip_options(&rest[1..], &[]),
                // `command -v x` only looks `x` up
                "command" if !matches!(rest.get(1).map(String::as_str), Some("-v" | "-V")) => {
                    rest = skip_options(&rest[1..], &[])
                }
                "nohup" | "exec" | "time" => rest = &rest[1..],
                _ => break,
            },
        }
    }

    if let Some((script, inherited)) = inner_script(rest, stdin) {
        if depth < MAX_DEPTH {
            out.append(parse_at(&script, depth + 1, inherited));
        }
    }

    out.push(rest.to_vec(), stdin);
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

const ENV_VALUE_FLAGS: &[&str] = &["-u", "--unset", "-C", "--chdir", "-S", "--split-string"];

const SUDO_VALUE_FLAGS: &[&str] = &[
    "-u",
    "--user",
    "-g",
    "--group",
    "-p",
    "--prompt",
    "-C",
    "--close-from",
    "-D",
    "--chdir",
    "-h",
    "--host",
    "-r",
    "--role",
    "-R",
    "--chroot",
    "-t",
    "--type",
    "-T",
    "--command-timeout",
    "-U",
    "--other-user",
];

const IONICE_VALUE_FLAGS: &[&str] = &[
    "-c",
    "--class",
    "-n",
    "--classdata",
    "-p",
    "--pid",
    "-P",
    "--pgid",
    "-u",
    "--uid",
];

const STDBUF_VALUE_FLAGS: &[&str] = &["-i", "--input", "-o", "--output", "-e", "--error"];

/// Skips a wrapper's options and `VAR=value` arguments up to the program it
/// runs, as in `env -u X GH_TOKEN=y gh ...` or `nice -n 19 make`, where
/// `value_flags` are the options that take the next word
fn skip_options<'a>(args: &'a [String], value_flags: &[&str]) -> &'a [String] {
    let mut rest = args;
    while let Some(arg) = rest.first() {
        match arg.as_str() {
            "--" => return &rest[1..],
            arg if value_flags.contains(&arg) => rest = rest.get(2..).unwrap_or_default(),
            arg if arg.starts_with('-') || is_assignment(arg) => rest = &rest[1..],
            _ => break,
        }
    }

    rest
}

/// The `ssh` flags that take a value, from its synopsis in `man ssh`
const SSH_VALUE_FLAGS: &str = "BbcDEeFIiJLlmOoPpQRSWw";

/// Skips `ssh`'s options, as in `ssh -p 2222 -o X=Y code`, and says whether a
/// `--` ended them
fn skip_ssh_options(args: &[String]) -> (&[String], bool) {
    let mut rest = args;
    while let Some(arg) = rest.first() {
        if arg == "--" {
            return (&rest[1..], true);
        }

        let Some(flags) = arg.strip_prefix('-').filter(|flags| !flags.is_empty()) else {
            break;
        };

        // a flag that takes a value ends its group, and the value is the rest of
        // the word, as in `-p2222`, or the next word, as in `-vp 2222`
        let value_at = flags.find(|flag| SSH_VALUE_FLAGS.contains(flag));
        rest = match value_at {
            Some(at) if at + 1 == flags.len() => rest.get(2..).unwrap_or_default(),
            _ => &rest[1..],
        };
    }

    (rest, false)
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

/// The script a command runs and the stdin that script reads: the argument of
/// `sh -c '...'`, `bash -lc "..."`, or `zsh -c ...`, the words of `eval`, or
/// the heredoc on `stdin` of a shell that reads its commands from there, as in
/// `bash <<'EOF'` or `bash -s arg <<'EOF'`
fn inner_script<'a>(
    words: &'a [String],
    stdin: Option<&'a str>,
) -> Option<(Cow<'a, str>, Option<&'a str>)> {
    let program = file_name(words.first()?);
    if program == "eval" {
        return Some((Cow::Owned(words[1..].join(" ")), stdin));
    }

    if !matches!(program, "sh" | "bash" | "zsh") {
        return None;
    }

    // a shell reading its script from stdin has no stdin left for its commands
    let from_stdin = || stdin.map(|body| (Cow::Borrowed(body), None));
    let mut has_c = false;
    let mut has_s = false;
    let mut args = words[1..].iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            // `-o pipefail` and `+o` take an option name
            "-o" | "+o" | "-O" | "+O" => {
                args.next();
            }
            arg if arg.starts_with("--") => {}
            arg if arg.starts_with('-') || arg.starts_with('+') => {
                has_c |= arg.contains('c');
                has_s |= arg.contains('s');
            }
            script if has_c => return Some((Cow::Borrowed(script), stdin)),
            // the first operand is a script file, unless `-s` keeps reading
            // commands from stdin and makes it a positional argument
            _ => return has_s.then(from_stdin).flatten(),
        }
    }

    match has_c {
        true => None,
        false => from_stdin(),
    }
}

/// A heredoc whose body starts after the current line
struct Heredoc {
    delimiter: String,
    /// `<<-`, which strips leading tabs from body lines and the terminator
    strip_tabs: bool,
    /// The index of the segment whose stdin it is, unless that segment had no
    /// words
    segment: Option<usize>,
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
    nested: Script,
    /// The next word is a redirection target, not an argument
    skip_word: bool,
    /// The next word is a heredoc delimiter
    heredoc_word: bool,
    heredocs: Vec<Heredoc>,
    heredoc_tabs: bool,
    /// Heredoc bodies by the index of the segment that reads them
    bodies: Vec<(usize, String)>,
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
            nested: Script::default(),
            skip_word: false,
            heredoc_word: false,
            heredocs: Vec::new(),
            heredoc_tabs: false,
            bodies: Vec::new(),
            text: String::with_capacity(source.len()),
        }
    }

    /// The heredoc body on a segment's stdin, the last one when it opened
    /// several, as the shell does
    fn heredoc_of(&self, segment: usize) -> Option<&str> {
        self.bodies
            .iter()
            .rev()
            .find(|(at, _)| *at == segment)
            .map(|(_, body)| body.as_str())
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
            self.heredocs.push(Heredoc {
                delimiter: word,
                strip_tabs: self.heredoc_tabs,
                segment: Some(self.segments.len()),
            });
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

        let at = self.segments.len();
        if !self.words.is_empty() {
            self.segments.push(std::mem::take(&mut self.words));
            return;
        }

        // a heredoc on a command with no words feeds nothing
        for heredoc in &mut self.heredocs {
            if heredoc.segment == Some(at) {
                heredoc.segment = None;
            }
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
                self.heredoc_tabs = self.peek(0) == Some('-');
                if self.heredoc_tabs {
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
        self.nested.append(parse_at(&inner, self.depth + 1, None));
    }

    /// Skips the bodies of heredocs opened on the line that just ended,
    /// keeping each one for the segment that reads it
    fn skip_heredoc_bodies(&mut self) {
        for heredoc in std::mem::take(&mut self.heredocs) {
            let mut body = String::new();
            while self.at < self.chars.len() {
                let line_end = self.chars[self.at..]
                    .iter()
                    .position(|&c| c == '\n')
                    .map_or(self.chars.len(), |offset| self.at + offset);

                let line = self.chars[self.at..line_end].iter().collect::<String>();
                self.at = (line_end + 1).min(self.chars.len());

                // as in the shell: the terminator is the whole line, after
                // leading tabs only for `<<-`
                let line = match heredoc.strip_tabs {
                    true => line.trim_start_matches('\t'),
                    false => line.as_str(),
                };
                if line == heredoc.delimiter {
                    break;
                }

                body.push_str(line);
                body.push('\n');
            }

            if let Some(segment) = heredoc.segment {
                self.bodies.push((segment, body));
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
            ("sudo -n -u root kill 1", &["kill 1"]),
            ("sudo -S -p '' FOO=1 kill 1", &["kill 1"]),
            ("nice -n 19 ionice -c 3 make", &["make"]),
            ("command -p make", &["make"]),
            ("command -v make", &["command -v make"]),
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
            ("eval 'a; b'", &["a", "b", "eval a; b"]),
            ("bash <<'EOF'\na; b\nEOF", &["a", "b", "bash"]),
            ("bash -s x <<EOF\na\nEOF\nc", &["a", "bash -s x", "c"]),
            ("bash x.sh <<EOF\na\nEOF", &["bash x.sh"]),
            ("cat <<EOF\na\nEOF", &["cat"]),
        ];

        for (source, want) in cases {
            assert_eq!(commands(source), *want, "{source:?}");
        }
    }

    #[test]
    fn reads_trailing_commands() {
        let trailing = |source: &str| {
            let script = Script::parse(source);
            let command = script.iter().next().unwrap();
            command.trailing_command().map(|script| {
                script
                    .commands
                    .iter()
                    .map(|words| words.join(" "))
                    .collect::<Vec<_>>()
            })
        };

        assert_eq!(
            trailing("homebased task run --name x -- pkill -f 'a b'"),
            Some(vec!["pkill -f a b".to_owned()])
        );
        assert_eq!(
            trailing("homebased task run -- echo \"it's\""),
            Some(vec!["echo it's".to_owned()])
        );
        assert_eq!(
            trailing("homebased task run -- sh -c 'echo a; kill 1'"),
            Some(vec![
                "echo a".to_owned(),
                "kill 1".to_owned(),
                "sh -c echo a; kill 1".to_owned(),
            ])
        );
        assert_eq!(trailing("homebased task list"), None);
    }

    #[test]
    fn reads_ssh_remote_commands() {
        let remote = |source: &str| {
            let script = Script::parse(source);
            let command = script.iter().next().unwrap();
            command.remote_command().map(|script| {
                script
                    .commands
                    .iter()
                    .map(|words| words.join(" "))
                    .collect::<Vec<_>>()
            })
        };

        let cases: &[(&str, Option<&[&str]>)] = &[
            ("ssh code pkill -f 'a b'", Some(&["pkill -f a b"])),
            ("ssh code 'a; b'", Some(&["a", "b"])),
            ("ssh -p 2222 -i key -o X=Y -J jump code a", Some(&["a"])),
            ("ssh -p2222 -vp 2222 -4 code a", Some(&["a"])),
            ("ssh code -l me -p 2222 a -p 1", Some(&["a -p 1"])),
            ("ssh -- code -p 2222", Some(&["-p 2222"])),
            ("ssh code \"sh -c 'a; b'\"", Some(&["a", "b", "sh -c a; b"])),
            ("ssh code", None),
            ("ssh -N -L 3000:localhost:3000 code", None),
            ("ssh-keygen -R code", None),
        ];

        for (source, want) in cases {
            let want = want.map(|want| want.iter().map(|&word| word.to_owned()).collect());
            assert_eq!(remote(source), want, "{source:?}");
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

        let script = Script::parse("cat <<EOF\n# note: x\nEOF\nls");
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
