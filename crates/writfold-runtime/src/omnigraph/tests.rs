use super::{NativeError, Parameter, Scalar, bootstrap, http, request};

fn headers(extra: &str) -> Vec<u8> {
    format!("HTTP/1.1 200 OK\r\nOmnigraph-Http-Api: 0.14\r\nConnection: close\r\n{extra}\r\n")
        .into_bytes()
}

#[test]
fn response_framing_requires_unique_length_and_exact_contract() {
    assert!(matches!(
        http::parse_headers(&headers("Content-Length: 2\r\n"), &mut || Ok(())),
        Ok((200, http::Framing::Length(2)))
    ));
    assert!(matches!(
        http::parse_headers(&headers("Transfer-Encoding: chunked\r\n"), &mut || Ok(())),
        Ok((200, http::Framing::Chunked))
    ));
    for extra in [
        "Content-Length: 2\r\nContent-Length: 2\r\n",
        "Content-Length: 2\r\ncontent-length: 3\r\n",
        "Content-Length: +2\r\n",
        "Content-Length: 2\r\nTransfer-Encoding: chunked\r\n",
        "Content-Length: 2\r\nContent-Encoding: gzip\r\n",
        "Content-Length: 2\r\nOmnigraph-Http-Api: 0.13\r\n",
        "Content-Length: 2\r\n folded: value\r\n",
        "Content-Length: 2\r\nX-Foo: one\r\nx-foo: two\r\n",
        "Content-Length: 2\nX-Smuggled: yes\r\n",
        "",
    ] {
        assert!(
            http::parse_headers(&headers(extra), &mut || Ok(())).is_err(),
            "accepted {extra:?}"
        );
    }
    assert!(http::parse_headers(b"HTTP/1.1 302 Found\r\nOmnigraph-Http-Api: 0.14\r\nConnection: close\r\nContent-Length: 0\r\n\r\n", &mut || Ok(())).is_err());
}

#[test]
fn header_work_is_bounded_and_observes_cancellation() {
    let mut extra = String::from("Content-Length: 0\r\n");
    for index in 0..126 {
        extra.push_str(&format!("X-{index}: v\r\n"));
    }
    assert!(matches!(
        http::parse_headers(&headers(&extra), &mut || Ok(())),
        Err(NativeError::Budget("response header fields"))
    ));
    let cancelled = std::sync::atomic::AtomicBool::new(true);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(1);
    assert!(matches!(
        http::parse_headers(&headers("Content-Length: 0\r\n"), &mut || http::stop(
            deadline, &cancelled
        )),
        Err(NativeError::Cancelled)
    ));
}

#[test]
fn listener_requires_complete_unique_owned_loopback_line() -> Result<(), NativeError> {
    assert_eq!(
        http::announcement(b"OMNIGRAPH_LISTEN_ADDR=127.0.0.1:1234")?,
        None
    );
    let address = http::announcement(b"some startup log\nOMNIGRAPH_LISTEN_ADDR=127.0.0.1:1234\n")?
        .ok_or(NativeError::Protocol("test announcement absent"))?;
    assert_eq!(address.port(), 1234);
    for bad in [
        "OMNIGRAPH_LISTEN_ADDR=0.0.0.0:1234\n",
        "OMNIGRAPH_LISTEN_ADDR=127.0.0.2:1234\n",
        "OMNIGRAPH_LISTEN_ADDR=[::1]:1234\n",
        "OMNIGRAPH_LISTEN_ADDR=127.0.0.1:0\n",
        "OMNIGRAPH_LISTEN_ADDR=127.0.0.1:1234\nOMNIGRAPH_LISTEN_ADDR=127.0.0.1:1234\n",
    ] {
        assert!(http::announcement(bad.as_bytes()).is_err());
    }
    Ok(())
}

