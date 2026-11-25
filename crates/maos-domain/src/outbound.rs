//! Story 23-0 — the one outbound-endpoint policy (D-23-0-02 A, D-23-1-04 A).
//!
//! Every Epic 23 adapter that dials a URL a server handed MAOS, and Worker
//! egress (D-23-0-03 A), shares this rule: https only, no loopback, link-local,
//! private or reserved destination, no redirect. The module performs no DNS
//! itself (`ResolveHost` is injected), reads no environment and no clock.
//!
//! A refusal is typed by its class and never carries any part of the input.
//! Host names: `PolicedIo` vets every DNS answer once and binds the request to
//! those addresses through `IoSubsystemPort::bind` (Story 23-0b), so a later
//! DNS answer cannot move the connection. A port that cannot bind refuses a
//! name (`outbound_unbound_refused`). Only a port that declares
//! `never_follows_redirects` is wrapped. Literal-IP URLs have nothing to
//! rebind. Worker egress connects to the vetted addresses itself.

use crate::ports::io_subsystem::{HttpExchange, IoError, IoSubsystemPort};
use std::fmt;
use std::net::{IpAddr, SocketAddr};
use url::{Host, Url};

/// Destination class of one IP address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpClass {
    /// Public unicast: the only class every policy admits.
    Public,
    /// 127/8, `::1`.
    Loopback,
    /// 169.254/16, fe80::/10 (holds the cloud metadata address).
    LinkLocal,
    /// RFC 1918, fc00::/7.
    Private,
    /// Everything else that is not public: unspecified, multicast, broadcast,
    /// 0/8, 240/4, CGNAT, documentation, benchmarking and other special use.
    Reserved,
}

/// Classify one address. `Public` is exactly Worker egress's former
/// `public_ip` true-set (ADR-061); the rest is partitioned by class.
#[must_use]
pub fn classify_ip(ip: IpAddr) -> IpClass {
    match ip {
        IpAddr::V4(ip) => classify_v4(ip),
        IpAddr::V6(ip) => match ip.to_ipv4_mapped() {
            Some(mapped) => classify_v4(mapped),
            None => classify_v6(ip),
        },
    }
}

fn classify_v4(ip: std::net::Ipv4Addr) -> IpClass {
    let [a, b, _, _] = ip.octets();
    if ip.is_loopback() {
        IpClass::Loopback
    } else if ip.is_link_local() {
        IpClass::LinkLocal
    } else if ip.is_private() {
        IpClass::Private
    } else if !ip.is_multicast()
        && !ip.is_broadcast()
        && !ip.is_unspecified()
        && a != 0
        && a < 240
        && !(a == 100 && (64..=127).contains(&b))
        && !(a == 192 && (b == 0 || b == 2))
        && !(a == 198 && (b == 18 || b == 19 || b == 51))
        && !(a == 203 && b == 0)
    {
        IpClass::Public
    } else {
        IpClass::Reserved
    }
}

fn classify_v6(ip: std::net::Ipv6Addr) -> IpClass {
    let segments = ip.segments();
    if ip.is_loopback() {
        IpClass::Loopback
    } else if segments[0] & 0xffc0 == 0xfe80 {
        IpClass::LinkLocal
    } else if segments[0] & 0xfe00 == 0xfc00 {
        IpClass::Private
    } else if segments[0] & 0xe000 == 0x2000
        && !(segments[0] == 0x2001 && segments[1] < 0x0200)
        && !(segments[0] == 0x2001 && segments[1] == 0x0db8)
        && segments[0] != 0x2002
        && !(segments[0] == 0x3fff && segments[1] & 0xf000 == 0)
    {
        // Only global unicast, minus the special-use blocks inside 2000::/3.
        IpClass::Public
    } else {
        IpClass::Reserved
    }
}

/// Why an outbound destination was refused. One variant per class; no
/// variant carries the input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutboundRefusal {
    Http,
    Loopback,
    LinkLocal,
    Private,
    Reserved,
    Redirect,
    Unresolved,
    UrlInvalid,
    /// The inner port cannot bind a host name to the vetted addresses.
    Unbound,
}

impl OutboundRefusal {
    /// The stable refusal code.
    #[must_use]
    pub fn code(self) -> &'static str {
        match self {
            Self::Http => "outbound_http_refused",
            Self::Loopback => "outbound_loopback_refused",
            Self::LinkLocal => "outbound_link_local_refused",
            Self::Private => "outbound_private_refused",
            Self::Reserved => "outbound_reserved_refused",
            Self::Redirect => "outbound_redirect_refused",
            Self::Unresolved => "outbound_unresolved",
            Self::Unbound => "outbound_unbound_refused",
            Self::UrlInvalid => "outbound_url_invalid",
        }
    }
}

impl fmt::Display for OutboundRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl std::error::Error for OutboundRefusal {}

/// The resolver failed or timed out. Carries nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolveFailed;

/// Name resolution, injected so the policy does no DNS of its own.
pub trait ResolveHost: Send + Sync {
    /// Every address the name resolves to.
    ///
    /// # Errors
    /// [`ResolveFailed`] when the lookup fails or exceeds its deadline.
    fn resolve(&self, host: &str, port: u16) -> Result<Vec<IpAddr>, ResolveFailed>;
}

