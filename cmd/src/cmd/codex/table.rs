use super::*;

pub(super) fn print_compact_profile_table(rows: &[ProfileRow]) {
    let widths = compact_profile_table_widths(rows);

    println!(
        "{}   {}   {}   {}",
        format!(
            "{:<profile_width$}",
            "PROFILE",
            profile_width = widths.profile
        )
        .blue()
        .bold(),
        format!("{:<label_width$}", "EMAIL", label_width = widths.label)
            .blue()
            .bold(),
        format!(
            "{:<five_hour_width$}",
            "5 HOUR LIMIT",
            five_hour_width = widths.five_hour
        )
        .blue()
        .bold(),
        format!(
            "{:<weekly_width$}",
            "WEEKLY LIMIT",
            weekly_width = widths.weekly
        )
        .blue()
        .bold(),
    );

    for row in rows {
        println!(
            "{}   {}   {}   {}",
            colorize_profile_cell(&row.profile, widths.profile, row),
            colorize_email_cell(&row.label, widths.label, row),
            colorize_limit_cell(
                &row.five_hour_compact,
                widths.five_hour,
                row.five_hour_style,
                row,
            ),
            colorize_limit_cell(&row.weekly_compact, widths.weekly, row.weekly_style, row),
        );
    }
}

pub(super) fn print_verbose_profile_table(rows: &[ProfileRow]) {
    let widths = profile_table_widths(rows);

    println!(
        "{}  {}  {}  {}  {}  {}  {}  {}  {}  {}  {}",
        format!(
            "{:<profile_width$}",
            "PROFILE",
            profile_width = widths.profile
        )
        .blue()
        .bold(),
        format!("{:<label_width$}", "EMAIL", label_width = widths.label)
            .blue()
            .bold(),
        format!(
            "{:<provider_width$}",
            "PROVIDER",
            provider_width = widths.provider
        )
        .blue()
        .bold(),
        format!("{:<user_width$}", "USER", user_width = widths.user)
            .blue()
            .bold(),
        format!(
            "{:<account_width$}",
            "ACCOUNT",
            account_width = widths.account
        )
        .blue()
        .bold(),
        format!("{:<plan_width$}", "PLAN", plan_width = widths.plan)
            .blue()
            .bold(),
        format!(
            "{:<five_hour_width$}",
            "5H",
            five_hour_width = widths.five_hour
        )
        .blue()
        .bold(),
        format!(
            "{:<five_hour_reset_width$}",
            "5H RESET",
            five_hour_reset_width = widths.five_hour_reset
        )
        .blue()
        .bold(),
        format!("{:<weekly_width$}", "WEEK", weekly_width = widths.weekly)
            .blue()
            .bold(),
        format!(
            "{:<weekly_reset_width$}",
            "WEEK RESET",
            weekly_reset_width = widths.weekly_reset
        )
        .blue()
        .bold(),
        "STATUS".blue().bold(),
    );

    for row in rows {
        println!(
            "{}  {}  {}  {}  {}  {}  {}  {}  {}  {}  {}",
            colorize_profile_cell(&row.profile, widths.profile, row),
            colorize_email_cell(&row.label, widths.label, row),
            colorize_row_cell(&row.provider, widths.provider, row),
            colorize_row_cell(&row.user, widths.user, row),
            colorize_row_cell(&row.account, widths.account, row),
            colorize_row_cell(&row.plan, widths.plan, row),
            colorize_limit_cell(&row.five_hour, widths.five_hour, row.five_hour_style, row),
            colorize_row_cell(&row.five_hour_reset, widths.five_hour_reset, row),
            colorize_limit_cell(&row.weekly, widths.weekly, row.weekly_style, row),
            colorize_row_cell(&row.weekly_reset, widths.weekly_reset, row),
            colorize_status(row),
        );
    }
}

