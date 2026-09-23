# Fleet tmux workspace

The Mini owns one shared tmux workspace. The `code` and `training` machines
keep their normal tmux servers and sessions. A remote workspace tab is a view
of an existing remote window. It does not start a second agent.

## Remote sessions

Use the existing commands from any normal shell:

```sh
ttc                    # list sessions on code
ttt                    # list sessions on training
ttc my-task            # create or attach on code
ttt my-task            # create or attach on training
ttc -a my-task         # attach only, with an exact session name
ttt --attach my-task
```

Use `--mode normal` for a normal remote tmux client. Use `--mode minimal` to
request a managed one-window view. The default `auto` mode imports all windows
when the command runs in the managed workspace. It uses the normal view in
other shells.

Managed service and view sessions have reserved `__fleet_` names. The `ttc`
and `ttt` lists omit them. A raw `tmux list-sessions` command can show them.

## Shared workspace

Create or attach a display on the Mini:

```sh
cmd tmux workspace attach --display main
cmd tmux workspace attach --display side
cmd tmux workspace attach --display phone
```

To keep an existing Mini session and its local windows, select it explicitly:

```sh
cmd tmux workspace attach --display main --adopt SESSION
```

The display sessions share the same window list. Each client can select a
different window. Panes and layouts in one shared window are still shared.

The Air must use the managed launcher so image routing can identify its local
terminal and Mini client:

```sh
cmd tmux workspace connect --display air
```

Normal SSH and normal tmux attachment remain available. If a connection has no
managed route, Paste Image asks for an exact machine, session, window, and
pane.

Use these recovery commands on the Mini:

```sh
cmd tmux workspace reconnect
cmd tmux workspace restore
cmd tmux unsubscribe code SESSION_ID
cmd tmux unsubscribe training SESSION_ID
cmd tmux workspace shutdown
```

`reconnect` uses saved identities and does not create replacement tasks.
`unsubscribe` removes only this workspace's views. `shutdown` stops the sync
worker and removes only managed workspace state and views.

Closing a mirrored tab with Cmd+W or prefix+x hides that tab and keeps the
remote task alive. Run `ttc SESSION` or `ttt SESSION` again to restore its
hidden windows. A direct `kill-window` is an intentional stop and ends that
remote window everywhere.

## Remote panes and keys

Managed remote views have no inner prefix, status line, or mouse handling.
The outer tmux keeps its normal keys. Use the **Remote Pane** action or
`cmd tmux picker remote-pane` to select a pane in a multi-pane remote window.

The main, side, Air, and active phone clients are visible clients. Observer
control connections use `ignore-size`; they do not change a task's terminal
size. An idle phone does not control the desktop size.

## Paste Image on macOS

Paste Image supports one clipboard image or one copied image file. It saves the
image first, then inserts the confirmed destination path into the exact tmux
pane. It does not send Return. Normal Cmd+V and phone uploads do not change.

Install it on each Mac:

1. Build and install `cmd` at `/Users/praveen/.local/bin/cmd`.
2. Install `pngpaste`. The full dotfiles bootstrap installs it with Homebrew.
3. Keep this repository at `/Users/praveen/code/dotfiles`.
4. Open [Paste Image.shortcut](../macos/Paste%20Image.shortcut) and add it to
   Shortcuts.
5. If Shortcuts uses Cmd+Shift+V for **Edit > Set Variable**, change that menu
   key first. In **System Settings > Keyboard > Keyboard Shortcuts > App
   Shortcuts**, select Shortcuts, enter `Set Variable` as the menu title, and
   assign Cmd+Option+Shift+V. Restart Shortcuts.
6. Open the shortcut's **Details** panel and select **Add Keyboard Shortcut**.
   Press Cmd+Shift+V in the key recorder, not in the action editor.
7. In Shortcuts settings, allow scripts to run when macOS asks.
8. Reload Ghostty configuration. Its default Cmd+Shift+V binding must be
   unbound so the shortcut receives the key. Keep `macos-shortcuts = ask`, and
   approve the Ghostty automation request when it appears.
9. Allow Shortcuts to automate Ghostty in **System Settings > Privacy &
   Security > Automation** if macOS asks.

The unsigned source is
[Paste Image.shortcut.xml](../macos/Paste%20Image.shortcut.xml). The shortcut
runs [fleet-paste-image](../macos/fleet-paste-image), which uses explicit paths
and does not depend on an interactive shell setup.

To use it:

1. Copy one image or screenshot.
2. Focus its agent pane in Ghostty.
3. Press Cmd+Shift+V.
4. Wait for the destination path to appear.
5. Add instructions and submit the prompt yourself.

The destination is `_scratch/attachments/` in the owning checkout, or below
the pane working directory when no checkout is found. Successful files remain
there so the agent can read them. The maximum image size is 20 MiB. A missing
image, failed transfer, changed focus, stale route, or cancelled picker inserts
nothing.

Apply the same setup steps on the Mini and Air. Permissions are local to
each Mac, so approval on one Mac does not approve the other.
