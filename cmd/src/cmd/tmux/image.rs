use super::managed::{
    list_paste_targets, resolve_route_for_tty, revalidate_paste_target, ResolvedPasteTarget,
};
use super::model::{MachineId, SshDestination};
use clap::Args;
use eyre::{eyre, Result, WrapErr};
use rand::RngExt;
use sha2::{Digest, Sha256};
use std::env;
use std::ffi::OsString;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const MAX_IMAGE_BYTES: u64 = 20 * 1024 * 1024;

#[derive(Debug, Clone, Args)]
/// Capture an image and paste its destination path into an exact tmux pane
pub struct PasteImageArgs {
    /// Ghostty terminal identity captured at invocation
    #[arg(long)]
    terminal_id: String,
    /// TTY of the focused Ghostty terminal at invocation
    #[arg(long)]
    tty: PathBuf,
    /// Read an image file instead of the clipboard
    #[arg(long)]
    source_file: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImageFormat {
    Png,
    Jpeg,
    Gif,
}

impl ImageFormat {
    fn extension(self) -> &'static str {
        match self {
            Self::Png => "png",
            Self::Jpeg => "jpg",
            Self::Gif => "gif",
        }
    }
}

#[derive(Debug)]
struct CapturedImage {
    file: tempfile::NamedTempFile,
    format: ImageFormat,
}

pub(super) fn paste_image(args: PasteImageArgs) -> Result<()> {
    validate_terminal_id(&args.terminal_id)?;
    validate_tty(&args.tty)?;
    let _lock = OperationLock::acquire(&args.tty)?;
    let image = capture_image(args.source_file.as_deref())?;
    let target = resolve_target(&args.tty)?;
    let destination = upload(&image, &target, &random_id())?;

    if let Err(error) = revalidate_ghostty_focus(&args.terminal_id, &args.tty) {
        return Err(error.wrap_err(format!(
            "image is uploaded at {} but terminal focus changed; nothing was inserted",
            destination.display()
        )));
    }
    if let Err(error) = revalidate_paste_target(&target, &target.source_tty) {
        return Err(error.wrap_err(format!(
            "image is uploaded at {} but focus or connection state changed; nothing was inserted",
            destination.display()
        )));
    }
    if let Err(failure) = insert_path(&target, &destination) {
        return match failure {
            InsertionFailure::NotInserted(error) => Err(error.wrap_err(format!(
                "image is uploaded at {} but nothing was inserted",
                destination.display()
            ))),
            InsertionFailure::Uncertain(error) => Err(error.wrap_err(format!(
                "image is uploaded at {} but insertion is uncertain; check the prompt before trying again",
                destination.display()
            ))),
        };
    }
    println!("Inserted {}", destination.display());
    Ok(())
}

fn revalidate_ghostty_focus(terminal_id: &str, tty: &Path) -> Result<()> {
    revalidate_ghostty_focus_with(command_path("CMD_OSASCRIPT", "osascript"), terminal_id, tty)
}

fn revalidate_ghostty_focus_with(program: OsString, terminal_id: &str, tty: &Path) -> Result<()> {
    let script = r#"tell application "Ghostty"
if not frontmost then error "Ghostty is not focused"
set activeTerminal to focused terminal of selected tab of front window
return (id of activeTerminal) & linefeed & (tty of activeTerminal)
end tell"#;
    let output = Command::new(program)
        .args(["-e", script])
        .output()
        .wrap_err("failed to revalidate the focused Ghostty terminal")?;
    if !output.status.success() {
        return Err(eyre!("focused Ghostty terminal is no longer available"));
    }
    let current = String::from_utf8(output.stdout).wrap_err("Ghostty returned invalid text")?;
    let expected = format!("{terminal_id}\n{}", tty.display());
    if current.trim_end() != expected {
        return Err(eyre!(
            "focused Ghostty terminal changed during image transfer"
        ));
    }
    Ok(())
}

fn validate_terminal_id(value: &str) -> Result<()> {
    if value.is_empty() || value.len() > 256 || value.contains(['\n', '\r', '\0']) {
        return Err(eyre!("invalid Ghostty terminal identity"));
    }
    Ok(())
}

