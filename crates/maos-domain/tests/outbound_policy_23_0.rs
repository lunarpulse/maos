//! Story 23-0 — the shared outbound-endpoint policy (D-23-0-02 A, D-23-1-04 A,
//! D-23-0-03 A). No DNS, no network, no clock: the resolver is a fake.

use maos_domain::outbound::{
    classify_ip, IpClass, OutboundPolicy, OutboundRefusal, PolicedIo, ResolveFailed, ResolveHost,
    VettedUrl,
};
use maos_domain::ports::io_subsystem::{HttpExchange, IoError, IoSubsystemPort};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::{Arc, Mutex};

struct FakeResolver(HashMap<&'static str, Result<Vec<IpAddr>, ResolveFailed>>);

impl FakeResolver {
    fn none() -> Self {
        Self(HashMap::new())
    }
    fn with(name: &'static str, answers: &[&str]) -> Self {
        let answers = answers.iter().map(|a| a.parse().unwrap()).collect();
        Self(HashMap::from([(name, Ok(answers))]))
    }
}

impl ResolveHost for FakeResolver {
    fn resolve(&self, host: &str, _port: u16) -> Result<Vec<IpAddr>, ResolveFailed> {
        self.0.get(host).cloned().unwrap_or(Err(ResolveFailed))
    }
}

fn refusal(policy: OutboundPolicy, url: &str) -> Option<OutboundRefusal> {
    policy.vet_url(url, &FakeResolver::none()).err()
}

fn strict(url: &str) -> Option<OutboundRefusal> {
    refusal(OutboundPolicy::strict(), url)
}

#[test]
fn an_http_url_to_a_public_host_is_refused() {
    assert_eq!(strict("http://1.1.1.1/"), Some(OutboundRefusal::Http));
    assert_eq!(
        strict("http://[2606:4700::1111]/"),
        Some(OutboundRefusal::Http)
    );
    assert_eq!(
        strict("http://[::ffff:1.1.1.1]/"),
        Some(OutboundRefusal::Http)
    );
    // Plain http to loopback is refused too, as http, unless the knob is set.
    assert_eq!(
        strict("http://127.0.0.1:8080/"),
        Some(OutboundRefusal::Http)
    );
    // The loopback knob admits plain http to a literal loopback only.
    let loopback = OutboundPolicy::strict().allow_loopback_http();
    assert_eq!(
        refusal(loopback, "http://1.1.1.1/"),
        Some(OutboundRefusal::Http)
    );
    // A scheme that is neither http nor https is not a URL this policy vets.
    assert_eq!(strict("ftp://1.1.1.1/"), Some(OutboundRefusal::UrlInvalid));
    assert_eq!(strict("not a url"), Some(OutboundRefusal::UrlInvalid));
}

#[test]
fn a_loopback_host_is_refused_unless_allowed() {
    for url in [
        "https://127.0.0.1/",
        "https://[::1]/",
        "https://[::ffff:127.0.0.1]/",
        "https://2130706433/",
        "https://0x7f.1/",
    ] {
        assert_eq!(strict(url), Some(OutboundRefusal::Loopback), "{url}");
        // `allow_private` admits loopback as well as private.
        assert_eq!(
            refusal(OutboundPolicy::strict().allow_private(), url),
            None,
            "{url}"
        );
    }
    let loopback = OutboundPolicy::strict().allow_loopback_http();
    for url in [
        "http://127.0.0.1:9000/",
        "http://[::1]:9000/",
        "https://127.0.0.1/",
    ] {
        assert_eq!(refusal(loopback, url), None, "{url}");
    }
    // The knob does not widen to private or link-local.
    assert_eq!(
        refusal(loopback, "https://10.0.0.1/"),
        Some(OutboundRefusal::Private)
    );
    assert_eq!(
        refusal(loopback, "https://169.254.169.254/"),
        Some(OutboundRefusal::LinkLocal)
    );
}

#[test]
fn a_link_local_host_is_refused_even_with_private_allowed() {
    let permissive = OutboundPolicy::strict()
        .allow_private()
        .allow_loopback_http();
    for url in [
        "https://169.254.169.254/",
        "https://[fe80::1]/",
        "https://[::ffff:169.254.169.254]/",
    ] {
        assert_eq!(strict(url), Some(OutboundRefusal::LinkLocal), "{url}");
        assert_eq!(
            refusal(permissive, url),
            Some(OutboundRefusal::LinkLocal),
            "{url}"
        );
    }
}

#[test]
fn a_private_host_is_refused_unless_private_is_allowed() {
    let allowed = OutboundPolicy::strict().allow_private();
    for url in [
        "https://10.1.2.3/",
        "https://172.16.0.1/",
        "https://172.31.255.254/",
        "https://192.168.1.1/",
        "https://[fd00::1]/",
        "https://[::ffff:10.0.0.1]/",
    ] {
        assert_eq!(strict(url), Some(OutboundRefusal::Private), "{url}");
        assert_eq!(refusal(allowed, url), None, "{url}");
    }
    // 172.32/16 is outside RFC 1918.
    assert_eq!(strict("https://172.32.0.1/"), None);
}

#[test]
fn a_reserved_host_is_always_refused() {
    let permissive = OutboundPolicy::strict()
        .allow_private()
        .allow_loopback_http();
    for url in [
        "https://0.0.0.0/",
        "https://[::]/",
        "https://0.1.2.3/",
        "https://240.0.0.1/",
        "https://255.255.255.255/",
        "https://224.0.0.1/",
        "https://[ff02::1]/",
        "https://100.64.0.1/",
        "https://100.127.255.254/",
        "https://192.0.0.1/",
        "https://192.0.2.1/",
        "https://198.18.0.1/",
        "https://198.19.255.254/",
        "https://198.51.100.1/",
        "https://203.0.113.1/",
        "https://[2001:db8::1]/",
        "https://[2001:1::1]/",
        "https://[2002::1]/",
        "https://[::ffff:0.0.0.0]/",
    ] {
        assert_eq!(strict(url), Some(OutboundRefusal::Reserved), "{url}");
        assert_eq!(
            refusal(permissive, url),
            Some(OutboundRefusal::Reserved),
            "{url}"
        );
    }
}

#[test]
fn a_public_https_url_is_vetted() {
    for url in [
        "https://1.1.1.1/",
        "https://[2606:4700::1111]/",
        "https://[::ffff:1.1.1.1]/",
        "https://8.8.8.8:8443/path?q=1",
    ] {
        let vetted = OutboundPolicy::strict()
            .vet_url(url, &FakeResolver::none())
            .unwrap();
        assert!(!vetted.addresses().is_empty(), "{url}");
    }
}

#[test]
fn a_refusal_names_its_class_and_never_the_url() {
    let secret = "s3cr3t-host-marker";
    let cases = [
        format!("http://1.1.1.1/{secret}"),
        format!("https://127.0.0.1/{secret}"),
        format!("https://169.254.169.254/{secret}"),
        format!("https://10.0.0.1/{secret}"),
        format!("https://0.0.0.0/{secret}"),
        format!("https://{secret}.invalid/x"),
        format!("ftp://{secret}"),
        format!("{secret} not a url"),
    ];
    for url in &cases {
        let refused = strict(url).expect("refused");
        for text in [
            refused.code().to_owned(),
            refused.to_string(),
            format!("{refused:?}"),
        ] {
            assert!(!text.contains(secret), "{text}");
            assert!(
                !text.contains("127.0.0.1") && !text.contains("10.0.0.1"),
                "{text}"
            );
        }
    }
    let codes = [
        (OutboundRefusal::Http, "outbound_http_refused"),
        (OutboundRefusal::Loopback, "outbound_loopback_refused"),
        (OutboundRefusal::LinkLocal, "outbound_link_local_refused"),
        (OutboundRefusal::Private, "outbound_private_refused"),
        (OutboundRefusal::Reserved, "outbound_reserved_refused"),
        (OutboundRefusal::Redirect, "outbound_redirect_refused"),
        (OutboundRefusal::Unresolved, "outbound_unresolved"),
        (OutboundRefusal::UrlInvalid, "outbound_url_invalid"),
    ];
    for (refusal, code) in codes {
        assert_eq!(refusal.code(), code);
        assert_eq!(refusal.to_string(), code);
    }
}

#[test]
fn a_name_whose_answers_include_a_private_address_is_refused() {
    let resolver = FakeResolver::with("mixed.example", &["1.1.1.1", "10.0.0.5"]);
    assert_eq!(
        OutboundPolicy::strict()
            .vet_url("https://mixed.example/", &resolver)
            .err(),
        Some(OutboundRefusal::Private)
    );
    // The bad answer is not first: every answer is checked.
    let resolver = FakeResolver::with("mixed.example", &["10.0.0.5", "1.1.1.1"]);
    assert_eq!(
        OutboundPolicy::strict()
            .vet_url("https://mixed.example/", &resolver)
            .err(),
        Some(OutboundRefusal::Private)
    );
    let resolver = FakeResolver::with("ok.example", &["1.1.1.1", "2606:4700::1111"]);
    let vetted = OutboundPolicy::strict()
        .vet_url("https://ok.example/", &resolver)
        .unwrap();
    assert_eq!(vetted.addresses().len(), 2);
}

#[test]
fn localhost_resolving_to_loopback_is_refused() {
    let resolver = FakeResolver::with("localhost", &["127.0.0.1", "::1"]);
    assert_eq!(
        OutboundPolicy::strict()
            .vet_url("https://localhost/", &resolver)
            .err(),
        Some(OutboundRefusal::Loopback)
    );
    // Plain http to a name is never admitted, even when it resolves to loopback.
    assert_eq!(
        OutboundPolicy::strict()
            .allow_loopback_http()
            .vet_url("http://localhost/", &resolver)
            .err(),
        Some(OutboundRefusal::Http)
    );
}

#[test]
fn an_unresolvable_name_is_refused() {
    assert_eq!(
        OutboundPolicy::strict()
            .vet_url("https://nowhere.example/", &FakeResolver::none())
            .err(),
        Some(OutboundRefusal::Unresolved)
    );
    let empty = FakeResolver::with("empty.example", &[]);
    assert_eq!(
        OutboundPolicy::strict()
            .vet_url("https://empty.example/", &empty)
            .err(),
        Some(OutboundRefusal::Unresolved)
    );
}

#[test]
fn a_host_and_port_for_a_tls_sink_is_vetted_by_host_class() {
    let policy = OutboundPolicy::strict();
    let none = FakeResolver::none();
    assert!(policy.vet_host_port("1.1.1.1", 6514, &none).is_ok());
    assert!(policy.vet_host_port("2606:4700::1111", 6514, &none).is_ok());
    assert!(policy
        .vet_host_port("[2606:4700::1111]", 6514, &none)
        .is_ok());
    assert_eq!(
        policy.vet_host_port("10.0.0.1", 6514, &none),
        Err(OutboundRefusal::Private)
    );
    assert_eq!(
        policy.vet_host_port("127.0.0.1", 6514, &none),
        Err(OutboundRefusal::Loopback)
    );
    assert_eq!(
        policy.vet_host_port("::1", 6514, &none),
        Err(OutboundRefusal::Loopback)
    );
    assert_eq!(
        policy.vet_host_port("169.254.169.254", 6514, &none),
        Err(OutboundRefusal::LinkLocal)
    );
    assert_eq!(
        policy.vet_host_port("", 6514, &none),
        Err(OutboundRefusal::UrlInvalid)
    );
    assert_eq!(
        policy.vet_host_port("nowhere.example", 6514, &none),
        Err(OutboundRefusal::Unresolved)
    );
    let mixed = FakeResolver::with("siem.example", &["1.1.1.1", "192.168.0.9"]);
    assert_eq!(
        policy.vet_host_port("siem.example", 6514, &mixed),
        Err(OutboundRefusal::Private)
    );
    assert!(OutboundPolicy::strict()
        .allow_private()
        .vet_host_port("siem.example", 6514, &mixed)
        .is_ok());
}

/// Worker egress's former private `public_ip` (ADR-061), copied verbatim as
/// the reference (D-23-0-03 A).
fn former_egress_public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, _, _] = ip.octets();
            !ip.is_private()
                && !ip.is_loopback()
                && !ip.is_link_local()
                && !ip.is_multicast()
                && !ip.is_broadcast()
                && !ip.is_unspecified()
                && a != 0
                && a < 240
                && !(a == 100 && (64..=127).contains(&b))
                && !(a == 192 && (b == 0 || b == 2))
                && !(a == 198 && (b == 18 || b == 19 || b == 51))
                && !(a == 203 && b == 0)
        }
        IpAddr::V6(ip) => {
            if let Some(mapped) = ip.to_ipv4_mapped() {
                return former_egress_public_ip(IpAddr::V4(mapped));
            }
            let segments = ip.segments();
            segments[0] & 0xe000 == 0x2000
                && !(segments[0] == 0x2001 && segments[1] < 0x0200)
                && !(segments[0] == 0x2001 && segments[1] == 0x0db8)
                && !(segments[0] == 0x2002)
        }
    }
}

