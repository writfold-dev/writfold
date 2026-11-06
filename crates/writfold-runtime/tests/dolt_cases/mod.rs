use super::*;

#[test]
fn native_empty_global_config_is_accepted_but_never_clears_settings() -> Result {
    let fixture = Fixture::new()?;
    private(&fixture.path("home/.dolt"))?;
    let config = fixture.path("home/.dolt/config_global.json");
    for empty in [b"{}".as_slice(), b" \r\n{\t }\n".as_slice()] {
        fs::write(&config, empty)?;
        let database = fixture.open()?;
        let lease = database.acquire()?;
        lease.check_current()?;
        assert_eq!(fs::read(&config)?, empty);
        fs::write(&config, br#"{"user.name":"not a read grant"}"#)?;
        profile(lease.check_current());
        assert_eq!(fs::read(&config)?, br#"{"user.name":"not a read grant"}"#);
    }
    for invalid in [
        b"".as_slice(),
        b"[]",
        b"{\"a\":0}",
        b"{}{}",
        b"{/*comment*/}",
        b"\xef\xbb\xbf{}",
    ] {
        fs::write(&config, invalid)?;
        profile(fixture.open());
        assert_eq!(fs::read(&config)?, invalid);
    }
    fs::write(&config, format!("{{{}}}", " ".repeat(4096)))?;
    assert!(matches!(
        fixture.open(),
        Err(DoltError::Budget {
            stream: "global configuration",
            ..
        })
    ));
    assert_eq!(fs::metadata(&config)?.len(), 4098);
    fixture.preserved()
}
