# Storage on ai5090 and code

`ai5090` has three NVMe drives. Use all of them, and fill the Kingston last. The Kingston holds the OS and the `code` root, throttles first (its firmware slows it at about 70 C), and has a rising count of media errors, so new large or write-heavy data goes to the TEAM or the Lexar.

| Mount in `code` | Drive (Incus pool) | Holds | Lifecycle |
| --- | --- | --- | --- |
| `/build` | TEAM 4 TB (`build`, volume `build`) | Cargo `build-dir` (`/build/cargo/{workspace-path-hash}`), uv cache (`/build/cache/uv`), Bazel output root, Bazelisk cache, venvs, and other large scratch work | Regenerable; clean freely |
| `/shared` | Lexar 2 TB (`bulk`, volume `shared`) | Staging, preprocessing output, training runs in progress, the hercunet slab cache (`~/.vesuvius` links to `/shared/cache/vesuvius`), and finished data while the Lexar has room | Active; move finished trees to `/data` only when the Lexar needs room |
| `/var/lib/docker`, `/var/lib/containerd` | Lexar (`bulk`) | Nested Docker daemon data and image layers | Regenerable through Docker |
| `/data` | Kingston 2 TB (`agents`, volume `data`) | Released datasets, model blobs, finished checkpoints, evaluation results, backups | Keep; written once, then read |
| `/`, `~` | Kingston (`agents`) | OS, home directory, source checkouts, toolchains, agent state | Keep small |

The host's 32 GiB swap file is on the TEAM drive (`/srv/build/swapfile`). `/build` moves from the Lexar to the TEAM drive in November 2026; until `incus config device get code build pool` on the host prints `build`, it is still on the Lexar.

The host and Mac mini do not mount `/shared` or `/data`. All of these are Incus custom volumes, so they survive container recreation. `~/code/homelab/ai5090/README.md` (**Storage** and **Shared data**) is the source of truth for the host side.

## Where to write

- Write new datasets, preprocessing output, and training runs under `/shared`.
- Put large scratch work, extracted archives, and build output under `/build`. Do not set `CARGO_TARGET_DIR` to `/tmp`; `/tmp` is tmpfs.
- Keep large caches and downloads, such as model hubs or dataset mirrors, under `/shared` or `/build`, not in `~/.cache` or `~/code`, which are on the Kingston. Put a cache on the same mount as the trees that hardlink from it: uv's cache is under `/build` because most venvs are, so create new venvs under `/build` too. When a tool has no setting for its cache path, link its default directory to the new location, as `~/.vesuvius` is.
- Read finished data from `/data` or `/shared`. Older `/shared` paths for moved trees are symlinks into `/data`, so existing scripts keep working.
- Keep the Lexar below about 70 percent full, the TEAM below about 80 percent, and the Kingston as empty as practical. The Lexar and the TEAM are DRAM-less, and writes slow sharply once a drive's write cache fills.
- Check fill levels from the host with `df -h / /srv/bulk /srv/build`.

## Moving finished data to /data

Move a finished tree to `/data` only when the Lexar needs room, once nothing writes to it, to the same relative path under `/data`, and leave a symlink:

```bash
src=/shared/checkpoints/project/run-1; dst=/data/${src#/shared/}
sudo lsof +D "$src"                                  # must print nothing
mkdir -p "$(dirname "$dst")"                         # match owner and mode of the /shared parents
ionice -c3 rsync -aHAX --numeric-ids "$src/" "$dst/"
rsync -aHAXn --checksum --itemize-changes --numeric-ids "$src/" "$dst/"   # must print nothing
rm -rf -- "$src" && ln -s "$dst" "$src"
```

- Hard links cannot cross drives. Move hard-linked trees together, or the shared files become full copies. Check with `find TREE -type f -links +1`.
- A relative symlink inside a moved tree must resolve inside that tree.
- When syncing into a moved path later, pass `rsync --keep-dirlinks` (`-K`). Without it rsync replaces the symlink with a real directory on the Lexar.

## Copies and duplicates

- Reflinks work only within one drive. `cp -a --reflink=always` between `/shared`, `/data`, and `/build` fails by design; use it within one of them for large independent writable copies.
- `ai5090-dedupe.timer` runs `duperemove` daily over the Kingston pool, which covers `/data`. The Lexar and the TEAM are left out, so deduplicate `/shared` by hand after large copies: find duplicate groups with `fclones group /shared --min 1MiB -f fdupes`, keep one path per inode, and pass the list to `sudo duperemove --fdupes`. This merges extents in place and keeps inodes and hard links.
- `fclones dedupe` replaces each file with a new reflinked inode, which splits existing hard-link groups. Prefer `duperemove`.

## Cleanup

- `cln rust` removes Cargo build directories. Check `pgrep -af 'cargo|rustc'` first and skip build directories modified in the last few minutes; another agent may be building.
- Bazel output and the repository cache under `/build/bazel` are regenerable but cost a full re-download.
- Use `docker builder prune` and remove only superseded image tags. Never run `docker system prune --volumes`.
- Delete from `/shared`, `/data`, or `/build` only data that is backed up or reproducible. `/build` also holds venvs, downloads, and experiment runs that agents put there, not only build output. Aligner and preprocessing temp directories, such as `mfa-tmp`, are safe once their outputs and manifests exist.