#[test]
fn classify_public_equals_the_former_egress_predicate_on_the_whole_vector_table() {
    // Every IPv4 /16 (first two octets), at three host offsets each.
    for a in 0..=255u8 {
        for b in 0..=255u8 {
            for (c, d) in [(0, 0), (0, 1), (255, 254)] {
                let ip = IpAddr::V4(Ipv4Addr::new(a, b, c, d));
                assert_eq!(
                    classify_ip(ip) == IpClass::Public,
                    former_egress_public_ip(ip),
                    "{ip}"
                );
            }
        }
    }
    for ip in [
        Ipv4Addr::BROADCAST,
        Ipv4Addr::UNSPECIFIED,
        Ipv4Addr::LOCALHOST,
    ] {
        let ip = IpAddr::V4(ip);
        assert_eq!(
            classify_ip(ip) == IpClass::Public,
            former_egress_public_ip(ip)
        );
    }
    // IPv6: every first segment, with the second segment swept around the 2001
    // specials and the 3fff::/20 edge. The ONE intended difference from the
    // former predicate: 3fff::/20 (RFC 9637 documentation) was admitted and is
    // now refused (23-0b ruling, 2026-11-25).
    let refused_rfc_9637 =
        |ip: std::net::Ipv6Addr| ip.segments()[0] == 0x3fff && ip.segments()[1] & 0xf000 == 0;
    for s0 in 0..=0xffffu16 {
        for s1 in [0, 1, 0x01ff, 0x0200, 0x0db8, 0x0db9, 0x0fff, 0x1000, 0xffff] {
            let v6 = std::net::Ipv6Addr::new(s0, s1, 0, 0, 0, 0, 0, 1);
            let ip = IpAddr::V6(v6);
            assert_eq!(
                classify_ip(ip) == IpClass::Public,
                former_egress_public_ip(ip) && !refused_rfc_9637(v6),
                "{ip}"
            );
        }
    }
    // v4-mapped: the same answer as the unwrapped v4 address.
    for a in [0u8, 1, 10, 100, 127, 169, 172, 192, 198, 203, 224, 240, 255] {
        let v4 = Ipv4Addr::new(a, 64, 1, 1);
        let mapped = IpAddr::V6(v4.to_ipv6_mapped());
        assert_eq!(classify_ip(mapped), classify_ip(IpAddr::V4(v4)), "{mapped}");
        assert_eq!(
            classify_ip(mapped) == IpClass::Public,
            former_egress_public_ip(mapped)
        );
    }
}