/// Which destinations are admitted. `strict()` admits public https only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutboundPolicy {
    allow_private: bool,
    allow_loopback_http: bool,
}

impl OutboundPolicy {
    /// Public https only.
    #[must_use]
    pub fn strict() -> Self {
        Self {
            allow_private: false,
            allow_loopback_http: false,
        }
    }

    /// Also admit loopback and private (RFC 1918, fc00::/7) destinations.
    /// Link-local and reserved blocks are never admitted.
    #[must_use]
    pub fn allow_private(mut self) -> Self {
        self.allow_private = true;
        self
    }

    /// Also admit a literal loopback address (not a name that resolves to
    /// one), and plain `http` to it. `allow_private` admits loopback by any
    /// route.
    #[must_use]
    pub fn allow_loopback_http(mut self) -> Self {
        self.allow_loopback_http = true;
        self
    }

    fn admit(&self, ip: IpAddr, literal_host: bool) -> Result<(), OutboundRefusal> {
        match classify_ip(ip) {
            IpClass::Public => Ok(()),
            IpClass::Loopback
                if self.allow_private || (self.allow_loopback_http && literal_host) =>
            {
                Ok(())
            }
            IpClass::Loopback => Err(OutboundRefusal::Loopback),
            IpClass::LinkLocal => Err(OutboundRefusal::LinkLocal),
            IpClass::Private if self.allow_private => Ok(()),
            IpClass::Private => Err(OutboundRefusal::Private),
            IpClass::Reserved => Err(OutboundRefusal::Reserved),
        }
    }

    fn vet_host<S: AsRef<str>>(
        &self,
        host: &Host<S>,
        port: u16,
        resolver: &dyn ResolveHost,
    ) -> Result<Vec<IpAddr>, OutboundRefusal> {
        let addresses = match host {
            Host::Ipv4(ip) => vec![IpAddr::V4(*ip)],
            Host::Ipv6(ip) => vec![IpAddr::V6(*ip)],
            Host::Domain(name) => resolver
                .resolve(name.as_ref(), port)
                .map_err(|_| OutboundRefusal::Unresolved)?,
        };
        if addresses.is_empty() {
            return Err(OutboundRefusal::Unresolved);
        }
        // Every answer must pass; a mixed answer fails closed.
        let literal_host = !matches!(host, Host::Domain(_));
        for ip in &addresses {
            self.admit(*ip, literal_host)?;
        }
        Ok(addresses)
    }

    /// Vet a URL a server supplied.
    ///
    /// # Errors
    /// The refusal class of the first check that fails.
    pub fn vet_url(
        &self,
        raw: &str,
        resolver: &dyn ResolveHost,
    ) -> Result<VettedUrl, OutboundRefusal> {
        let url = Url::parse(raw).map_err(|_| OutboundRefusal::UrlInvalid)?;
        let https = match url.scheme() {
            "https" => true,
            "http" => false,
            _ => return Err(OutboundRefusal::UrlInvalid),
        };
        let host = url.host().ok_or(OutboundRefusal::UrlInvalid)?;
        let port = url
            .port_or_known_default()
            .ok_or(OutboundRefusal::UrlInvalid)?;
        if !https {
            let loopback_literal = match &host {
                Host::Ipv4(ip) => classify_ip(IpAddr::V4(*ip)) == IpClass::Loopback,
                Host::Ipv6(ip) => classify_ip(IpAddr::V6(*ip)) == IpClass::Loopback,
                Host::Domain(_) => false,
            };
            if !(self.allow_loopback_http && loopback_literal) {
                return Err(OutboundRefusal::Http);
            }
        }
        let addresses = self.vet_host(&host, port, resolver)?;
        Ok(VettedUrl { url, addresses })
    }

    /// Vet a host and port for a non-HTTP TLS sink. The policy applies to the
    /// host class only (D-23-4-02 A). `host` is a name, an IPv4 literal, or an
    /// IPv6 literal with or without brackets.
    ///
    /// # Errors
    /// The refusal class of the first check that fails.
    pub fn vet_host_port(
        &self,
        host: &str,
        port: u16,
        resolver: &dyn ResolveHost,
    ) -> Result<Vec<IpAddr>, OutboundRefusal> {
        let parsed = match Host::parse(host) {
            Ok(parsed) => parsed,
            Err(_) => Host::Ipv6(host.parse().map_err(|_| OutboundRefusal::UrlInvalid)?),
        };
        self.vet_host(&parsed, port, resolver)
    }
}

/// A URL that passed the policy, with the addresses it was vetted against.
/// `Debug` shows scheme, host and port only: never userinfo, path or query.
#[derive(Clone, PartialEq, Eq)]
pub struct VettedUrl {
    url: Url,
    addresses: Vec<IpAddr>,
}

impl fmt::Debug for VettedUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VettedUrl")
            .field("scheme", &self.url.scheme())
            .field("host", &self.url.host_str())
            .field("port", &self.url.port_or_known_default())
            .field("addresses", &self.addresses)
            .finish()
    }
}

