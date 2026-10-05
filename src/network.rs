use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    net::{IpAddr, ToSocketAddrs},
    time::{Duration, Instant},
};
use url::Url;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Policy {
    pub hosts: Vec<String>,
    pub private: bool,
}
impl Policy {
    pub fn permits(&self, url: &Url) -> bool {
        let Some(host) = url.host_str() else {
            return false;
        };
        let port = url.port_or_known_default().unwrap_or(0);
        self.hosts
            .iter()
            .any(|grant| grant == host || grant == &format!("{host}:{port}"))
    }
}
fn public(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => {
            !(v.is_private()
                || v.is_loopback()
                || v.is_link_local()
                || v.is_multicast()
                || v.is_unspecified()
                || v.is_broadcast()
                || v.is_documentation()
                || v.octets()[0] == 0
                || v.octets()[0] >= 240
                || (v.octets()[0] == 192 && v.octets()[1] == 0 && v.octets()[2] == 0)
                || (v.octets()[0] == 198 && (18..=19).contains(&v.octets()[1]))
                || (v.octets()[0] == 100 && (64..=127).contains(&v.octets()[1])))
        }
        IpAddr::V6(v) => {
            if let Some(v4) = v.to_ipv4_mapped() {
                return public(IpAddr::V4(v4));
            }
            let s = v.segments();
            !v.is_loopback()
                && !v.is_unspecified()
                && !v.is_multicast()
                && (s[0] & 0xe000) == 0x2000
                && !(s[0] == 0x2001 && (s[1] < 0x0200 || s[1] == 0x0db8))
                && s[0] != 0x2002
        }
    }
}
pub struct Download {
    pub url: String,
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub bytes: Vec<u8>,
}
pub fn download(
    address: &str,
    policy: &Policy,
    max: usize,
    timeout: Duration,
    https_only: bool,
) -> Result<Download> {
    let deadline = Instant::now() + timeout;
    let mut url = Url::parse(address)?;
    for _ in 0..6 {
        ensure!(
            matches!(url.scheme(), "http" | "https"),
            "unsupported URL scheme"
        );
        ensure!(
            !https_only || url.scheme() == "https",
            "module imports require HTTPS"
        );
        ensure!(
            url.username().is_empty() && url.password().is_none(),
            "URL credentials are forbidden"
        );
        ensure!(
            policy.permits(&url),
            "network permission denied for {}",
            url.origin().ascii_serialization()
        );
        let host = url.host_str().context("URL has no host")?;
        let port = url.port_or_known_default().context("URL has no port")?;
        let addresses: Vec<_> = (host, port).to_socket_addrs()?.collect();
        ensure!(!addresses.is_empty(), "host has no addresses");
        ensure!(
            policy.private || addresses.iter().all(|a| public(a.ip())),
            "private or reserved network address denied"
        );
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .context("network deadline exceeded")?;
        let client = reqwest::blocking::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(remaining)
            .resolve_to_addrs(host, &addresses)
            .build()?;
        let mut response = client.get(url.clone()).send()?;
        if response.status().is_redirection() {
            let location = response
                .headers()
                .get("location")
                .context("redirect missing Location")?
                .to_str()?;
            url = url.join(location)?;
            continue;
        }
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .filter_map(|(k, v)| v.to_str().ok().map(|v| (k.to_string(), v.to_owned())))
            .collect();
        let mut bytes = Vec::new();
        response
            .by_ref()
            .take((max + 1) as u64)
            .read_to_end(&mut bytes)?;
        ensure!(bytes.len() <= max, "response exceeds byte limit");
        return Ok(Download {
            url: url.to_string(),
            status,
            headers,
            bytes,
        });
    }
    bail!("too many redirects")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grants_and_private_addresses() {
        let p = Policy {
            hosts: vec!["example.com:443".into()],
            private: false,
        };
        assert!(p.permits(&Url::parse("https://example.com/path").unwrap()));
        assert!(!p.permits(&Url::parse("https://example.com:444/path").unwrap()));
        for ip in [
            "127.0.0.1",
            "10.1.2.3",
            "169.254.169.254",
            "::1",
            "::ffff:127.0.0.1",
            "fc00::1",
        ] {
            assert!(!public(ip.parse().unwrap()));
        }
        assert!(public("8.8.8.8".parse().unwrap()));
    }
}
