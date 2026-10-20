# Storage on ai5090 and code

`ai5090` has two 2 TB NVMe drives. Split data by write load, not by size: heavy writes go to the Lexar, so the Kingston that holds the OS and the `code` root stays responsive, and read-mostly data fills the Kingston so both drives are used.

| Mount in `code` | Drive (Incus pool) | Holds | Lifecycle |
| --- | --- | --- | --- |
| `/`, `~` | Kingston (`agents`) | OS, home directory, source checkouts, toolchains | Keep |
| `/data` | Kingston (`agents`, volume `data`) | Released datasets, model blobs, finished checkpoints, evaluation results, backups | Keep; written once, then read |
| `/shared` | Lexar (`bulk`, volume `shared`) | Staging, preprocessing output, training runs in progress | Active; move to `/data` when final |
| `/build` | Lexar (`bulk`, volume `build`) | Cargo `build-dir` (`/build/cargo/{workspace-path-hash}`), Bazel output root, Bazelisk cache | Regenerable; clean freely |
| `/var/lib/docker`, `/var/lib/containerd` | Lexar (`bulk`) | Nested Docker daemon data and image layers | Regenerable through Docker |

The host and Mac mini do not mount `/shared` or `/data`. All of these are Incus custom volumes, so they survive container recreation. `~/code/homelab/ai5090/README.md` (**Storage** and **Shared data**) is the source of truth for the host side.

## Where to write

- Write new datasets, preprocessing output, and training runs under `/shared`.
- Read finished data from `/data`. Older `/shared` paths for moved trees are symlinks into `/data`, so existing scripts keep working.
- Keep Cargo and Bazel output under `/build`. Do not set `CARGO_TARGET_DIR` to `/tmp`; `/tmp` is tmpfs.
- Keep the Lexar below about 70 percent full. It is a DRAM-less drive, and writes slow sharply once its write cache fills.

## Moving finished data to /data

Move a tree once nothing writes to it, to the same relative path under `/data`, and leave a symlink:

```bash
src=/shared/checkpoints/project/run-1; dst=/data/${src#/shared/}
sudo lsof +D "$src"                                  # must print nothing
mkdir -p "$(dirname "$dst")"                         # match owner and mode of the /shared parents
ionice -c3 rsync -aHAX --numeric-ids "$src/" "$dst/"
rsync -aHAXn --checksum --itemize-changes --numeric-ids "$src/" "$dst/"   # must print nothing
rm -rf -- "$src" && ln -s "$dst" "$src"
```

- Hard links cannot cross the two drives. Move hard-linked trees together, or the shared files become full copies. Check with `find TREE -type f -links +1`.
- A relative symlink inside a moved tree must resolve inside that tree.
- When syncing into a moved path later, pass `rsync --keep-dirlinks` (`-K`). Without it rsync replaces the symlink with a real directory on the Lexar.

## Copies and duplicates

- Reflinks work only within one drive. `cp -a --reflink=always` between `/shared` and `/data` fails by design; use it within one of them for large independent writable copies.
- `ai5090-dedupe.timer` runs `duperemove` daily over the Kingston pool, which covers `/data`. The Lexar is left out, so deduplicate `/shared` by hand after large copies: find duplicate groups with `fclones group /shared --min 1MiB -f fdupes`, keep one path per inode, and pass the list to `sudo duperemove --fdupes`. This merges extents in place and keeps inodes and hard links.
- `fclones dedupe` replaces each file with a new reflinked inode, which splits existing hard-link groups. Prefer `duperemove`.

## Cleanup

- `cln rust` removes Cargo build directories. Check `pgrep -af 'cargo|rustc'` first and skip build directories modified in the last few minutes; another agent may be building.
- Bazel output and the repository cache under `/build/bazel` are regenerable but cost a full re-download.
- Use `docker builder prune` and remove only superseded image tags. Never run `docker system prune --volumes`.
- Delete from `/shared` or `/data` only data that is backed up or reproducible. Aligner and preprocessing temp directories, such as `mfa-tmp`, are safe once their outputs and manifests exist.
