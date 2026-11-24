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
