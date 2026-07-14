use std::collections::HashMap;

use crate::minecraft::version::Rule;

/// The current OS name as Mojang spells it in version manifests.
pub fn os_name() -> &'static str {
    match std::env::consts::OS {
        "macos" => "osx",
        "windows" => "windows",
        _ => "linux",
    }
}

/// The current CPU architecture as Mojang spells it in rules/classifiers.
pub fn os_arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "arm64",
        "x86" => "x86",
        other => other,
    }
}

/// Evaluate a rule list against the current OS and the given feature flags.
/// Default is deny; each matching rule flips the decision to its action.
pub fn rules_allow(rules: &[Rule], features: &HashMap<String, bool>) -> bool {
    if rules.is_empty() {
        return true;
    }
    let mut allowed = false;
    for rule in rules {
        if rule_matches(rule, features) {
            allowed = rule.action == "allow";
        }
    }
    allowed
}

fn rule_matches(rule: &Rule, features: &HashMap<String, bool>) -> bool {
    if let Some(os) = &rule.os {
        if let Some(name) = &os.name {
            if name != os_name() {
                return false;
            }
        }
        if let Some(arch) = &os.arch {
            // Manifests sometimes use "x86" loosely; match exactly otherwise.
            if arch != os_arch() {
                return false;
            }
        }
        // os.version is a regex against the kernel version; we don't gate on it.
    }

    if let Some(required) = &rule.features {
        for (key, want) in required {
            if features.get(key).copied().unwrap_or(false) != *want {
                return false;
            }
        }
    }

    true
}