impl VettedUrl {
    #[must_use]
    pub fn as_str(&self) -> &str {
        self.url.as_str()
    }

    #[must_use]
    pub fn addresses(&self) -> &[IpAddr] {
        &self.addresses
    }

    /// True when the host is a DNS name (so the connection can be rebound),
    /// false for an IP literal.
    #[must_use]
    pub fn is_name(&self) -> bool {
        matches!(self.url.host(), Some(Host::Domain(_)))
    }

    /// The host exactly as a connector parses it from the URL (lowercase,
    /// punycode for a name; an IPv6 literal keeps its brackets).
    #[must_use]
    pub fn host_str(&self) -> &str {
        self.url.host_str().unwrap_or_default()
    }

    /// The explicit or scheme-default port; `vet_url` refuses a URL without one.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.url.port_or_known_default().unwrap_or_default()
    }

    /// The vetted addresses paired with the URL's port.
    #[must_use]
    pub fn socket_addrs(&self) -> Vec<SocketAddr> {
        let port = self.port();
        self.addresses
            .iter()
            .map(|ip| SocketAddr::new(*ip, port))
            .collect()
    }
}

/// An [`IoSubsystemPort`] that vets every URL before it delegates. A refused
/// URL never reaches the inner port.
///
/// A host NAME is vetted once (every answer must pass) and the request goes
/// through `inner.bind(&vetted)`: the bound port connects only to the vetted
/// addresses (Story 23-0b), so a second DNS answer cannot move the
/// connection. An inner port that cannot bind refuses a name
/// (`outbound_unbound_refused`); a name is never sent down the raw-string
/// path. A literal-IP URL has nothing to rebind and goes down the raw-string
/// path.
///
/// No-follow contract: the redirect check below runs after the transport
/// returned, so a transport that follows a redirect would already have
/// connected to an unvetted address. `new` refuses an inner port that does
/// not declare [`IoSubsystemPort::never_follows_redirects`], and a bound port
/// is held to the same declaration. A 3xx answer is then refused here for
/// `http_post` and `http_post_exchange` (both read the exchange status);
/// `http_get` returns bytes only, so a GET redirect is refused by the inner
/// adapter (the kernel adapter never follows and fails a non-2xx).
pub struct PolicedIo<P: IoSubsystemPort, R: ResolveHost> {
    inner: P,
    resolver: R,
    policy: OutboundPolicy,
}

impl<P: IoSubsystemPort, R: ResolveHost> PolicedIo<P, R> {
    /// # Errors
    /// `OutboundRefusal::Redirect` when `inner` does not declare
    /// [`IoSubsystemPort::never_follows_redirects`].
    pub fn new(inner: P, resolver: R, policy: OutboundPolicy) -> Result<Self, OutboundRefusal> {
        if !inner.never_follows_redirects() {
            return Err(OutboundRefusal::Redirect);
        }
        Ok(Self {
            inner,
            resolver,
            policy,
        })
    }

    /// Vet `url`, then run `call` on the port that may carry it: the inner
    /// port for a literal IP, a port bound to the vetted addresses for a name.
    fn through<T>(
        &self,
        url: &str,
        call: impl FnOnce(&dyn IoSubsystemPort, &str) -> Result<T, IoError>,
    ) -> Result<T, IoError> {
        let vetted = self
            .policy
            .vet_url(url, &self.resolver)
            .map_err(|refusal| IoError::InvalidUrl(refusal.code().to_owned()))?;
        if !vetted.is_name() {
            return call(&self.inner, url);
        }
        let bound = self.inner.bind(&vetted)?;
        if !bound.never_follows_redirects() {
            return Err(IoError::InvalidUrl(
                OutboundRefusal::Redirect.code().to_owned(),
            ));
        }
        call(&*bound, vetted.as_str())
    }
}

impl<P: IoSubsystemPort, R: ResolveHost> IoSubsystemPort for PolicedIo<P, R> {
    fn never_follows_redirects(&self) -> bool {
        true
    }

    fn http_get(&self, url: &str) -> Result<Vec<u8>, IoError> {
        self.through(url, |port, url| port.http_get(url))
    }

    fn http_post(
        &self,
        url: &str,
        body: &[u8],
        headers: &[(&str, &str)],
    ) -> Result<Vec<u8>, IoError> {
        let exchange = self.http_post_exchange(url, body, headers, "")?;
        match exchange.status {
            200..=299 => Ok(exchange.body),
            status => Err(IoError::Transport(format!("status {status}"))),
        }
    }

    fn http_post_exchange(
        &self,
        url: &str,
        body: &[u8],
        headers: &[(&str, &str)],
        response_header: &str,
    ) -> Result<HttpExchange, IoError> {
        let exchange = self.through(url, |port, url| {
            port.http_post_exchange(url, body, headers, response_header)
        })?;
        if (300..400).contains(&exchange.status) {
            return Err(IoError::Transport(
                OutboundRefusal::Redirect.code().to_owned(),
            ));
        }
        Ok(exchange)
    }
}
