//! `--page <id>` CLI arg: launches straight into a specific settings page
//! instead of Home. Paired with the single-instance plugin registered in
//! `lib.rs`, a second launch with `--page bluetooth` while bos-settings is
//! already running re-navigates the existing window instead of spawning a
//! duplicate process.
//!
//! Unlike `screenshot::parse`'s `KNOWN_VIEWS`, this accepts any id
//! unvalidated: `frontend/src/lib/sidebar.ts::resolvePage` already falls
//! back to rendering a `Placeholder` for an id it doesn't recognise (see
//! `+page.svelte`), so there is no second id list to keep in sync here.

/// Last `--page <id>` value in `args`, or `None` if the flag isn't present
/// (or has no value after it). `args[0]` is the executable path, both for
/// the initial process's `std::env::args()` and for a second instance's
/// argv handed to the single-instance callback, so this mirrors
/// `screenshot::parse`'s `skip(1)`.
pub fn parse(args: &[String]) -> Option<String> {
    let mut page = None;
    let mut it = args.iter().skip(1);
    while let Some(arg) = it.next() {
        if arg == "--page" {
            page = it.next().cloned();
        }
    }
    page
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn parse_no_page_flag_is_none() {
        assert_eq!(parse(&args(&["bos-settings"])), None);
    }

    #[test]
    fn parse_reads_the_value() {
        assert_eq!(
            parse(&args(&["bos-settings", "--page", "bluetooth"])),
            Some("bluetooth".to_string())
        );
    }

    #[test]
    fn parse_missing_value_is_none() {
        assert_eq!(parse(&args(&["bos-settings", "--page"])), None);
    }

    #[test]
    fn parse_last_occurrence_wins() {
        assert_eq!(
            parse(&args(&[
                "bos-settings",
                "--page",
                "network",
                "--page",
                "bluetooth"
            ])),
            Some("bluetooth".to_string())
        );
    }

    #[test]
    fn parse_ignores_unrelated_flags() {
        assert_eq!(
            parse(&args(&[
                "bos-settings",
                "--screenshot",
                "home",
                "--output",
                "/tmp/x.png"
            ])),
            None
        );
    }
}
