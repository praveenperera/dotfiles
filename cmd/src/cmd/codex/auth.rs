use super::*;

pub(super) fn read_auth_identity(path: &Path) -> Result<AuthIdentity> {
    let auth = read_stored_auth(path)?;
    stored_auth_identity(&auth)
}

pub(super) fn read_stored_auth(path: &Path) -> Result<StoredAuth> {
    let auth = stdfs::read_to_string(path)?;
    Ok(serde_json::from_str(&auth)?)
}

pub(super) fn read_auth_snapshot(path: &Path) -> Result<AuthSnapshot> {
    let raw = stdfs::read(path)?;
    let identity = parse_auth_identity_bytes(&raw)?;
    Ok(AuthSnapshot { raw, identity })
}

pub(super) fn parse_auth_identity_bytes(raw: &[u8]) -> Result<AuthIdentity> {
    let auth = std::str::from_utf8(raw).wrap_err("auth.json is not valid UTF-8")?;
    parse_auth_identity(auth)
}

pub(super) fn promote_launch_auth_if_unchanged(
    profile_auth: &Path,
    launch_auth: &AuthSnapshot,
    final_launch_auth_path: &Path,
) -> Result<()> {
    let Ok(final_launch_raw) = stdfs::read(final_launch_auth_path) else {
        return Ok(());
    };
    if final_launch_raw == launch_auth.raw {
        return Ok(());
    }

    let Ok(final_launch_identity) = parse_auth_identity_bytes(&final_launch_raw) else {
        return Ok(());
    };
    if !matches_launch_identity(&launch_auth.identity, &final_launch_identity) {
        return Ok(());
    }

    write_auth_raw_if_unchanged(profile_auth, &launch_auth.raw, &final_launch_raw)?;
    Ok(())
}

pub(super) fn write_auth_raw_if_unchanged(
    path: &Path,
    expected_raw: &[u8],
    replacement_raw: &[u8],
) -> Result<bool> {
    let Ok(current_raw) = stdfs::read(path) else {
        return Ok(false);
    };
    if current_raw != expected_raw {
        return Ok(false);
    }

    stdfs::write(path, replacement_raw)?;
    Ok(true)
}

fn matches_launch_identity(initial: &AuthIdentity, final_auth: &AuthIdentity) -> bool {
    if !is_same_user(initial, final_auth) {
        return false;
    }

    match (
        initial.chatgpt_account_id.as_deref(),
        final_auth.chatgpt_account_id.as_deref(),
    ) {
        (Some(initial_account), Some(final_account)) => initial_account == final_account,
        _ => true,
    }
}

pub(super) fn stored_auth_identity(auth: &StoredAuth) -> Result<AuthIdentity> {
    parse_auth_identity(&serde_json::to_string(auth)?)
}

pub(super) fn parse_auth_identity(auth: &str) -> Result<AuthIdentity> {
    let auth: StoredAuth = serde_json::from_str(auth)?;
    let tokens = auth
        .tokens
        .ok_or_else(|| eyre!("Missing tokens in auth.json"))?;
    let id_token = tokens
        .id_token
        .as_deref()
        .ok_or_else(|| eyre!("Missing id_token in auth.json"))?;
    let claims = parse_id_token_claims(id_token)?;
    let openai_auth = claims.openai_auth.unwrap_or_default();

    Ok(AuthIdentity {
        auth_mode: auth.auth_mode,
        subject: claims.sub,
        user_id: openai_auth.user_id,
        chatgpt_account_id: openai_auth.chatgpt_account_id.or(tokens.account_id),
        email: claims.email,
        name: claims.name,
        auth_provider: claims.auth_provider,
    })
}

fn parse_id_token_claims(id_token: &str) -> Result<IdTokenClaims> {
    let payload = jwt_payload(id_token)?;
    Ok(serde_json::from_slice(&payload)?)
}

pub(super) fn jwt_payload(token: &str) -> Result<Vec<u8>> {
    let mut parts = token.split('.');
    let _header = parts.next().ok_or_else(|| eyre!("Malformed JWT"))?;
    let payload = parts.next().ok_or_else(|| eyre!("Malformed JWT"))?;
    let _signature = parts.next().ok_or_else(|| eyre!("Malformed JWT"))?;
    let padded = pad_base64url(payload);
    URL_SAFE_NO_PAD
        .decode(payload.as_bytes())
        .or_else(|_| URL_SAFE.decode(padded.as_bytes()))
        .wrap_err("Failed to decode JWT payload")
}

fn pad_base64url(value: &str) -> String {
    let mut value = value.to_owned();
    let padding = (4 - value.len() % 4) % 4;
    for _ in 0..padding {
        value.push('=');
    }
    value
}

pub(super) fn load_saved_profiles(dir: &Path) -> Result<Vec<SavedProfile>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut profiles = Vec::new();

    for entry in stdfs::read_dir(dir)? {
        let entry = entry?;
        let auth_path = entry.path().join("auth.json");
        if !auth_path.exists() {
            continue;
        }

        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };

        let (identity, invalid_auth) = match read_auth_identity(&auth_path) {
            Ok(identity) => (Some(identity), false),
            Err(_) => (None, true),
        };

        profiles.push(SavedProfile {
            name,
            auth_path,
            identity,
            invalid_auth,
            usage: ProfileUsageState::Unchecked,
        });
    }

    profiles.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(profiles)
}

pub(super) fn prompt_for_confirmation(prompt: &str) -> Result<bool> {
    print!("{prompt} [y/N] ");
    io::stdout().flush()?;

    let mut answer = String::new();
    io::stdin().read_line(&mut answer)?;
    Ok(matches!(
        answer.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

pub(super) fn save_profile_auth(
    profile: &str,
    auth_path: &Path,
    profiles_dir: &Path,
) -> Result<()> {
    let profile_dir = profiles_dir.join(profile);
    stdfs::create_dir_all(&profile_dir)?;
    stdfs::copy(auth_path, profile_dir.join("auth.json"))?;

    Ok(())
}

pub(super) fn is_same_user(left: &AuthIdentity, right: &AuthIdentity) -> bool {
    matches!(
        (&left.subject, &right.subject),
        (Some(left_subject), Some(right_subject)) if left_subject == right_subject
    ) || matches!(
        (&left.user_id, &right.user_id),
        (Some(left_user_id), Some(right_user_id)) if left_user_id == right_user_id
    )
}

pub(super) fn shares_account(left: &AuthIdentity, right: &AuthIdentity) -> bool {
    if is_same_user(left, right) {
        return false;
    }

    matches!(
        (&left.chatgpt_account_id, &right.chatgpt_account_id),
        (Some(left_account), Some(right_account)) if left_account == right_account
    )
}

pub(super) fn best_label(identity: &AuthIdentity) -> String {
    identity
        .email
        .clone()
        .or_else(|| identity.name.clone())
        .unwrap_or_else(|| "-".into())
}

pub(super) fn shorten_id(value: &str) -> String {
    if value.len() <= 16 {
        return value.to_string();
    }

    format!("{}…{}", &value[..10], &value[value.len() - 4..])
}
