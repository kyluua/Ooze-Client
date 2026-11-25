use crate::EngineState;

pub(crate) fn parse(line: &str) -> Option<EngineState> {
    let line = line
        .strip_suffix('\n')
        .map_or(line, |line| line.strip_suffix('\r').unwrap_or(line));
    let line = line.strip_prefix("[stdout] ").unwrap_or(line);
    let (clock, message) = line.split_at_checked(10)?;
    let clock = clock.strip_prefix('[')?.strip_suffix(']')?;
    let [h1, h2, b':', m1, m2, b':', s1, s2] = *clock.as_bytes() else {
        return None;
    };
    let number = |a: u8, b: u8, max| {
        a.is_ascii_digit() && b.is_ascii_digit() && (a - b'0') * 10 + (b - b'0') <= max
    };
    if !number(h1, h2, 23) || !number(m1, m2, 59) || !number(s1, s2, 59) {
        return None;
    }
    let state =
        message.strip_prefix(" [Render thread/INFO]: OOZE_READY v1 screen=title engine=")?;
    Some(match state {
        "LOADED" => EngineState::Loaded,
        "MISSING_LIBRARY" => EngineState::MissingLibrary,
        "UNSUPPORTED_PLATFORM" => EngineState::UnsupportedPlatform,
        "INCOMPATIBLE" => EngineState::Incompatible,
        "LOAD_FAILED" => EngineState::LoadFailed,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_an_exact_known_title_record_is_ready() {
        const MARKER: &str = "[15:16:59] [Render thread/INFO]: OOZE_READY v1 screen=title engine=";
        for (name, state) in [
            ("LOADED", EngineState::Loaded),
            ("MISSING_LIBRARY", EngineState::MissingLibrary),
            ("UNSUPPORTED_PLATFORM", EngineState::UnsupportedPlatform),
            ("INCOMPATIBLE", EngineState::Incompatible),
            ("LOAD_FAILED", EngineState::LoadFailed),
        ] {
            assert_eq!(parse(&format!("{MARKER}{name}")), Some(state));
            assert_eq!(parse(&format!("[stdout] {MARKER}{name}\r\n")), Some(state));
        }
        for line in [
            format!("other game output {MARKER}LOADED"),
            format!("[stderr] {MARKER}LOADED"),
            format!("{MARKER}UNKNOWN"),
            format!("{MARKER}LOADED "),
            format!("{MARKER}LOADED\n\n"),
            format!("{MARKER}LOADED additional text"),
            format!(" {MARKER}LOADED"),
            format!("{}LOADED", MARKER.replace("59]", "60]")),
            format!(
                "{}LOADED",
                MARKER.replace("[Render thread/INFO]", "[main/INFO]")
            ),
        ] {
            assert_eq!(parse(&line), None);
        }
    }
}