fn validate_tty(tty: &Path) -> Result<()> {
    if !tty.is_absolute() {
        return Err(eyre!("focused terminal TTY must be an absolute path"));
    }
    Ok(())
}

fn resolve_target(tty: &Path) -> Result<ResolvedPasteTarget> {
    resolve_route_for_tty(tty).or_else(|automatic_error| {
        pick_target(tty).wrap_err_with(|| {
            format!("automatic image destination routing failed: {automatic_error:#}")
        })
    })
}

fn pick_target(tty: &Path) -> Result<ResolvedPasteTarget> {
    let targets = list_paste_targets(tty)?;
    if targets.is_empty() {
        return Err(eyre!("no live tmux image destinations are available"));
    }
    let labels = targets.iter().map(target_label).collect::<Vec<_>>();
    let mut command = Command::new(command_path("CMD_OSASCRIPT", "osascript"));
    command.arg("-e").arg(picker_script(labels.len()));
    command.args(labels);
    let output = command
        .output()
        .wrap_err("failed to open the image destination picker")?;
    if !output.status.success() {
        return Err(eyre!("image destination selection was cancelled"));
    }
    let index = String::from_utf8(output.stdout)
        .wrap_err("image destination picker returned invalid text")?
        .trim()
        .parse::<usize>()
        .wrap_err("image destination picker returned an invalid selection")?;
    targets
        .into_iter()
        .nth(index)
        .ok_or_else(|| eyre!("image destination picker returned an unknown selection"))
}

fn picker_script(count: usize) -> String {
    format!(
        r#"on run argv
set choices to items 1 thru {count} of argv
set picked to choose from list choices with title "Paste Image" with prompt "Choose the exact tmux pane" without multiple selections allowed
if picked is false then error number -128
set chosen to item 1 of picked
repeat with i from 1 to count choices
if item i of choices is chosen then return (i - 1) as text
end repeat
error number -128
end run"#
    )
}

fn target_label(target: &ResolvedPasteTarget) -> String {
    format!(
        "{}  {} / {}  {}  {}",
        machine_name(target.machine),
        target.display_session,
        target.display_window,
        target.pane,
        target.cwd.display()
    )
}

fn capture_image(source: Option<&Path>) -> Result<CapturedImage> {
    let mut snapshot = tempfile::Builder::new()
        .prefix("cmd-image-")
        .suffix(".snapshot")
        .tempfile()
        .wrap_err("failed to create private image snapshot")?;
    snapshot
        .as_file()
        .set_permissions(fs::Permissions::from_mode(0o600))?;
    if let Some(source) = source {
        snapshot_file(source, &mut snapshot)?;
    } else {
        capture_clipboard(snapshot.path())?;
    }
    let format = validate_image(snapshot.path())?;
    Ok(CapturedImage {
        file: snapshot,
        format,
    })
}

fn snapshot_file(source: &Path, destination: &mut tempfile::NamedTempFile) -> Result<()> {
    let metadata = fs::symlink_metadata(source).wrap_err("failed to inspect source image")?;
    if !metadata.file_type().is_file() {
        return Err(eyre!("source image must be a regular file"));
    }
    if metadata.len() > MAX_IMAGE_BYTES {
        return Err(eyre!("source image exceeds the 20 MiB size limit"));
    }
    let mut input = fs::File::open(source).wrap_err("source image is not readable")?;
    io::copy(&mut input, destination).wrap_err("failed to snapshot source image")?;
    Ok(())
}

fn capture_clipboard(destination: &Path) -> Result<()> {
    let status = Command::new(command_path("CMD_PNGPASTE", "pngpaste"))
        .arg(destination)
        .status()
        .wrap_err("pngpaste is required to read an image from the clipboard")?;
    if !status.success() {
        return Err(eyre!("clipboard does not contain one image"));
    }
    Ok(())
}