#[test]
fn classify_names_one_class_per_block() {
    let class = |text: &str| classify_ip(text.parse().unwrap());
    assert_eq!(class("1.1.1.1"), IpClass::Public);
    assert_eq!(class("2606:4700::1111"), IpClass::Public);
    assert_eq!(class("127.0.0.1"), IpClass::Loopback);
    assert_eq!(class("127.255.255.254"), IpClass::Loopback);
    assert_eq!(class("::1"), IpClass::Loopback);
    assert_eq!(class("::ffff:127.0.0.1"), IpClass::Loopback);
    assert_eq!(class("169.254.169.254"), IpClass::LinkLocal);
    assert_eq!(class("fe80::1"), IpClass::LinkLocal);
    assert_eq!(class("febf::1"), IpClass::LinkLocal);
    assert_eq!(class("10.0.0.1"), IpClass::Private);
    assert_eq!(class("172.16.0.1"), IpClass::Private);
    assert_eq!(class("192.168.0.1"), IpClass::Private);
    assert_eq!(class("fc00::1"), IpClass::Private);
    assert_eq!(class("fdff::1"), IpClass::Private);
    assert_eq!(class("0.0.0.0"), IpClass::Reserved);
    assert_eq!(class("::"), IpClass::Reserved);
    assert_eq!(class("100.64.0.1"), IpClass::Reserved);
    assert_eq!(class("fec0::1"), IpClass::Reserved);
    assert_eq!(class("ff02::1"), IpClass::Reserved);
}

