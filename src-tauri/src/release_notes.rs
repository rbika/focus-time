pub fn release_notes_for_available_update(
    changelog: Option<&str>,
    installed: &str,
    latest_version: &str,
    latest_body: Option<&str>,
) -> Option<String> {
    match changelog {
        Some(text) => match assemble_release_notes(text, installed) {
            Assemble::Notes(notes) => Some(notes),
            Assemble::Empty => None,
            Assemble::Unparsed => fallback_release_notes(latest_version, latest_body),
        },
        None => fallback_release_notes(latest_version, latest_body),
    }
}

enum Assemble {
    Notes(String),
    Empty,
    Unparsed,
}

fn assemble_release_notes(changelog: &str, installed: &str) -> Assemble {
    let Some(installed) = parse_version(installed) else {
        return Assemble::Unparsed;
    };
    let mut sections = Vec::new();
    let mut current: Option<(Version, String)> = None;

    for line in changelog.lines() {
        if let Some(version) = parse_version_heading(line) {
            if let Some(section) = current.take() {
                sections.push(section);
            }
            current = Some((version, String::new()));
            continue;
        }
        if let Some((_, body)) = current.as_mut() {
            body.push_str(line);
            body.push('\n');
        }
    }
    if let Some(section) = current {
        sections.push(section);
    }

    if sections.is_empty() {
        return Assemble::Unparsed;
    }

    let notes: Vec<String> = sections
        .into_iter()
        .filter(|(version, _)| *version > installed)
        .filter_map(|(version, body)| {
            let trimmed = body.trim();
            if trimmed.is_empty() {
                return None;
            }
            Some(format_notes(version, trimmed))
        })
        .collect();

    if notes.is_empty() {
        Assemble::Empty
    } else {
        Assemble::Notes(notes.join("\n\n"))
    }
}

fn fallback_release_notes(latest_version: &str, latest_body: Option<&str>) -> Option<String> {
    let trimmed = latest_body?.trim();
    if trimmed.is_empty() {
        return None;
    }
    let version = parse_version(latest_version)?;
    Some(format_notes(version, trimmed))
}

fn format_notes(version: Version, body: &str) -> String {
    format!("## v{}.{}.{}\n\n{body}", version.0, version.1, version.2)
}

type Version = (u64, u64, u64);

fn parse_version_heading(line: &str) -> Option<Version> {
    let rest = line.trim().strip_prefix("## ")?;
    let rest = rest.strip_prefix('v').or_else(|| rest.strip_prefix('V'))?;
    parse_version(rest)
}

fn parse_version(label: &str) -> Option<Version> {
    let label = label.trim().trim_start_matches(['v', 'V']);
    let mut parts = label.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn includes_every_published_version_newer_than_installed() {
        let changelog = "\
## v1.16.1

### Improved

- Latest fix.

## v1.16.0

### Improved

- Middle change.

## v1.15.0

### New

- Already installed.
";

        let notes = release_notes_for_available_update(
            Some(changelog),
            "1.15.0",
            "1.16.1",
            Some("Latest fix."),
        )
        .expect("notes");

        assert_eq!(
            notes,
            "\
## v1.16.1

### Improved

- Latest fix.

## v1.16.0

### Improved

- Middle change."
        );
    }

    #[test]
    fn omits_empty_newer_versions() {
        let changelog = "\
## v1.16.1

### Improved

- Latest fix.

## v1.16.0

## v1.15.0

### New

- Already installed.
";

        let notes = release_notes_for_available_update(
            Some(changelog),
            "1.15.0",
            "1.16.1",
            Some("Latest fix."),
        )
        .expect("notes");

        assert_eq!(
            notes,
            "\
## v1.16.1

### Improved

- Latest fix."
        );
    }

    #[test]
    fn empty_newer_versions_yield_no_notes() {
        let changelog = "\
## v1.16.1

## v1.15.0

### New

- Already installed.
";

        assert_eq!(
            release_notes_for_available_update(
                Some(changelog),
                "1.15.0",
                "1.16.1",
                Some("Latest.")
            ),
            None
        );
    }

    #[test]
    fn improved_heading_is_not_a_version_boundary() {
        let changelog = "\
## v1.13.1

## Improved

- Updated the completion sounds list.

## v1.13.0

### New

- Already installed.
";

        let notes = release_notes_for_available_update(
            Some(changelog),
            "1.13.0",
            "1.13.1",
            Some("- Updated the completion sounds list."),
        )
        .expect("notes");

        assert_eq!(
            notes,
            "\
## v1.13.1

## Improved

- Updated the completion sounds list."
        );
    }

    #[test]
    fn fetch_failure_falls_back_to_latest_body() {
        let notes = release_notes_for_available_update(
            None,
            "1.15.0",
            "1.16.1",
            Some("### Improved\n\n- Latest fix."),
        )
        .expect("notes");

        assert_eq!(
            notes,
            "\
## v1.16.1

### Improved

- Latest fix."
        );
    }

    #[test]
    fn fetch_failure_with_empty_body_yields_no_notes() {
        assert_eq!(
            release_notes_for_available_update(None, "1.15.0", "1.16.1", Some("   ")),
            None
        );
    }

    #[test]
    fn unparseable_changelog_falls_back_to_latest_body() {
        let notes = release_notes_for_available_update(
            Some("not a changelog"),
            "1.15.0",
            "1.16.1",
            Some("### Improved\n\n- Latest fix."),
        )
        .expect("notes");

        assert_eq!(
            notes,
            "\
## v1.16.1

### Improved

- Latest fix."
        );
    }
}