fn validate_image(path: &Path) -> Result<ImageFormat> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() || metadata.len() == 0 {
        return Err(eyre!("captured image is empty or not a regular file"));
    }
    if metadata.len() > MAX_IMAGE_BYTES {
        return Err(eyre!("captured image exceeds the 20 MiB size limit"));
    }
    let bytes = fs::read(path)?;
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Ok(ImageFormat::Png);
    }
    if bytes.starts_with(b"\xff\xd8\xff") && bytes.ends_with(b"\xff\xd9") {
        return Ok(ImageFormat::Jpeg);
    }
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return Ok(ImageFormat::Gif);
    }
    Err(eyre!(
        "clipboard data is not a supported PNG, JPEG, or GIF image"
    ))
}

fn upload(
    image: &CapturedImage,
    target: &ResolvedPasteTarget,
    transfer_id: &str,
) -> Result<PathBuf> {
    match &target.ssh {
        None => upload_local(image.file.path(), &target.cwd, image.format, transfer_id),
        Some(destination) => upload_remote(image, target, destination, transfer_id),
    }
}

fn upload_local(
    source: &Path,
    cwd: &Path,
    format: ImageFormat,
    transfer_id: &str,
) -> Result<PathBuf> {
    let directory = attachment_directory(cwd);
    fs::create_dir_all(&directory).wrap_err("failed to create attachment directory")?;
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700))?;
    let destination = directory.join(filename(transfer_id, format));
    publish_local(source, &destination)?;
    Ok(destination)
}

fn attachment_directory(cwd: &Path) -> PathBuf {
    let checkout = cwd
        .ancestors()
        .find(|ancestor| ancestor.join(".git").exists())
        .unwrap_or(cwd);
    checkout.join("_scratch/attachments")
}

fn publish_local(source: &Path, destination: &Path) -> Result<()> {
    let directory = destination
        .parent()
        .ok_or_else(|| eyre!("attachment destination has no parent"))?;
    let mut staging = tempfile::Builder::new()
        .prefix(".upload-")
        .tempfile_in(directory)
        .wrap_err("failed to create private attachment staging file")?;
    let mut input = fs::File::open(source)?;
    io::copy(&mut input, &mut staging)?;
    staging.as_file().sync_all()?;
    staging
        .as_file()
        .set_permissions(fs::Permissions::from_mode(0o600))?;
    staging
        .persist_noclobber(destination)
        .wrap_err("failed to publish attachment atomically")?;
    Ok(())
}

fn upload_remote(
    image: &CapturedImage,
    target: &ResolvedPasteTarget,
    destination: &SshDestination,
    transfer_id: &str,
) -> Result<PathBuf> {
    let mut child = Command::new(command_path("CMD_SSH", "ssh"))
        .arg(&destination.alias)
        .arg(remote_upload_command(target, transfer_id, image.format)?)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .wrap_err_with(|| format!("failed to start upload to {}", destination.alias))?;
    let mut input = fs::File::open(image.file.path())?;
    io::copy(
        &mut input,
        child
            .stdin
            .as_mut()
            .ok_or_else(|| eyre!("remote upload input is unavailable"))?,
    )?;
    drop(child.stdin.take());
    parse_remote_upload_output(child.wait_with_output()?, &destination.alias)
}

fn remote_upload_command(
    target: &ResolvedPasteTarget,
    transfer_id: &str,
    format: ImageFormat,
) -> Result<String> {
    validate_transfer_id(transfer_id)?;
    let socket = target
        .server
        .socket
        .to_str()
        .ok_or_else(|| eyre!("remote tmux socket is not UTF-8"))?;
    let arguments = [
        shell_quote(socket),
        shell_quote(&target.server.generation.to_string()),
        shell_quote(&target.pane.to_string()),
        shell_quote(transfer_id),
        shell_quote(format.extension()),
    ];
    Ok(format!(
        "sh -c {} sh {}",
        shell_quote(REMOTE_UPLOAD_SCRIPT),
        arguments.join(" ")
    ))
}