// ---- PolicedIo (AC5) ----

struct InnerPort {
    calls: Arc<Mutex<Vec<String>>>,
    binds: Mutex<Vec<String>>,
    exchange_status: u16,
    get_error: Option<IoError>,
}

impl InnerPort {
    fn new(exchange_status: u16) -> Self {
        Self {
            calls: Arc::new(Mutex::new(Vec::new())),
            binds: Mutex::new(Vec::new()),
            exchange_status,
            get_error: None,
        }
    }
    /// The bound port's `http_get` fails with `error` (the raw-string path still succeeds).
    fn with_get_error(mut self, error: IoError) -> Self {
        self.get_error = Some(error);
        self
    }
    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
    fn binds(&self) -> Vec<String> {
        self.binds.lock().unwrap().clone()
    }
}

/// What `InnerPort::bind` returns: records `bound ...` calls in the same list.
struct BoundPort {
    calls: Arc<Mutex<Vec<String>>>,
    exchange_status: u16,
    get_error: Option<IoError>,
}

impl IoSubsystemPort for BoundPort {
    fn never_follows_redirects(&self) -> bool {
        true
    }
    fn http_get(&self, url: &str) -> Result<Vec<u8>, IoError> {
        self.calls.lock().unwrap().push(format!("bound-get {url}"));
        match &self.get_error {
            Some(error) => Err(error.clone()),
            None => Ok(b"bound-get-body".to_vec()),
        }
    }
    fn http_post(&self, url: &str, _: &[u8], _: &[(&str, &str)]) -> Result<Vec<u8>, IoError> {
        self.calls.lock().unwrap().push(format!("bound-post {url}"));
        Ok(b"bound-post-body".to_vec())
    }
    fn http_post_exchange(
        &self,
        url: &str,
        _: &[u8],
        _: &[(&str, &str)],
        _: &str,
    ) -> Result<HttpExchange, IoError> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("bound-exchange {url}"));
        Ok(HttpExchange {
            status: self.exchange_status,
            body: b"bound-exchange-body".to_vec(),
            header: Some("https://elsewhere.example/".to_owned()),
        })
    }
}

