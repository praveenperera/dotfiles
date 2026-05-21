import * as cp from "child_process";
import * as path from "path";
import * as vscode from "vscode";

const compareScheme = "dotfiles-git-compare";

type GitStatusKind = "A" | "D" | "R" | "C" | string;

interface GitStatusEntry {
  kind: GitStatusKind;
  oldPath: string;
  newPath: string;
}

interface GitGraphFileRequest {
  repo?: string;
  filePath?: string;
}

export function activate(context: vscode.ExtensionContext) {
  context.subscriptions.push(
    vscode.commands.registerCommand("dotfilesGit.compareWithMaster", compareWithMaster),
    vscode.commands.registerCommand(
      "dotfilesGit.openGitGraphWorkspaceFile",
      openGitGraphWorkspaceFile
    )
  );
}

async function openGitGraphWorkspaceFile() {
  const editor = vscode.window.activeTextEditor;
  const uri = editor?.document.uri;

  if (!uri || uri.scheme !== "git-graph") {
    vscode.window.showErrorMessage("Open a Git Graph diff file first");
    return;
  }

  let request: GitGraphFileRequest;
  try {
    request = JSON.parse(Buffer.from(uri.query, "base64").toString("utf8"));
  } catch (error) {
    vscode.window.showErrorMessage(`Unable to read Git Graph file path: ${errorMessage(error)}`);
    return;
  }

  if (!request.repo || !request.filePath) {
    vscode.window.showErrorMessage("Git Graph did not provide a workspace file path");
    return;
  }

  const workspaceUri = vscode.Uri.file(path.join(request.repo, request.filePath));
  try {
    await vscode.workspace.fs.stat(workspaceUri);
  } catch (_) {
    vscode.window.showErrorMessage(`Workspace file does not exist: ${workspaceUri.fsPath}`);
    return;
  }

  await vscode.window.showTextDocument(workspaceUri, {
    preview: false,
    viewColumn: vscode.ViewColumn.Active,
  });
}

async function compareWithMaster() {
  const cwd = getStartingDirectory();
  if (!cwd) {
    vscode.window.showErrorMessage("Open a file or workspace folder inside a Git repository first");
    return;
  }

  let root: string;
  try {
    root = (await git(["rev-parse", "--show-toplevel"], cwd)).trim();
  } catch (error) {
    vscode.window.showErrorMessage(`Unable to find Git repository: ${errorMessage(error)}`);
    return;
  }

  const baseRef = vscode.workspace
    .getConfiguration("dotfilesGit")
    .get("compareBaseRef", "master");

  try {
    await git(["rev-parse", "--verify", `${baseRef}^{commit}`], root);
  } catch (_) {
    vscode.window.showErrorMessage(`Unable to compare: Git ref "${baseRef}" was not found`);
    return;
  }

  let entries: GitStatusEntry[];
  try {
    entries = parseNameStatus(
      await gitBuffer(["diff", "--name-status", "--find-renames", "-z", baseRef], root)
    );
    entries.push(
      ...parseUntracked(await gitBuffer(["ls-files", "--others", "--exclude-standard", "-z"], root))
    );
  } catch (error) {
    vscode.window.showErrorMessage(`Unable to compare with ${baseRef}: ${errorMessage(error)}`);
    return;
  }

  if (entries.length === 0) {
    vscode.window.showInformationMessage(`There are no workspace changes compared with "${baseRef}"`);
    return;
  }

  const resources = entries.map((entry) => toDiffResource(root, baseRef, entry));
  const sourceUri = vscode.Uri.from({
    scheme: compareScheme,
    path: `${root}/${baseRef}..workspace`,
  });

  await vscode.commands.executeCommand("_workbench.openMultiDiffEditor", {
    multiDiffSourceUri: sourceUri,
    title: `${baseRef} <-> workspace`,
    resources,
  });
}

function getStartingDirectory() {
  const editor = vscode.window.activeTextEditor;
  if (editor && editor.document.uri.scheme === "file") {
    return path.dirname(editor.document.uri.fsPath);
  }

  const folder = vscode.workspace.workspaceFolders?.[0];
  return folder?.uri.scheme === "file" ? folder.uri.fsPath : null;
}

function git(args: string[], cwd: string) {
  return gitBuffer(args, cwd).then((buffer) => buffer.toString("utf8"));
}

function gitBuffer(args: string[], cwd: string): Promise<Buffer> {
  return new Promise((resolve, reject) => {
    cp.execFile(
      "git",
      args,
      { cwd, maxBuffer: 50 * 1024 * 1024, encoding: "buffer" },
      (error, stdout, stderr) => {
        if (error) {
          const message = stderr.toString("utf8").trim() || error.message;
          reject(new Error(message));
          return;
        }

        resolve(stdout);
      }
    );
  });
}

function parseNameStatus(buffer: Buffer) {
  const fields = splitNul(buffer);
  const entries: GitStatusEntry[] = [];

  for (let index = 0; index < fields.length; ) {
    const status = fields[index++];
    if (!status) {
      continue;
    }

    const kind = status[0];
    if (kind === "R" || kind === "C") {
      entries.push({ kind, oldPath: fields[index++], newPath: fields[index++] });
    } else {
      entries.push({ kind, oldPath: fields[index], newPath: fields[index] });
      index += 1;
    }
  }

  return entries;
}

function parseUntracked(buffer: Buffer) {
  return splitNul(buffer)
    .filter(Boolean)
    .map((filePath) => ({ kind: "A", oldPath: filePath, newPath: filePath }));
}

function splitNul(buffer: Buffer) {
  return buffer.toString("utf8").split("\0").filter(Boolean);
}

function toDiffResource(root: string, baseRef: string, entry: GitStatusEntry) {
  if (entry.kind === "A") {
    return { originalUri: undefined, modifiedUri: fileUri(root, entry.newPath) };
  }

  if (entry.kind === "D") {
    return { originalUri: gitUri(root, entry.oldPath, baseRef), modifiedUri: undefined };
  }

  if (entry.kind === "R" || entry.kind === "C") {
    return {
      originalUri: gitUri(root, entry.oldPath, baseRef),
      modifiedUri: fileUri(root, entry.newPath),
    };
  }

  return {
    originalUri: gitUri(root, entry.oldPath, baseRef),
    modifiedUri: fileUri(root, entry.newPath),
  };
}

function fileUri(root: string, filePath: string) {
  return vscode.Uri.file(path.join(root, filePath));
}

function gitUri(root: string, filePath: string, ref: string) {
  const absolutePath = path.join(root, filePath);
  const uri = vscode.Uri.file(absolutePath);

  return uri.with({
    scheme: "git",
    path: uri.path,
    query: JSON.stringify({ path: absolutePath, ref }),
  });
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}