const REMOTE_UPLOAD_SCRIPT: &str = r#"set -eu
socket=$1
generation=$2
pane=$3
transfer=$4
extension=$5
record=$(tmux -S "$socket" display-message -p -t "$pane" '#{pid}\t#{pane_id}\t#{pane_current_path}') || exit 41
IFS='	' read -r live_generation live_pane cwd <<EOF
$record
EOF
[ "$live_generation" = "$generation" ] || exit 42
[ "$live_pane" = "$pane" ] || exit 43
checkout=$(git -C "$cwd" rev-parse --show-toplevel 2>/dev/null || true)
[ -n "$checkout" ] || checkout=$cwd
directory=$checkout/_scratch/attachments
(umask 077 && mkdir -p "$directory")
chmod 700 "$directory"
final=$directory/image-$transfer.$extension
[ ! -e "$final" ] || exit 44
staging=$(umask 077 && mktemp "$directory/.upload-XXXXXX")
trap 'rm -f "$staging"' EXIT HUP INT TERM
cat >"$staging"
chmod 600 "$staging"
mv "$staging" "$final"
trap - EXIT HUP INT TERM
printf '%s\n' "$final""#;

fn parse_remote_upload_output(output: Output, alias: &str) -> Result<PathBuf> {
    if !output.status.success() {
        return Err(eyre!(
            "image transfer to {alias} failed with {}; no path was inserted",
            output.status
        ));
    }
    let value =
        String::from_utf8(output.stdout).wrap_err("remote attachment path was not UTF-8")?;
    let path = PathBuf::from(value.trim_end());
    if !path.is_absolute() || value.contains('\0') || value.lines().count() != 1 {
        return Err(eyre!("remote host returned an invalid attachment path"));
    }
    Ok(path)
}

enum InsertionFailure {
    NotInserted(eyre::Report),
    Uncertain(eyre::Report),
}

fn insert_path(
    target: &ResolvedPasteTarget,
    destination: &Path,
) -> std::result::Result<(), InsertionFailure> {
    let path = destination
        .to_str()
        .ok_or_else(|| InsertionFailure::NotInserted(eyre!("attachment path is not UTF-8")))?;
    let buffer = format!("fleet-image-{}", random_id());
    run_tmux_with_input(
        target,
        &[
            "-S".into(),
            target.server.socket.to_string_lossy().into_owned(),
            "load-buffer".into(),
            "-b".into(),
            buffer.clone(),
            "-".into(),
        ],
        path.as_bytes(),
    )
    .map_err(InsertionFailure::NotInserted)?;
    let paste = [
        "-S".into(),
        target.server.socket.to_string_lossy().into_owned(),
        "paste-buffer".into(),
        "-d".into(),
        "-p".into(),
        "-b".into(),
        buffer.clone(),
        "-t".into(),
        target.pane.to_string(),
    ];
    if let Err(error) = run_tmux_with_input(target, &paste, &[]) {
        let _ = run_tmux_with_input(
            target,
            &[
                "-S".into(),
                target.server.socket.to_string_lossy().into_owned(),
                "delete-buffer".into(),
                "-b".into(),
                buffer,
            ],
            &[],
        );
        return Err(InsertionFailure::Uncertain(
            error.wrap_err("tmux did not confirm exact-pane bracketed paste"),
        ));
    }
    Ok(())
}

fn run_tmux_with_input(
    target: &ResolvedPasteTarget,
    arguments: &[String],
    input: &[u8],
) -> Result<()> {
    let output = match &target.ssh {
        None => command_with_input(command_path("CMD_TMUX", "tmux"), arguments, input)?,
        Some(destination) => {
            let command = format!(
                "tmux {}",
                arguments
                    .iter()
                    .map(|argument| shell_quote(argument))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
            command_with_input(
                command_path("CMD_SSH", "ssh"),
                &[destination.alias.clone(), command],
                input,
            )?
        }
    };
    if output.status.success() {
        Ok(())
    } else {
        Err(eyre!("tmux command failed with {}", output.status))
    }
}

fn command_with_input(program: OsString, arguments: &[String], input: &[u8]) -> Result<Output> {
    let mut child = Command::new(program)
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .as_mut()
        .ok_or_else(|| eyre!("command input is unavailable"))?
        .write_all(input)?;
    drop(child.stdin.take());
    Ok(child.wait_with_output()?)
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn filename(transfer_id: &str, format: ImageFormat) -> String {
    format!("image-{transfer_id}.{}", format.extension())
}

fn validate_transfer_id(value: &str) -> Result<()> {
    if value.len() != 24 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(eyre!("invalid image transfer identity"));
    }
    Ok(())
}

fn random_id() -> String {
    let random: [u8; 12] = rand::rng().random();
    hex::encode(random)
}

fn command_path(variable: &str, default: &str) -> OsString {
    env::var_os(variable).unwrap_or_else(|| OsString::from(default))
}

fn machine_name(machine: MachineId) -> &'static str {
    match machine {
        MachineId::Mini => "mini",
        MachineId::Code => "code",
        MachineId::Training => "training",
    }
}