impl IoSubsystemPort for &InnerPort {
    fn never_follows_redirects(&self) -> bool {
        true
    }
    fn bind(&self, vetted: &VettedUrl) -> Result<Box<dyn IoSubsystemPort>, IoError> {
        self.binds.lock().unwrap().push(vetted.as_str().to_owned());
        Ok(Box::new(BoundPort {
            calls: Arc::clone(&self.calls),
            exchange_status: self.exchange_status,
            get_error: self.get_error.clone(),
        }))
    }
    fn http_get(&self, url: &str) -> Result<Vec<u8>, IoError> {
        self.calls.lock().unwrap().push(format!("get {url}"));
        Ok(b"get-body".to_vec())
    }
    fn http_post(
        &self,
        url: &str,
        _body: &[u8],
        _headers: &[(&str, &str)],
    ) -> Result<Vec<u8>, IoError> {
        self.calls.lock().unwrap().push(format!("post {url}"));
        Ok(b"post-body".to_vec())
    }
    fn http_post_exchange(
        &self,
        url: &str,
        _body: &[u8],
        _headers: &[(&str, &str)],
        _response_header: &str,
    ) -> Result<HttpExchange, IoError> {
        self.calls.lock().unwrap().push(format!("exchange {url}"));
        Ok(HttpExchange {
            status: self.exchange_status,
            body: b"exchange-body".to_vec(),
            header: Some("https://elsewhere.example/".to_owned()),
        })
    }
}

#[test]
fn a_redirect_answer_is_refused_and_not_followed() {
    let inner = InnerPort::new(302);
    let io = PolicedIo::new(&inner, FakeResolver::none(), OutboundPolicy::strict()).unwrap();
    let result = io.http_post_exchange("https://1.1.1.1/token", b"x", &[], "location");
    assert_eq!(
        result,
        Err(IoError::Transport("outbound_redirect_refused".to_owned()))
    );
    let text = format!("{result:?}");
    assert!(
        !text.contains("elsewhere") && !text.contains("exchange-body"),
        "{text}"
    );
    assert_eq!(
        inner.calls(),
        vec!["exchange https://1.1.1.1/token".to_owned()]
    );
    for status in [300, 301, 303, 307, 308, 399] {
        let inner = InnerPort::new(status);
        let io = PolicedIo::new(&inner, FakeResolver::none(), OutboundPolicy::strict()).unwrap();
        assert!(io
            .http_post_exchange("https://1.1.1.1/", b"", &[], "location")
            .is_err());
    }
    // Non-3xx exchanges pass through as data, 4xx and 5xx included.
    for status in [200, 204, 400, 401, 500] {
        let inner = InnerPort::new(status);
        let io = PolicedIo::new(&inner, FakeResolver::none(), OutboundPolicy::strict()).unwrap();
        let exchange = io
            .http_post_exchange("https://1.1.1.1/", b"", &[], "location")
            .unwrap();
        assert_eq!(exchange.status, status);
    }
}

#[test]
fn a_refused_url_never_reaches_the_inner_port() {
    let inner = InnerPort::new(200);
    let io = PolicedIo::new(&inner, FakeResolver::none(), OutboundPolicy::strict()).unwrap();
    for (url, code) in [
        ("http://1.1.1.1/", "outbound_http_refused"),
        ("https://127.0.0.1/", "outbound_loopback_refused"),
        (
            "https://169.254.169.254/latest/meta-data",
            "outbound_link_local_refused",
        ),
        ("https://10.0.0.1/", "outbound_private_refused"),
        ("https://0.0.0.0/", "outbound_reserved_refused"),
        ("https://nowhere.example/", "outbound_unresolved"),
        ("ftp://x/", "outbound_url_invalid"),
    ] {
        let expected = Err(IoError::InvalidUrl(code.to_owned()));
        assert_eq!(io.http_get(url), expected, "{url}");
        assert_eq!(io.http_post(url, b"", &[]), expected, "{url}");
        assert_eq!(
            io.http_post_exchange(url, b"", &[], "location").map(|_| ()),
            expected.map(|_: Vec<u8>| ()),
            "{url}"
        );
    }
    assert!(inner.calls().is_empty(), "{:?}", inner.calls());
    // A vetted URL does reach it.
    assert_eq!(io.http_get("https://1.1.1.1/").unwrap(), b"get-body");
    assert_eq!(
        io.http_post("https://1.1.1.1/", b"", &[]).unwrap(),
        b"exchange-body"
    );
    assert_eq!(inner.calls().len(), 2);
}

