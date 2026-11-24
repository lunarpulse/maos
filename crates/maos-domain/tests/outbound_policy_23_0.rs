//! Story 23-0 — the shared outbound-endpoint policy (D-23-0-02 A, D-23-1-04 A,
//! D-23-0-03 A). No DNS, no network, no clock: the resolver is a fake.

use maos_domain::outbound::{
    classify_ip, IpClass, OutboundPolicy, OutboundRefusal, PolicedIo, ResolveFailed, ResolveHost,
};
use maos_domain::ports::io_subsystem::{HttpExchange, IoError, IoSubsystemPort};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Mutex;

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
    // IPv6: every first segment, with the second segment swept around the 2001 specials.
    for s0 in 0..=0xffffu16 {
        for s1 in [0, 1, 0x01ff, 0x0200, 0x0db8, 0x0db9, 0xffff] {
            let ip = IpAddr::V6(std::net::Ipv6Addr::new(s0, s1, 0, 0, 0, 0, 0, 1));
            assert_eq!(
                classify_ip(ip) == IpClass::Public,
                former_egress_public_ip(ip),
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
    calls: Mutex<Vec<String>>,
    exchange_status: u16,
}

impl InnerPort {
    fn new(exchange_status: u16) -> Self {
        Self {
            calls: Mutex::new(Vec::new()),
            exchange_status,
        }
    }
    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

impl IoSubsystemPort for &InnerPort {
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
    let io = PolicedIo::new(&inner, FakeResolver::none(), OutboundPolicy::strict());
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
        let io = PolicedIo::new(&inner, FakeResolver::none(), OutboundPolicy::strict());
        assert!(io
            .http_post_exchange("https://1.1.1.1/", b"", &[], "location")
            .is_err());
    }
    // Non-3xx exchanges pass through as data, 4xx and 5xx included.
    for status in [200, 204, 400, 401, 500] {
        let inner = InnerPort::new(status);
        let io = PolicedIo::new(&inner, FakeResolver::none(), OutboundPolicy::strict());
        let exchange = io
            .http_post_exchange("https://1.1.1.1/", b"", &[], "location")
            .unwrap();
        assert_eq!(exchange.status, status);
    }
}

#[test]
fn a_refused_url_never_reaches_the_inner_port() {
    let inner = InnerPort::new(200);
    let io = PolicedIo::new(&inner, FakeResolver::none(), OutboundPolicy::strict());
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
        let io = PolicedIo::new(&inner, FakeResolver::none(), OutboundPolicy::strict());
        assert_eq!(
            io.http_post("https://1.1.1.1/", b"", &[]),
            Err(IoError::Transport("outbound_redirect_refused".to_owned())),
            "{status}"
        );
    }
    // Other non-2xx stays an error, never a body.
    let inner = InnerPort::new(500);
    let io = PolicedIo::new(&inner, FakeResolver::none(), OutboundPolicy::strict());
    assert!(io.http_post("https://1.1.1.1/", b"", &[]).is_err());
    // http_get gets bytes only: the inner adapter's refusal of a 3xx (the
    // kernel adapter's contract) comes through unchanged and no body does.
    struct RedirectingGet;
    impl IoSubsystemPort for RedirectingGet {
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
    );
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