struct OperationLock(PathBuf);

impl OperationLock {
    fn acquire(tty: &Path) -> Result<Self> {
        let base = env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(env::temp_dir)
            .join("cmd-image-paste");
        Self::acquire_in(&base, tty)
    }

    fn acquire_in(base: &Path, tty: &Path) -> Result<Self> {
        fs::create_dir_all(base)?;
        fs::set_permissions(base, fs::Permissions::from_mode(0o700))?;
        let digest = Sha256::digest(tty.as_os_str().as_encoded_bytes());
        let path = base.join(format!("{}.lock", hex::encode(digest)));
        Self::create(path, true)
    }

    fn create(path: PathBuf, remove_stale: bool) -> Result<Self> {
        let opened = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path);
        match opened {
            Ok(mut file) => {
                writeln!(file, "{}", std::process::id())?;
                file.sync_all()?;
                Ok(Self(path))
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists && remove_stale => {
                let owner = fs::read_to_string(&path)
                    .ok()
                    .and_then(|value| value.trim().parse::<u32>().ok());
                if owner.is_some_and(process_is_live) {
                    return Err(eyre!("an image paste is already running for this terminal"));
                }
                let stale = path.with_extension(format!("stale-{}", random_id()));
                fs::rename(&path, &stale).wrap_err("failed to claim stale image paste lock")?;
                let result = Self::create(path, false);
                let _ = fs::remove_file(stale);
                result
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                Err(eyre!("an image paste is already running for this terminal"))
            }
            Err(error) => Err(error.into()),
        }
    }
}