#[test]
fn a_redirect_is_refused_on_post_and_a_failing_get_is_not_passed_as_bytes() {
    // http_post reads the exchange status, so a 3xx is refused here, not only
    // by the inner adapter.
    for status in [301, 302, 307, 308] {
        let inner = InnerPort::new(status);
        let io = PolicedIo::new(&inner, FakeResolver::none(), OutboundPolicy::strict()).unwrap();
        assert_eq!(
            io.http_post("https://1.1.1.1/", b"", &[]),
            Err(IoError::Transport("outbound_redirect_refused".to_owned())),
            "{status}"
        );
    }
    // Other non-2xx stays an error, never a body.
    let inner = InnerPort::new(500);
    let io = PolicedIo::new(&inner, FakeResolver::none(), OutboundPolicy::strict()).unwrap();
    assert!(io.http_post("https://1.1.1.1/", b"", &[]).is_err());
    // http_get gets bytes only: the inner adapter's refusal of a 3xx (the
    // kernel adapter's contract) comes through unchanged and no body does.
    struct RedirectingGet;
    impl IoSubsystemPort for RedirectingGet {
        fn never_follows_redirects(&self) -> bool {
            true
        }
        fn http_get(&self, _url: &str) -> Result<Vec<u8>, IoError> {
            Err(IoError::Transport("status 302".to_owned()))
        }
        fn http_post(&self, _: &str, _: &[u8], _: &[(&str, &str)]) -> Result<Vec<u8>, IoError> {
            Err(IoError::Transport("status 302".to_owned()))
        }
    }
    let io = PolicedIo::new(
        RedirectingGet,
        FakeResolver::none(),
        OutboundPolicy::strict(),
    )
    .unwrap();
    assert_eq!(
        io.http_get("https://1.1.1.1/"),
        Err(IoError::Transport("status 302".to_owned()))
    );
}

#[test]
fn loopback_by_name_is_refused_under_the_loopback_knob_alone() {
    let resolver = FakeResolver::with("sneaky.example", &["127.0.0.1"]);
    let knob = OutboundPolicy::strict().allow_loopback_http();
    for url in ["https://sneaky.example/", "http://sneaky.example/"] {
        assert!(knob.vet_url(url, &resolver).is_err(), "{url}");
    }
    assert_eq!(
        knob.vet_url("https://sneaky.example/", &resolver).err(),
        Some(OutboundRefusal::Loopback)
    );
    assert_eq!(
        knob.vet_host_port("sneaky.example", 6514, &resolver),
        Err(OutboundRefusal::Loopback)
    );
    // A literal loopback still passes, over http and https.
    for url in [
        "http://127.0.0.1:9000/",
        "https://127.0.0.1/",
        "http://[::1]:9000/",
    ] {
        assert!(knob.vet_url(url, &FakeResolver::none()).is_ok(), "{url}");
    }
    // `allow_private` keeps its wider meaning: loopback by name too.
    assert!(OutboundPolicy::strict()
        .allow_private()
        .vet_url("https://sneaky.example/", &resolver)
        .is_ok());
}

#[test]
fn the_vetted_url_debug_shows_no_userinfo_path_or_query() {
    let vetted = OutboundPolicy::strict()
        .vet_url(
            "https://user:p4ssw0rd@1.1.1.1:8443/secret-path?token=t0k3n#frag",
            &FakeResolver::none(),
        )
        .unwrap();
    let text = format!("{vetted:?}");
    for leak in ["user", "p4ssw0rd", "secret-path", "t0k3n", "frag"] {
        assert!(!text.contains(leak), "{text}");
    }
    assert!(text.contains("1.1.1.1") && text.contains("8443"), "{text}");
}

// ---- Story 23-0b: binding, no-follow contract, 3fff::/20 ----

#[test]
fn the_classifier_refuses_the_rfc_9637_documentation_block() {
    let class = |text: &str| classify_ip(text.parse().unwrap());
    // 3fff::/20 is refused at both ends of the block.
    assert_eq!(class("3fff::"), IpClass::Reserved);
    assert_eq!(
        class("3fff:fff:ffff:ffff:ffff:ffff:ffff:ffff"),
        IpClass::Reserved
    );
    assert_eq!(class("3fff:800::1"), IpClass::Reserved);
    // The neighbours are unchanged: below it (inside 2000::/3), above it
    // (outside 2000::/3), and just past its end.
    assert_eq!(class("3ffe:ffff::"), IpClass::Public);
    assert_eq!(class("4000::"), IpClass::Reserved);
    assert_eq!(class("3fff:1000::"), IpClass::Public);
    assert_eq!(class("2fff:ffff::"), IpClass::Public);
}