pub(super) fn print_refresh_all_rows(rows: &[RefreshAllRow]) {
    let profile_width = rows
        .iter()
        .map(|row| row.profile.len())
        .max()
        .unwrap_or("PROFILE".len())
        .max("PROFILE".len());
    let result_width = rows
        .iter()
        .map(|row| row.result.text().len())
        .max()
        .unwrap_or("RESULT".len())
        .max("RESULT".len());

    println!(
        "{}  {}  {}",
        format!(
            "{:<profile_width$}",
            "PROFILE",
            profile_width = profile_width
        )
        .blue()
        .bold(),
        format!("{:<result_width$}", "RESULT", result_width = result_width)
            .blue()
            .bold(),
        "DETAIL".blue().bold(),
    );

    for row in rows {
        println!(
            "{:<profile_width$}  {}  {}",
            row.profile,
            row.result.render(result_width),
            row.detail,
            profile_width = profile_width,
        );
    }
}

pub(super) fn colorize_row_cell(value: &str, width: usize, row: &ProfileRow) -> String {
    let padded = format!("{value:<width$}");
    match row.status.whole_row_style() {
        ProfileStyleKind::Error => colorize_ansi(&padded, "31", true),
        _ => padded,
    }
}

pub(super) fn colorize_profile_cell(value: &str, width: usize, row: &ProfileRow) -> String {
    let padded = format!("{value:<width$}");
    match row.status.whole_row_style() {
        ProfileStyleKind::Error => colorize_ansi(&padded, "31", true),
        _ if row.status.is_active() => colorize_active_cell(&padded),
        _ => padded,
    }
}

pub(super) fn colorize_email_cell(value: &str, width: usize, row: &ProfileRow) -> String {
    let padded = format!("{value:<width$}");
    match row.status.whole_row_style() {
        ProfileStyleKind::Error => colorize_ansi(&padded, "31", true),
        _ if row.status.is_active() => colorize_active_cell(&padded),
        _ => padded,
    }
}

fn colorize_active_cell(value: &str) -> String {
    colorize_ansi(value, "97", true)
}

pub(super) fn colorize_limit_cell(
    value: &str,
    width: usize,
    style: LimitStyleKind,
    row: &ProfileRow,
) -> String {
    let padded = format!("{value:<width$}");
    if row.status.whole_row_style() == ProfileStyleKind::Error {
        return colorize_ansi(&padded, "31", true);
    }
    if row.status.is_active() {
        return match style {
            LimitStyleKind::Normal => padded.bold().to_string(),
            LimitStyleKind::Success => colorize_ansi(&padded, "32", true),
            LimitStyleKind::Warning => colorize_ansi(&padded, "33", true),
            LimitStyleKind::Caution => colorize_ansi(&padded, "38;2;255;165;0", true),
            LimitStyleKind::Error | LimitStyleKind::Critical => colorize_ansi(&padded, "31", true),
        };
    }

    match style {
        LimitStyleKind::Normal => padded,
        LimitStyleKind::Success => colorize_ansi(&padded, "32", false),
        LimitStyleKind::Warning => colorize_ansi(&padded, "33", false),
        LimitStyleKind::Caution => colorize_ansi(&padded, "38;2;255;165;0", false),
        LimitStyleKind::Error => colorize_ansi(&padded, "31", false),
        LimitStyleKind::Critical => colorize_ansi(&padded, "31", true),
    }
}

fn colorize_ansi(value: &str, code: &str, bold: bool) -> String {
    if !colored::control::SHOULD_COLORIZE.should_colorize() {
        return value.to_string();
    }

    if bold {
        format!("\u{1b}[1;{code}m{value}\u{1b}[0m")
    } else {
        format!("\u{1b}[{code}m{value}\u{1b}[0m")
    }
}

fn colorize_status(row: &ProfileRow) -> String {
    row.status.render(row.status.whole_row_style())
}

impl ProfileStatus {
    fn push(&mut self, item: ProfileStatusItem) {
        self.items.push(item);
    }

    fn is_active(&self) -> bool {
        self.items
            .iter()
            .any(|item| matches!(item, ProfileStatusItem::Active))
    }

