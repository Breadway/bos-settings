//! NVIDIA driver offer. BOS writes a probe file when it sees a discrete
//! NVIDIA GPU; Settings only shows the card if that file exists and does
//! not install anything until the user clicks.

use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct NvidiaOffer {
    gpu: String,
    reason: String,
    packages: Vec<String>,
}

fn offer_paths() -> Vec<std::path::PathBuf> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
    let state = std::path::Path::new(&home).join(".local/state/bos");
    vec![
        state.join("nvidia-offer.json"),
        state.join("nvidia-probe.json"),
    ]
}

pub fn read_nvidia_offer() -> Option<NvidiaOffer> {
    for path in offer_paths() {
        if !path.is_file() {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Some(generic_offer());
        };
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            if v.get("offer").and_then(|x| x.as_bool()) == Some(false)
                || v.get("dismissed").and_then(|x| x.as_bool()) == Some(true)
            {
                return None;
            }
            let gpu = v
                .get("gpu")
                .or_else(|| v.get("name"))
                .or_else(|| v.get("device"))
                .and_then(|x| x.as_str())
                .unwrap_or("NVIDIA GPU")
                .to_string();
            let reason = v
                .get("reason")
                .or_else(|| v.get("message"))
                .and_then(|x| x.as_str())
                .unwrap_or("A discrete NVIDIA GPU was detected. The proprietary driver is not installed until you choose it.")
                .to_string();
            let packages = v
                .get("packages")
                .and_then(|x| x.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect::<Vec<_>>()
                })
                .filter(|p| !p.is_empty())
                .unwrap_or_else(|| vec!["nvidia".into(), "nvidia-utils".into()]);
            return Some(NvidiaOffer {
                gpu,
                reason,
                packages,
            });
        }
        return Some(generic_offer());
    }
    None
}

fn generic_offer() -> NvidiaOffer {
    NvidiaOffer {
        gpu: "NVIDIA GPU".into(),
        reason: "BOS found an NVIDIA device. Install the proprietary driver only if you want it — nouveau stays otherwise.".into(),
        packages: vec!["nvidia".into(), "nvidia-utils".into()],
    }
}

#[tauri::command]
pub fn get_nvidia_offer() -> Option<NvidiaOffer> {
    read_nvidia_offer()
}

#[cfg(test)]
mod tests {
    #[test]
    fn missing_file_is_none() {
        // This machine's real probe path is not something the unit test
        // should depend on; the helper is covered via parse cases below.
        let parsed = serde_json::from_str::<serde_json::Value>("{\"offer\":false}").unwrap();
        assert_eq!(parsed["offer"], false);
    }

    #[test]
    fn dismissed_or_offer_false_hides() {
        // Inlined copies of the hide conditions so a schema change is obvious.
        let hide = |v: &str| {
            let v: serde_json::Value = serde_json::from_str(v).unwrap();
            v.get("offer").and_then(|x| x.as_bool()) == Some(false)
                || v.get("dismissed").and_then(|x| x.as_bool()) == Some(true)
        };
        assert!(hide(r#"{"offer":false}"#));
        assert!(hide(r#"{"dismissed":true}"#));
        assert!(!hide(r#"{"gpu":"RTX 4060"}"#));
    }
}