#[test]
fn a_name_answering_in_the_rfc_9637_block_is_refused() {
    let resolver = FakeResolver::with("doc.example", &["3fff:abc::1"]);
    assert_eq!(
        OutboundPolicy::strict().vet_url("https://doc.example/", &resolver),
        Err(OutboundRefusal::Reserved)
    );
    assert_eq!(
        OutboundPolicy::strict()
            .allow_private()
            .vet_url("https://[3fff::1]/", &FakeResolver::none())
            .err(),
        Some(OutboundRefusal::Reserved)
    );
}

/// Counts every call; declares the no-follow contract and does not bind.
struct NoBindPort {
    raw_calls: Arc<Mutex<Vec<String>>>,
}

impl IoSubsystemPort for NoBindPort {
    fn never_follows_redirects(&self) -> bool {
        true
    }
    fn http_get(&self, url: &str) -> Result<Vec<u8>, IoError> {
        self.raw_calls.lock().unwrap().push(format!("get {url}"));
        Ok(Vec::new())
    }
    fn http_post(&self, url: &str, _: &[u8], _: &[(&str, &str)]) -> Result<Vec<u8>, IoError> {
        self.raw_calls.lock().unwrap().push(format!("post {url}"));
        Ok(Vec::new())
    }
    fn http_post_exchange(
        &self,
        url: &str,
        _: &[u8],
        _: &[(&str, &str)],
        _: &str,
    ) -> Result<HttpExchange, IoError> {
        self.raw_calls
            .lock()
            .unwrap()
            .push(format!("exchange {url}"));
        Ok(HttpExchange {
            status: 200,
            body: Vec::new(),
            header: None,
        })
    }
}

#[test]
fn a_port_that_cannot_bind_refuses_host_names_and_is_never_called() {
    let raw_calls = Arc::new(Mutex::new(Vec::new()));
    let io = PolicedIo::new(
        NoBindPort {
            raw_calls: Arc::clone(&raw_calls),
        },
        FakeResolver::with("api.example", &["93.184.216.34"]),
        OutboundPolicy::strict(),
    )
    .unwrap();
    let expected = IoError::InvalidUrl("outbound_unbound_refused".to_owned());
    assert_eq!(io.http_get("https://api.example/x"), Err(expected.clone()));
    assert_eq!(
        io.http_post("https://api.example/x", b"", &[]),
        Err(expected.clone())
    );
    assert_eq!(
        io.http_post_exchange("https://api.example/x", b"", &[], "")
            .map(|_| ()),
        Err(expected)
    );
    assert!(raw_calls.lock().unwrap().is_empty());
    // A literal IP never needed to bind.
    assert!(io.http_get("https://1.1.1.1/").is_ok());
    assert_eq!(raw_calls.lock().unwrap().len(), 1);
    assert_eq!(OutboundRefusal::Unbound.code(), "outbound_unbound_refused");
}

#[test]
fn a_literal_ip_never_binds_and_a_name_binds_once_and_never_goes_raw() {
    let inner = InnerPort::new(200);
    let policy = OutboundPolicy::strict().allow_loopback_http();
    let io = PolicedIo::new(
        &inner,
        FakeResolver::with("api.example", &["93.184.216.34"]),
        policy,
    )
    .unwrap();
    for url in [
        "https://1.1.1.1/",
        "https://[2606:4700::1111]/",
        "http://127.0.0.1:9000/",
    ] {
        io.http_get(url).unwrap();
    }
    assert!(inner.binds().is_empty(), "{:?}", inner.binds());
    assert_eq!(inner.calls().len(), 3);
    // One name request: bind once, the request goes to the bound port only.
    io.http_get("https://api.example/x").unwrap();
    assert_eq!(inner.binds(), vec!["https://api.example/x".to_owned()]);
    assert_eq!(
        inner.calls().last().map(String::as_str),
        Some("bound-get https://api.example/x")
    );
    assert_eq!(inner.calls().len(), 4, "{:?}", inner.calls());
}

