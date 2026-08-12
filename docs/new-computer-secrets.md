# `cmd` secrets setup on a new computer

The shell loads private values from `~/.secrets.zsh`. The dotfiles `zshrc` sources this file when a new Zsh shell starts. Do not use `~/.secrets.sh`, and do not commit the secrets file to this repository.

This is also the general shell secrets file. It currently contains API keys for CLIProxy, Cloudflare, DigitalOcean, Greptile, Groq, OpenAI, OpenAlex, and Semantic Scholar. Copy the complete file to preserve those services. If the file is not available, create replacement keys in each service dashboard.

`cmd` also uses login data from AWS, GitHub CLI, Codex, Google Cloud CLI, and 1Password CLI. These credentials are not all in `~/.secrets.zsh`.

## Preferred setup: copy from the current computer

Use an encrypted transfer method to copy these files from the old computer:

```text
~/.secrets.zsh
~/.aws/config
~/.local/bin/aws-secrets-credentials
```

Restore the file permissions:

```sh
chmod 600 ~/.secrets.zsh
chmod 600 ~/.aws/config
chmod 755 ~/.local/bin/aws-secrets-credentials
```

The current setup does not use `~/.aws/credentials`.

## Values in `~/.secrets.zsh` used by `cmd`

Keep these entries as Zsh exports:

```sh
export CLOUDFLARE_ACCOUNT_ID="..."
export CMD_CLOUDFLARE_BILLING_API_TOKEN="..."
export CMD_CLOUDFLARE_REDIRECT_API_TOKEN="..."
export DIGITALOCEAN_BILLING_TOKEN="..."
export AWS_LOCAL_AUTOMATION_ACCESS_KEY_ID="..."
export AWS_LOCAL_AUTOMATION_SECRET_ACCESS_KEY="..."
```

`cmd billing` uses the Cloudflare billing token, DigitalOcean token, and AWS credentials. `cmd cloudflare redirect` uses `CMD_CLOUDFLARE_REDIRECT_API_TOKEN`.

The AWS `default` and `local-automation` profiles use the two `AWS_LOCAL_AUTOMATION_*` values through `~/.local/bin/aws-secrets-credentials`. The `infraops-admin` profile uses an AWS CLI login session.

`GITHUB_TOKEN` and `GH_TOKEN` are optional. Most GitHub operations use `gh auth token` when these variables are not set. `cmd install` only reads `GITHUB_TOKEN`, but public GitHub releases do not normally need it.

Do not put tokens on the command line because shell history can retain them. Do not print the file when checking the setup.

## If the old computer is not available

- **Cloudflare:** In the Cloudflare dashboard, copy the account ID and create a new API token with **Account Billing Read** permission. Save them as `CLOUDFLARE_ACCOUNT_ID` and `CMD_CLOUDFLARE_BILLING_API_TOKEN`.
- **DigitalOcean:** In the DigitalOcean control panel, create a token with the `billing:read` scope. Save it as `DIGITALOCEAN_BILLING_TOKEN`. The command also accepts the older `DIGITALOCEAN_TOKEN`, but the dedicated billing token matches the current setup.
- **AWS local automation:** Create or recover the access key for the same IAM identity. Save the key ID and secret access key in the two `AWS_LOCAL_AUTOMATION_*` variables. Restore the helper and `~/.aws/config` from a trusted backup.
- **AWS `infraops-admin`:** Recreate the AWS CLI login session in `~/.aws/config`, then run `aws login --profile infraops-admin`. This profile does not use `~/.aws/credentials` in the current setup.
- **Cloudflare redirects:** Create a token with the access that the redirect commands need. Save it as `CMD_CLOUDFLARE_REDIRECT_API_TOKEN`.

Revoke old provider tokens if the old computer was lost or was not erased securely.

## GitHub CLI authentication

GitHub tokens are in the system keyring in the current setup. They are not in `~/.secrets.zsh` or `~/.config/gh/hosts.yml`.

On the new computer, run:

```sh
gh auth login
gh auth setup-git
gh auth status
```

This supplies credentials for `cmd pr-context`, `cmd better-context`, private Git repositories, and GitHub API metadata.

## Codex authentication

Codex access and refresh tokens are in these files:

```text
~/.codex/auth.json
~/.codex/profiles/*/auth.json
```

Do not commit these files. A new login is better than a manual copy. The current setup has the saved profile `a`:

```sh
cmd codex login a
cmd codex switch a
cmd codex list
```

Use `cmd codex login <name>` again for each extra profile that you want to restore.

## Google Cloud authentication

`cmd gcloud` reads project and cluster metadata from `cmd/secrets/ln.yaml` and `cmd/secrets/sq.yaml`. It uses the active Google Cloud CLI credentials in `~/.config/gcloud/` for access.

On the new computer, authenticate again instead of copying the credential database:

```sh
gcloud auth login
gcloud auth list
```

Then restore the ignored metadata files with the 1Password steps below.

## 1Password-backed `cmd` secrets

The following commands use the 1Password CLI and the `CLI` vault:

- `cmd secret get`, `save`, and `update`
- `cmd file encrypt`, `decrypt`, and `init`
- `cmd vault encrypt` and `decrypt`
- `cmd terraform init`, `encrypt`, `decrypt`, and `run`

Enable the 1Password desktop-app CLI integration or sign in with `op`. Check access without showing secrets:

```sh
op whoami
```

The `CLI` vault contains the `cmd_secrets` item. Its `ln.yaml` and `sq.yaml` fields restore the ignored local files:

```sh
mkdir -p cmd/secrets
cmd secret save
cmd release
```

`cmd` includes these YAML files at build time. Run `cmd release` after `cmd secret save` so the installed binary contains the restored metadata.

The same vault contains the age encryption keys that are named in the first line of each encrypted file. No local age identity file is used.

## Verify the setup

Start a new shell so `~/.secrets.zsh` is loaded:

```sh
exec zsh
```

Check that the required variable names are set without printing their values:

```sh
for name in \
  CLOUDFLARE_ACCOUNT_ID \
  CMD_CLOUDFLARE_BILLING_API_TOKEN \
  CMD_CLOUDFLARE_REDIRECT_API_TOKEN \
  DIGITALOCEAN_BILLING_TOKEN \
  AWS_LOCAL_AUTOMATION_ACCESS_KEY_ID \
  AWS_LOCAL_AUTOMATION_SECRET_ACCESS_KEY
do
  if [[ -n ${(P)name} ]]; then
    print "$name: set"
  else
    print "$name: missing"
  fi
done
```

Then run:

```sh
cmd billing
cmd billing --output json
gh auth status
cmd codex list
gcloud auth list
op whoami
```

The overview can show successful providers even when another provider is not configured. It returns a nonzero status if any provider fails.