    pub(super) fn text(&self) -> String {
        if self.items.is_empty() {
            return "-".into();
        }

        self.items
            .iter()
            .map(ProfileStatusItem::text)
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub(super) fn whole_row_style(&self) -> ProfileStyleKind {
        if self
            .items
            .iter()
            .any(|item| matches!(item, ProfileStatusItem::SameUser(_)))
        {
            ProfileStyleKind::Error
        } else {
            ProfileStyleKind::Normal
        }
    }

    fn render(&self, whole_row_style: ProfileStyleKind) -> String {
        if self.items.is_empty() {
            return "-".into();
        }

        if whole_row_style == ProfileStyleKind::Error {
            return self.text().red().bold().to_string();
        }

        self.items
            .iter()
            .map(ProfileStatusItem::render)
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl ProfileStatusItem {
    fn text(&self) -> String {
        match self {
            Self::Active => "active".into(),
            Self::SameUser(profiles) => format!("same-user-as:{}", profiles.join(",")),
            Self::SharedAccount(profiles) => {
                format!("shared-account-with:{}", profiles.join(","))
            }
            Self::InvalidAuth => "invalid-auth".into(),
            Self::ReauthNeeded => "reauth-needed".into(),
            Self::UsageUnavailable => "usage-unavailable".into(),
        }
    }

    fn style_kind(&self) -> ProfileStyleKind {
        match self {
            Self::Active => ProfileStyleKind::Success,
            Self::SameUser(_) => ProfileStyleKind::Error,
            Self::SharedAccount(_)
            | Self::InvalidAuth
            | Self::ReauthNeeded
            | Self::UsageUnavailable => ProfileStyleKind::Warning,
        }
    }

    fn render(&self) -> String {
        let text = self.text();
        match self.style_kind() {
            ProfileStyleKind::Success => text.green().to_string(),
            ProfileStyleKind::Warning => text.yellow().to_string(),
            ProfileStyleKind::Error => text.red().bold().to_string(),
            ProfileStyleKind::Normal => text,
        }
    }
}

impl RefreshAllResultKind {
    fn text(&self) -> &'static str {
        match self {
            Self::Refreshed => "refreshed",
            Self::Fresh => "fresh",
            Self::Deferred => "deferred",
            Self::Invalid => "invalid",
            Self::Failed => "failed",
        }
    }

    fn render(&self, width: usize) -> String {
        let padded = format!("{:<width$}", self.text());
        match self {
            Self::Refreshed => padded.green().to_string(),
            Self::Fresh => padded,
            Self::Deferred => padded.yellow().to_string(),
            Self::Invalid | Self::Failed => padded.red().bold().to_string(),
        }
    }
}

pub(super) fn build_profile_rows(
    profiles: &[SavedProfile],
    active_identity: Option<&AuthIdentity>,
) -> Vec<ProfileRow> {
    profiles
        .iter()
        .map(|profile| {
            let Some(identity) = profile.identity.as_ref() else {
                return ProfileRow {
                    profile: profile.name.clone(),
                    label: "-".into(),
                    provider: "-".into(),
                    user: "-".into(),
                    account: "-".into(),
                    plan: "-".into(),
                    five_hour: "-".into(),
                    five_hour_reset: "-".into(),
                    five_hour_compact: "-".into(),
                    five_hour_style: LimitStyleKind::Normal,
                    weekly: "-".into(),
                    weekly_reset: "-".into(),
                    weekly_compact: "-".into(),
                    weekly_style: LimitStyleKind::Normal,
                    status: ProfileStatus {
                        items: if profile.invalid_auth {
                            vec![ProfileStatusItem::InvalidAuth]
                        } else {
                            Vec::new()
                        },
                    },
                };
            };

            let mut status = ProfileStatus::default();

            if active_identity.is_some_and(|active| is_same_user(active, identity)) {
                status.push(ProfileStatusItem::Active);
            }

            let same_user_profiles = profiles
                .iter()
                .filter(|other| other.name != profile.name)
                .filter_map(|other| {
                    other
                        .identity
                        .as_ref()
                        .filter(|other_identity| is_same_user(identity, other_identity))
                        .map(|_| other.name.clone())
                })
                .collect::<Vec<_>>();

            if !same_user_profiles.is_empty() {
                status.push(ProfileStatusItem::SameUser(same_user_profiles));
            }

            let shared_account_profiles = profiles
                .iter()
                .filter(|other| other.name != profile.name)
                .filter_map(|other| {
                    other
                        .identity
                        .as_ref()
                        .filter(|other_identity| shares_account(identity, other_identity))
                        .map(|_| other.name.clone())
                })
                .collect::<Vec<_>>();

            if !shared_account_profiles.is_empty() {
                status.push(ProfileStatusItem::SharedAccount(shared_account_profiles));
            }

            match profile.usage {
                ProfileUsageState::ReauthNeeded => status.push(ProfileStatusItem::ReauthNeeded),
                ProfileUsageState::Unavailable => status.push(ProfileStatusItem::UsageUnavailable),
                ProfileUsageState::Unchecked | ProfileUsageState::Available(_) => {}
            }

            ProfileRow {
                profile: profile.name.clone(),
                label: best_label(identity),
                provider: identity
                    .auth_provider
                    .clone()
                    .or_else(|| identity.auth_mode.clone())
                    .unwrap_or_else(|| "-".into()),
                user: identity
                    .user_id
                    .as_deref()
                    .map(shorten_id)
                    .unwrap_or_else(|| {
                        identity
                            .subject
                            .as_deref()
                            .map(shorten_id)
                            .unwrap_or_else(|| "-".into())
                    }),
                account: identity
                    .chatgpt_account_id
                    .as_deref()
                    .map(shorten_id)
                    .unwrap_or_else(|| "-".into()),
                plan: usage_plan(&profile.usage),
                five_hour: usage_window_percent(&profile.usage, UsageWindowKind::Primary),
                five_hour_reset: usage_window_reset(&profile.usage, UsageWindowKind::Primary),
                five_hour_compact: usage_window_compact(&profile.usage, UsageWindowKind::Primary),
                five_hour_style: five_hour_limit_style(&profile.usage),
                weekly: usage_window_percent(&profile.usage, UsageWindowKind::Secondary),
                weekly_reset: usage_window_reset(&profile.usage, UsageWindowKind::Secondary),
                weekly_compact: usage_window_compact(&profile.usage, UsageWindowKind::Secondary),
                weekly_style: usage_window_style(&profile.usage, UsageWindowKind::Secondary),
                status,
            }
        })
        .collect()
}

fn profile_table_widths(rows: &[ProfileRow]) -> ProfileTableWidths {
    rows.iter().fold(
        ProfileTableWidths {
            profile: "PROFILE".len(),
            label: "EMAIL".len(),
            provider: "PROVIDER".len(),
            user: "USER".len(),
            account: "ACCOUNT".len(),
            plan: "PLAN".len(),
            five_hour: "5H".len(),
            five_hour_reset: "5H RESET".len(),
            weekly: "WEEK".len(),
            weekly_reset: "WEEK RESET".len(),
        },
        |widths, row| ProfileTableWidths {
            profile: widths.profile.max(row.profile.len()),
            label: widths.label.max(row.label.len()),
            provider: widths.provider.max(row.provider.len()),
            user: widths.user.max(row.user.len()),
            account: widths.account.max(row.account.len()),
            plan: widths.plan.max(row.plan.len()),
            five_hour: widths.five_hour.max(row.five_hour.len()),
            five_hour_reset: widths.five_hour_reset.max(row.five_hour_reset.len()),
            weekly: widths.weekly.max(row.weekly.len()),
            weekly_reset: widths.weekly_reset.max(row.weekly_reset.len()),
        },
    )
}

fn compact_profile_table_widths(rows: &[ProfileRow]) -> CompactProfileTableWidths {
    rows.iter().fold(
        CompactProfileTableWidths {
            profile: "PROFILE".len(),
            label: "EMAIL".len(),
            five_hour: "5 HOUR LIMIT".len(),
            weekly: "WEEKLY LIMIT".len(),
        },
        |widths, row| CompactProfileTableWidths {
            profile: widths.profile.max(row.profile.len()),
            label: widths.label.max(row.label.len()),
            five_hour: widths.five_hour.max(row.five_hour_compact.len()),
            weekly: widths.weekly.max(row.weekly_compact.len()),
        },
    )
}

#[derive(Debug, Clone, Copy)]
pub(super) enum UsageWindowKind {
    Primary,
    Secondary,
}

fn usage_plan(usage: &ProfileUsageState) -> String {
    match usage {
        ProfileUsageState::Available(snapshot) => snapshot
            .plan_type
            .as_deref()
            .map(title_case)
            .unwrap_or_else(|| "-".into()),
        _ => "-".into(),
    }
}

fn usage_window_percent(usage: &ProfileUsageState, kind: UsageWindowKind) -> String {
    usage_window(usage, kind)
        .map(|window| format!("{:.0}%", window.used_percent))
        .unwrap_or_else(|| "-".into())
}

pub(super) fn usage_window_style(
    usage: &ProfileUsageState,
    kind: UsageWindowKind,
) -> LimitStyleKind {
    usage_window(usage, kind)
        .map(|window| limit_style(window.used_percent))
        .unwrap_or(LimitStyleKind::Normal)
}

pub(super) fn five_hour_limit_style(usage: &ProfileUsageState) -> LimitStyleKind {
    let weekly_exhausted = usage_window(usage, UsageWindowKind::Secondary)
        .is_some_and(|window| format!("{:.0}", window.used_percent) == "100");

    if weekly_exhausted {
        LimitStyleKind::Critical
    } else {
        usage_window_style(usage, UsageWindowKind::Primary)
    }
}

pub(super) fn usage_window_reset(usage: &ProfileUsageState, kind: UsageWindowKind) -> String {
    usage_window(usage, kind)
        .filter(|window| window.used_percent > 0.0)
        .and_then(|window| window.reset_at)
        .and_then(|timestamp| Local.timestamp_opt(timestamp, 0).single())
        .map(|timestamp| format_reset_timestamp(timestamp, Local::now()))
        .unwrap_or_else(|| "-".into())
}

pub(super) fn usage_window_compact(usage: &ProfileUsageState, kind: UsageWindowKind) -> String {
    let percent = usage_window_percent(usage, kind);
    let reset = usage_window_reset_compact(usage, kind);

    match (percent.as_str(), reset.as_str()) {
        ("-", _) => "-".into(),
        (_, "-") => format_compact_percent(&percent),
        _ => format!("{} ({reset})", format_compact_percent(&percent)),
    }
}

pub(super) fn format_compact_percent(percent: &str) -> String {
    let Some(number) = percent.strip_suffix('%') else {
        return percent.to_string();
    };

    format!("{number:>3}%")
}

fn usage_window_reset_compact(usage: &ProfileUsageState, kind: UsageWindowKind) -> String {
    usage_window(usage, kind)
        .filter(|window| window.used_percent > 0.0)
        .and_then(|window| window.reset_at)
        .and_then(|timestamp| Local.timestamp_opt(timestamp, 0).single())
        .map(|timestamp| format_reset_timestamp_compact(timestamp, Local::now(), kind))
        .unwrap_or_else(|| "-".into())
}

fn usage_window(usage: &ProfileUsageState, kind: UsageWindowKind) -> Option<&UsageWindowSnapshot> {
    match usage {
        ProfileUsageState::Available(snapshot) => match kind {
            UsageWindowKind::Primary => snapshot.primary.as_ref(),
            UsageWindowKind::Secondary => snapshot.secondary.as_ref(),
        },
        _ => None,
    }
}

pub(super) fn limit_style(used_percent: f64) -> LimitStyleKind {
    if used_percent < 50.0 {
        LimitStyleKind::Success
    } else if used_percent < 80.0 {
        LimitStyleKind::Warning
    } else if used_percent <= 90.0 {
        LimitStyleKind::Caution
    } else if used_percent <= 95.0 {
        LimitStyleKind::Error
    } else {
        LimitStyleKind::Critical
    }
}

pub(super) fn format_reset_timestamp(
    dt: chrono::DateTime<Local>,
    captured_at: chrono::DateTime<Local>,
) -> String {
    let time = dt.format("%-I:%M %p").to_string();
    if dt.date_naive() == captured_at.date_naive() {
        time
    } else {
        format!("{time} on {}", dt.format("%-d %b"))
    }
}

pub(super) fn format_reset_timestamp_compact(
    dt: chrono::DateTime<Local>,
    captured_at: chrono::DateTime<Local>,
    kind: UsageWindowKind,
) -> String {
    let time = dt.format("%-I:%M %p").to_string();

    match kind {
        UsageWindowKind::Primary => time,
        UsageWindowKind::Secondary if dt.date_naive() == captured_at.date_naive() => time,
        UsageWindowKind::Secondary => format!("{} {time}", dt.format("%a")),
    }
}

fn title_case(value: &str) -> String {
    if value.is_empty() {
        return String::new();
    }

    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let rest = chars.as_str().to_ascii_lowercase();
    first.to_uppercase().collect::<String>() + &rest
}