#[test]
fn a_redirect_on_the_bound_path_is_refused_with_no_location_and_no_body() {
    for status in [301, 302, 307, 308] {
        let inner = InnerPort::new(status);
        let io = PolicedIo::new(
            &inner,
            FakeResolver::with("api.example", &["93.184.216.34"]),
            OutboundPolicy::strict(),
        )
        .unwrap();
        let result = io.http_post_exchange("https://api.example/token", b"x", &[], "location");
        assert_eq!(
            result,
            Err(IoError::Transport("outbound_redirect_refused".to_owned())),
            "{status}"
        );
        let text = format!("{result:?}");
        assert!(
            !text.contains("elsewhere") && !text.contains("bound-exchange-body"),
            "{text}"
        );
        assert_eq!(
            inner.calls(),
            vec!["bound-exchange https://api.example/token".to_owned()],
            "the bound port sees exactly one call"
        );
        assert_eq!(
            io.http_post("https://api.example/token", b"", &[]),
            Err(IoError::Transport("outbound_redirect_refused".to_owned()))
        );
    }
}

/// A transport that, like ureq with `.redirects(5)`, follows a 3xx before it
/// returns: it records the private address it was sent to.
struct FollowingPort {
    connected: Arc<Mutex<Vec<String>>>,
    bound_follows: bool,
}

impl IoSubsystemPort for FollowingPort {
    fn http_get(&self, _url: &str) -> Result<Vec<u8>, IoError> {
        self.connected
            .lock()
            .unwrap()
            .push("followed https://10.0.0.5/internal".to_owned());
        Ok(b"secret".to_vec())
    }
    fn http_post(&self, url: &str, _: &[u8], _: &[(&str, &str)]) -> Result<Vec<u8>, IoError> {
        self.http_get(url)
    }
    fn bind(&self, _vetted: &VettedUrl) -> Result<Box<dyn IoSubsystemPort>, IoError> {
        Ok(Box::new(FollowingPort {
            connected: Arc::clone(&self.connected),
            bound_follows: self.bound_follows,
        }))
    }
    fn never_follows_redirects(&self) -> bool {
        self.bound_follows
    }
}

#[test]
fn a_transport_that_follows_redirects_is_refused_before_any_request() {
    let connected = Arc::new(Mutex::new(Vec::new()));
    // Default declaration (false): the wrapper refuses to be built over it.
    let result = PolicedIo::new(
        FollowingPort {
            connected: Arc::clone(&connected),
            bound_follows: false,
        },
        FakeResolver::none(),
        OutboundPolicy::strict(),
    );
    assert_eq!(result.err(), Some(OutboundRefusal::Redirect));
    assert!(connected.lock().unwrap().is_empty());
}

#[test]
fn a_bound_port_that_does_not_declare_no_follow_is_refused_before_any_request() {
    struct DeclaresOnlyOuter(Arc<Mutex<Vec<String>>>);
    impl IoSubsystemPort for DeclaresOnlyOuter {
        fn never_follows_redirects(&self) -> bool {
            true
        }
        fn http_get(&self, _: &str) -> Result<Vec<u8>, IoError> {
            Ok(Vec::new())
        }
        fn http_post(&self, _: &str, _: &[u8], _: &[(&str, &str)]) -> Result<Vec<u8>, IoError> {
            Ok(Vec::new())
        }
        fn bind(&self, _vetted: &VettedUrl) -> Result<Box<dyn IoSubsystemPort>, IoError> {
            Ok(Box::new(FollowingPort {
                connected: Arc::clone(&self.0),
                bound_follows: false,
            }))
        }
    }
    let connected = Arc::new(Mutex::new(Vec::new()));
    let io = PolicedIo::new(
        DeclaresOnlyOuter(Arc::clone(&connected)),
        FakeResolver::with("api.example", &["93.184.216.34"]),
        OutboundPolicy::strict(),
    )
    .unwrap();
    assert_eq!(
        io.http_get("https://api.example/x"),
        Err(IoError::InvalidUrl("outbound_redirect_refused".to_owned()))
    );
    assert!(connected.lock().unwrap().is_empty());
}

#[test]
fn a_failing_bound_get_reaches_the_caller_as_the_same_error_never_as_bytes() {
    for error in [
        IoError::Transport("status 302".to_owned()),
        IoError::Transport("status 500".to_owned()),
        IoError::Decode("body".to_owned()),
    ] {
        let inner = InnerPort::new(200).with_get_error(error.clone());
        let io = PolicedIo::new(
            &inner,
            FakeResolver::with("api.example", &["93.184.216.34"]),
            OutboundPolicy::strict(),
        )
        .unwrap();
        assert_eq!(io.http_get("https://api.example/x"), Err(error));
        assert_eq!(inner.binds(), vec!["https://api.example/x".to_owned()]);
        assert_eq!(
            inner.calls(),
            vec!["bound-get https://api.example/x".to_owned()],
            "exactly one bound call and no raw-string call"
        );
    }
}