fn process_is_live(pid: u32) -> bool {
    Command::new("/bin/kill")
        .args(["-0", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

impl Drop for OperationLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::process::ExitStatusExt;

    fn executable(contents: &str) -> (tempfile::TempDir, PathBuf) {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("fake-command");
        fs::write(&path, contents).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        (directory, path)
    }

    fn write_image(size: u64) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"\x89PNG\r\n\x1a\n").unwrap();
        file.as_file().set_len(size).unwrap();
        file
    }

    #[test]
    fn validates_supported_image_signatures() {
        let mut png = tempfile::NamedTempFile::new().unwrap();
        png.write_all(b"\x89PNG\r\n\x1a\ncontent").unwrap();
        assert_eq!(validate_image(png.path()).unwrap(), ImageFormat::Png);
        let mut jpeg = tempfile::NamedTempFile::new().unwrap();
        jpeg.write_all(b"\xff\xd8\xffcontent\xff\xd9").unwrap();
        assert_eq!(validate_image(jpeg.path()).unwrap(), ImageFormat::Jpeg);
        let mut gif = tempfile::NamedTempFile::new().unwrap();
        gif.write_all(b"GIF89acontent").unwrap();
        assert_eq!(validate_image(gif.path()).unwrap(), ImageFormat::Gif);
    }

    #[test]
    fn enforces_image_size_at_exact_boundary() {
        assert_eq!(
            validate_image(write_image(MAX_IMAGE_BYTES).path()).unwrap(),
            ImageFormat::Png
        );
        assert!(validate_image(write_image(MAX_IMAGE_BYTES + 1).path()).is_err());
    }

    #[test]
    fn rejects_non_images_and_empty_images() {
        let mut text = tempfile::NamedTempFile::new().unwrap();
        text.write_all(b"not an image").unwrap();
        assert!(validate_image(text.path()).is_err());
        assert!(validate_image(tempfile::NamedTempFile::new().unwrap().path()).is_err());
    }

    #[test]
    fn rejects_invalid_image_before_resolving_route() {
        let mut text = tempfile::NamedTempFile::new().unwrap();
        text.write_all(b"not an image").unwrap();
        let error = paste_image(PasteImageArgs {
            terminal_id: "terminal-7".to_owned(),
            tty: PathBuf::from(format!("/dev/cmd-image-{}", random_id())),
            source_file: Some(text.path().to_path_buf()),
        })
        .unwrap_err();

        assert!(format!("{error:#}").contains("not a supported PNG, JPEG, or GIF image"));
    }

    #[test]
    fn derives_destination_from_owning_checkout() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join(".git")).unwrap();
        let nested = root.path().join("src/deep");
        fs::create_dir_all(&nested).unwrap();
        assert_eq!(
            attachment_directory(&nested),
            root.path().join("_scratch/attachments")
        );
    }

    #[test]
    fn publishes_private_file_without_overwriting() {
        let directory = tempfile::tempdir().unwrap();
        let mut source = tempfile::NamedTempFile::new().unwrap();
        source.write_all(b"image bytes").unwrap();
        let destination = directory.path().join("image.png");
        publish_local(source.path(), &destination).unwrap();
        assert_eq!(fs::read(&destination).unwrap(), b"image bytes");
        assert_eq!(
            fs::metadata(&destination).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert!(publish_local(source.path(), &destination).is_err());
    }

    #[test]
    fn shell_quotes_remote_values_without_interpolation() {
        assert_eq!(shell_quote("a b'c;$HOME"), "'a b'\\''c;$HOME'");
    }

    #[test]
    fn rejects_failed_remote_upload() {
        let failed = Output {
            status: std::process::ExitStatus::from_raw(256),
            stdout: Vec::new(),
            stderr: b"private error".to_vec(),
        };
        assert!(parse_remote_upload_output(failed, "code").is_err());
    }

    #[test]
    fn serializes_operations_per_terminal() {
        let runtime = tempfile::tempdir().unwrap();
        let first = OperationLock::acquire_in(runtime.path(), Path::new("/dev/ttys001")).unwrap();
        assert!(OperationLock::acquire_in(runtime.path(), Path::new("/dev/ttys001")).is_err());
        assert!(OperationLock::acquire_in(runtime.path(), Path::new("/dev/ttys002")).is_ok());
        drop(first);
        assert!(OperationLock::acquire_in(runtime.path(), Path::new("/dev/ttys001")).is_ok());
    }

    #[test]
    fn replaces_a_stale_terminal_lock() {
        let runtime = tempfile::tempdir().unwrap();
        let tty = Path::new("/dev/ttys001");
        let digest = Sha256::digest(tty.as_os_str().as_encoded_bytes());
        let path = runtime.path().join(format!("{}.lock", hex::encode(digest)));
        fs::write(&path, "4294967295\n").unwrap();
        assert!(OperationLock::acquire_in(runtime.path(), tty).is_ok());
    }

    #[test]
    fn picker_runs_outside_terminal_input() {
        assert!(picker_script(3).contains("choose from list"));
        assert!(!picker_script(3).contains("System Events"));
    }

    #[test]
    fn revalidates_terminal_identity_and_tty_with_fake_ghostty() {
        let (_directory, program) =
            executable("#!/bin/sh\nprintf 'terminal-7\\n/dev/ttys007\\n'\n");
        revalidate_ghostty_focus_with(
            program.clone().into_os_string(),
            "terminal-7",
            Path::new("/dev/ttys007"),
        )
        .unwrap();
        assert!(revalidate_ghostty_focus_with(
            program.into_os_string(),
            "terminal-8",
            Path::new("/dev/ttys007"),
        )
        .is_err());
    }

    #[test]
    fn command_input_is_not_shell_interpreted() {
        let (_directory, program) = executable("#!/bin/sh\ncat\n");
        let bytes = b"path with spaces; $(touch nope) ' quote";
        let output = command_with_input(program.into_os_string(), &[], bytes).unwrap();
        assert!(output.status.success());
        assert_eq!(output.stdout, bytes);
    }
}
