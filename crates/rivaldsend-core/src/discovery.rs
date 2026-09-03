use std::collections::HashMap;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use crate::error::CoreError;

pub const SERVICE_TYPE: &str = "_rivaldsend._tcp.local.";
pub const SERVICE_NAME_PREFIX: &str = "rivaldsend-";

#[derive(Debug, Clone)]
pub struct DiscoveredPeer {
    pub instance_name: String,
    pub hostname: String,
    pub port: u16,
    pub addresses: Vec<std::net::IpAddr>,
    pub txt: HashMap<String, String>,
}

pub struct Discovery {
    daemon: ServiceDaemon,
}

impl Discovery {
    pub fn new() -> Result<Self, CoreError> {
        let daemon = ServiceDaemon::new().map_err(|e| CoreError::IoString(e.to_string()))?;
        Ok(Self { daemon })
    }

    pub fn register(&self, device_name: &str, port: u16, fingerprint_short: &str, protocol_version: &str) -> Result<ServiceInfo, CoreError> {
        let instance = format!("{}{}", SERVICE_NAME_PREFIX, device_name.replace(' ', "-"));
        let host = format!("{}.local.", instance);
        let props: HashMap<String, String> = HashMap::from([
            ("protocol_version".into(), protocol_version.into()),
            ("device_name".into(), device_name.into()),
            ("fingerprint_short".into(), fingerprint_short.into()),
        ]);
        let info = ServiceInfo::new(SERVICE_TYPE, &instance, &host, "", port, props).map_err(|e| CoreError::IoString(e.to_string()))?;
        self.daemon.register(info.clone()).map_err(|e| CoreError::IoString(e.to_string()))?;
        Ok(info)
    }

    pub fn browse(&self) -> Result<mdns_sd::ServiceDaemon, CoreError> {
        Ok(self.daemon.clone())
    }

    pub fn shutdown(self) -> Result<(), CoreError> {
        self.daemon.shutdown().map_err(|e| CoreError::IoString(e.to_string()))?;
        Ok(())
    }
}

pub fn list_interfaces() -> Vec<(String, std::net::IpAddr)> {
    let mut out = Vec::new();
    if let Ok(ifaces) = if_addrs::get_if_addrs() {
        for iface in ifaces {
            let ip = iface.ip();
            if ip.is_loopback() {
                continue;
            }
            if iface.name.starts_with("docker") || iface.name.starts_with("br-") || iface.name.contains("veth") {
                continue;
            }
            out.push((iface.name, ip));
        }
    }
    out
}
