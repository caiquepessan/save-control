use std::{cell::RefCell, collections::VecDeque};

use chrono::{Duration, TimeZone, Utc};
use savecontrol_core::{error::Result, manifest::store::*};

struct FakeFetcher {
    calls: RefCell<Vec<Option<String>>>,
    replies: RefCell<VecDeque<FetchResult>>,
}

impl Fetcher for FakeFetcher {
    fn fetch(&self, url: &str, etag: Option<&str>) -> Result<FetchResult> {
        assert_eq!(url, MANIFEST_URL);
        self.calls.borrow_mut().push(etag.map(String::from));
        Ok(self.replies.borrow_mut().pop_front().unwrap())
    }
}

fn fetcher(r: Vec<FetchResult>) -> FakeFetcher {
    FakeFetcher { calls: RefCell::new(vec![]), replies: RefCell::new(r.into()) }
}

fn yaml() -> Vec<u8> {
    include_bytes!("fixtures/manifest.yaml").to_vec()
}

#[test]
fn baixa_guarda_e_respeita_24h() {
    let d = tempfile::tempdir().unwrap();
    let st = ManifestStore::new(d.path().to_path_buf());
    assert!(st.load().unwrap().is_none());
    let f = fetcher(vec![
        FetchResult::Fetched { body: yaml(), etag: Some("abc".into()) },
        FetchResult::NotModified,
    ]);
    let t0 = Utc.with_ymd_and_hms(2026, 11, 24, 10, 0, 0).unwrap();
    assert_eq!(st.update(&f, t0).unwrap(), UpdateOutcome::Updated);
    assert_eq!(st.load().unwrap().unwrap().games.len(), 4);
    assert_eq!(st.update(&f, t0 + Duration::hours(1)).unwrap(), UpdateOutcome::Skipped);
    assert_eq!(f.calls.borrow().len(), 1);
    assert_eq!(st.update(&f, t0 + Duration::hours(25)).unwrap(), UpdateOutcome::NotModified);
    assert_eq!(f.calls.borrow()[1].as_deref(), Some("abc"));
}

#[test]
fn cache_corrompido_rele_o_yaml() {
    let d = tempfile::tempdir().unwrap();
    let st = ManifestStore::new(d.path().to_path_buf());
    let f = fetcher(vec![FetchResult::Fetched { body: yaml(), etag: Some("abc".into()) }]);
    st.update(&f, Utc::now()).unwrap();
    std::fs::write(d.path().join("manifest.cache"), b"lixo").unwrap();
    assert_eq!(st.load().unwrap().unwrap().games.len(), 4);
    assert!(std::fs::metadata(d.path().join("manifest.cache")).unwrap().len() > 100);
}

/// Um servidor que manda o começo da resposta e trava: o download tem de desistir
/// sozinho (senão o primeiro uso e "Atualizar agora" ficam presos para sempre).
#[test]
fn download_desiste_quando_o_servidor_trava() {
    use std::io::{Read, Write};
    use std::time::{Duration as Dur, Instant};
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let porta = l.local_addr().unwrap().port();
    std::thread::spawn(move || {
        let (mut s, _) = l.accept().unwrap();
        let mut buf = [0u8; 2048];
        let _ = s.read(&mut buf);
        let _ = s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 100000\r\n\r\nmetade");
        std::thread::sleep(Dur::from_secs(20));
    });
    let f = HttpFetcher { connect: Dur::from_secs(2), read: Dur::from_millis(300), total: Dur::from_secs(10) };
    let t0 = Instant::now();
    let r = f.fetch(&format!("http://127.0.0.1:{porta}/manifest.yaml"), None);
    assert!(r.is_err(), "o download travado não deu erro");
    assert!(t0.elapsed() < Dur::from_secs(5), "demorou {:?}", t0.elapsed());
}
