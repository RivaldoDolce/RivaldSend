use crate::error::CoreError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NetworkProfile { Private, Public, Domain, Unknown }

pub fn detect_windows_firewall() -> Result<NetworkProfile, CoreError> {
    #[cfg(windows)]
    {
        let output = std::process::Command::new("powershell")
            .args(["-Command", "Get-NetConnectionProfile | Select-Object -ExpandProperty NetworkCategory"])
            .output()
            .map_err(|e| CoreError::IoString(e.to_string()))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
        if stdout.contains("public") { Ok(NetworkProfile::Public) }
        else if stdout.contains("private") { Ok(NetworkProfile::Private) }
        else if stdout.contains("domain") { Ok(NetworkProfile::Domain) }
        else { Ok(NetworkProfile::Unknown) }
    }
    #[cfg(not(windows))]
    {
        Ok(NetworkProfile::Private)
    }
}

pub fn should_block_server(profile: &NetworkProfile) -> bool {
    matches!(profile, NetworkProfile::Public)
}