#[test]
fn scalar_parameters_cannot_override_mutation_policy() -> Result<(), NativeError> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let cancelled = std::sync::atomic::AtomicBool::new(false);
    let params = [Parameter {
        name: "payload",
        value: Scalar::String("\"},\"expect_mutation\":true,\"params\":{\""),
    }];
    let body = request::query_body(
        request::Target::Branch("main"),
        &params,
        false,
        4096,
        deadline,
        &cancelled,
    )?;
    let value: serde_json::Value = serde_json::from_slice(&body)?;
    assert_eq!(value["expect_mutation"], false);
    assert_eq!(value["branch"], "main");
    assert_eq!(
        value["params"]["payload"],
        "\"},\"expect_mutation\":true,\"params\":{\""
    );
    assert!(
        request::query_body(
            request::Target::Branch("main"),
            &params,
            false,
            12,
            deadline,
            &cancelled
        )
        .is_err()
    );
    let duplicate = [
        Parameter {
            name: "key",
            value: Scalar::Null,
        },
        Parameter {
            name: "key",
            value: Scalar::Integer(1),
        },
    ];
    assert!(
        request::query_body(
            request::Target::Branch("main"),
            &duplicate,
            true,
            4096,
            deadline,
            &cancelled
        )
        .is_err()
    );
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let params = [Parameter {
            name: "number",
            value: Scalar::Float(value),
        }];
        assert!(
            request::query_body(
                request::Target::Branch("main"),
                &params,
                true,
                4096,
                deadline,
                &cancelled
            )
            .is_err()
        );
    }
    Ok(())
}

#[test]
fn query_names_are_one_encoded_segment() -> Result<(), NativeError> {
    assert_eq!(
        http::segment("lookup(key:String)/?x=#")?,
        "lookup%28key%3AString%29%2F%3Fx%3D%23"
    );
    for invalid in ["", ".", "..", "bad\r\nHeader: injected"] {
        assert!(http::segment(invalid).is_err());
    }
    Ok(())
}

#[test]
fn exact_lock_handoff_rejects_ambiguous_or_foreign_evidence() -> Result<(), NativeError> {
    let diagnostic = b"Cluster-root: file:///private/cluster\nAdmission lock: lock-123; retain until prior work is quiescent, then exact-ID force-unlock\n";
    assert_eq!(
        bootstrap::exact_lock(diagnostic, "file:///private/cluster")?,
        "lock-123"
    );
    assert!(bootstrap::exact_lock(diagnostic, "file:///other/cluster").is_err());
    let duplicated = [diagnostic.as_slice(), diagnostic.as_slice()].concat();
    assert!(bootstrap::exact_lock(&duplicated, "file:///private/cluster").is_err());
    assert!(bootstrap::exact_lock(b"Cluster-root: file:///private/cluster\nAdmission lock: arbitrary; establish prior-work quiescence before exact-ID force-unlock\n", "file:///private/cluster").is_err());
    Ok(())
}

#[test]
fn transport_rejects_truncated_oversized_and_trailing_bodies()
-> Result<(), Box<dyn std::error::Error>> {
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::atomic::AtomicBool;
    use std::time::{Duration, Instant};
    use writfold_domain::ByteCount;
    let count = ByteCount::new(4096)?;
    let limits = super::NativeLimits {
        request_header: count,
        request_body: count,
        response_header: count,
        response_body: ByteCount::new(2)?,
        stdout: count,
        stderr: count,
        shutdown_grace: Duration::from_secs(1),
    };
    for (length, payload) in [
        (2, b"x".as_slice()),
        (3, b"abc".as_slice()),
        (2, b"abc".as_slice()),
    ] {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let mut socket = TcpStream::connect(listener.local_addr()?)?;
        socket.set_nonblocking(true)?;
        let mut response = headers(&format!("Content-Length: {length}\r\n"));
        response.extend_from_slice(payload);
        let peer = std::thread::spawn(move || -> std::io::Result<()> {
            let (mut stream, _) = listener.accept()?;
            stream.set_read_timeout(Some(Duration::from_secs(2)))?;
            let mut request = [0; 1];
            stream.read_exact(&mut request)?;
            stream.write_all(&response)?;
            Ok(())
        });
        let cancelled = AtomicBool::new(false);
        let deadline = Instant::now() + Duration::from_secs(2);
        let outcome = http::exchange(&mut socket, b"x", b"", limits, || {
            http::stop(deadline, &cancelled)
        });
        assert!(outcome.is_err());
        peer.join()
            .map_err(|_| std::io::Error::other("test peer panicked"))??;
    }
    Ok(())
}

#[path = "tests/chunked.rs"]
mod chunked;
