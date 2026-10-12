//! Socket-level fixtures. Every server here is a scripted `TcpListener` on a
//! random loopback port; nothing talks to a real service.
use james_loopback::{Client, Error, Limits, Method, Timeouts};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

enum Step {
    Write(&'static [u8]),
    WriteOwned(Vec<u8>),
    Sleep(Duration),
}

/// Accepts one connection, reads the full request, plays the script, closes.
/// Returns the raw request bytes so tests can inspect what James sent.
fn serve(script: Vec<Step>) -> (Client, thread::JoinHandle<Vec<u8>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let join = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let request = read_request(&mut socket);
        for step in script {
            match step {
                Step::Write(bytes) => {
                    if socket.write_all(bytes).is_err() {
                        break;
                    }
                }
                Step::WriteOwned(bytes) => {
                    if socket.write_all(&bytes).is_err() {
                        break;
                    }
                }
                Step::Sleep(duration) => thread::sleep(duration),
            }
        }
        let _ = socket.flush();
        request
    });
    (Client::new(port).unwrap(), join)
}

fn read_request(socket: &mut TcpStream) -> Vec<u8> {
    let mut request = Vec::new();
    let mut byte = [0_u8; 1];
    while !request.ends_with(b"\r\n\r\n") {
        if socket.read(&mut byte).unwrap_or(0) == 0 {
            return request;
        }
        request.push(byte[0]);
    }
    let head = String::from_utf8_lossy(&request).to_string();
    let length: usize = head
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse().ok())?
        })
        .unwrap_or(0);
    let mut body = vec![0_u8; length];
    if length > 0 {
        socket.read_exact(&mut body).unwrap();
    }
    request.extend_from_slice(&body);
    request
}

fn quick() -> Timeouts {
    Timeouts {
        connect: Duration::from_secs(2),
        idle: Duration::from_secs(5),
        total: None,
    }
}

#[test]
fn a_post_carries_exactly_the_configured_request_and_reads_a_sized_body() {
    let (client, server) = serve(vec![Step::Write(
        b"HTTP/1.1 200 OK\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: 13\r\n\r\n{\"ok\":true}\r\n",
    )]);
    let response = client
        .with_timeouts(quick())
        .post("/v1/chat/completions")
        .header("Accept", "text/event-stream, application/json")
        .body("application/json", br#"{"stream":true}"#)
        .send()
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.media_type().as_deref(), Some("application/json"));
    assert_eq!(response.header("CONTENT-length"), Some("13"));
    let body = response.read_to_end(1024).unwrap();
    assert_eq!(body, b"{\"ok\":true}\r\n");

    let request = String::from_utf8(server.join().unwrap()).unwrap();
    let mut lines = request.lines();
    assert_eq!(lines.next(), Some("POST /v1/chat/completions HTTP/1.1"));
    assert!(request.contains(&format!("\r\nHost: 127.0.0.1:{}\r\n", client.port())));
    assert!(request.contains("\r\nConnection: close\r\n"));
    assert!(request.contains("\r\nAccept: text/event-stream, application/json\r\n"));
    assert!(request.contains("\r\nContent-Type: application/json\r\n"));
    assert!(request.contains("\r\nContent-Length: 15\r\n"));
    assert!(request.ends_with("\r\n\r\n{\"stream\":true}"));
    assert!(
        !request.contains("Accept-Encoding"),
        "never invite compression"
    );
}

#[test]
fn chunked_bodies_are_decoded_across_arbitrary_write_boundaries() {
    // Chunk extensions, a trailer, and splits inside size lines, data and CRLFs.
    let (client, server) = serve(vec![
        Step::Write(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nContent-Type: text/event-stream\r\n\r\n"),
        Step::Write(b"6;ext=1\r\ndata: "),
        Step::Sleep(Duration::from_millis(30)),
        Step::Write(b"\r\n"),
        Step::Write(b"B\r\n{\"a\":\"h\xC3\xA9"),
        Step::Sleep(Duration::from_millis(30)),
        Step::Write(b"\"}\r"),
        Step::Write(b"\n5\r\n\n\ndat\r\n7\r\na: end\n\r\n0\r\nX-Trailer: yes\r\n\r\n"),
    ]);
    let mut body = client
        .with_timeouts(quick())
        .get("/stream")
        .send()
        .unwrap()
        .into_body();
    let mut lines = Vec::new();
    while let Some(line) = body.next_line(4096).unwrap() {
        lines.push(String::from_utf8(line).unwrap());
    }
    assert_eq!(lines, vec!["data: {\"a\":\"hé\"}", "", "data: end"]);
    assert!(body.is_complete());
    server.join().unwrap();
}

#[test]
fn a_server_that_vanishes_mid_chunk_yields_the_data_so_far_then_closed() {
    let (client, server) = serve(vec![
        Step::Write(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n"),
        Step::Write(b"5\r\nfirst\r\n"),
        Step::Write(b"9\r\nseco"),
    ]);
    let mut body = client
        .with_timeouts(quick())
        .get("/stream")
        .send()
        .unwrap()
        .into_body();
    let mut buf = [0_u8; 64];
    let mut got = Vec::new();
    let error = loop {
        match body.read_chunk(&mut buf) {
            Ok(0) => panic!("a truncated chunked body must not look complete"),
            Ok(n) => got.extend_from_slice(&buf[..n]),
            Err(error) => break error,
        }
    };
    assert_eq!(got, b"firstseco");
    assert_eq!(error, Error::Closed);
    assert!(!body.is_complete());
    server.join().unwrap();
}

#[test]
fn a_short_content_length_body_is_closed_not_complete() {
    let (client, server) = serve(vec![Step::Write(
        b"HTTP/1.1 200 OK\r\nContent-Length: 10\r\n\r\nfour",
    )]);
    let result = client
        .with_timeouts(quick())
        .get("/x")
        .send()
        .unwrap()
        .read_to_end(1024);
    assert_eq!(result, Err(Error::Closed));
    server.join().unwrap();
}

#[test]
fn until_close_bodies_end_cleanly_and_final_unterminated_lines_are_kept() {
    let (client, server) = serve(vec![Step::Write(b"HTTP/1.0 200 OK\r\n\r\na\r\nb\nc")]);
    let mut body = client
        .with_timeouts(quick())
        .get("/x")
        .send()
        .unwrap()
        .into_body();
    assert_eq!(body.next_line(100).unwrap(), Some(b"a".to_vec()));
    assert_eq!(body.next_line(100).unwrap(), Some(b"b".to_vec()));
    assert_eq!(body.next_line(100).unwrap(), Some(b"c".to_vec()));
    assert_eq!(body.next_line(100).unwrap(), None);
    assert!(body.is_complete());
    server.join().unwrap();
}

#[test]
fn oversized_headers_bodies_and_lines_are_refused() {
    let huge_header = format!(
        "HTTP/1.1 200 OK\r\nX-Big: {}\r\nContent-Length: 1\r\n\r\nx",
        "a".repeat(20 * 1024)
    );
    let (client, server) = serve(vec![Step::WriteOwned(huge_header.into_bytes())]);
    assert!(matches!(
        client.with_timeouts(quick()).get("/x").send(),
        Err(Error::TooLarge)
    ));
    server.join().unwrap();

    // Declared length above the cap fails before a single body byte is read.
    let (client, server) = serve(vec![Step::Write(
        b"HTTP/1.1 200 OK\r\nContent-Length: 5000000\r\n\r\n",
    )]);
    let response = client.with_timeouts(quick()).get("/x").send().unwrap();
    let started = Instant::now();
    assert_eq!(response.read_to_end(1024), Err(Error::TooLarge));
    assert!(started.elapsed() < Duration::from_secs(1));
    server.join().unwrap();

    // A chunked stream that keeps going is cut off at the client-wide cap.
    let mut flood = b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n".to_vec();
    for _ in 0..200 {
        flood.extend_from_slice(b"64\r\n");
        flood.extend_from_slice(&[b'z'; 100]);
        flood.extend_from_slice(b"\r\n");
    }
    flood.extend_from_slice(b"0\r\n\r\n");
    let (client, server) = serve(vec![Step::WriteOwned(flood)]);
    let client = client.with_timeouts(quick()).with_limits(Limits {
        headers: 16 * 1024,
        body: 5000,
    });
    let mut body = client.get("/x").send().unwrap().into_body();
    let mut buf = [0_u8; 1000];
    let outcome = loop {
        match body.read_chunk(&mut buf) {
            Ok(0) => break Ok(()),
            Ok(_) => {}
            Err(error) => break Err(error),
        }
    };
    assert_eq!(outcome, Err(Error::TooLarge));
    server.join().unwrap();

    // A single line longer than its cap is refused without buffering forever.
    let long_line = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: 3002\r\n\r\n{}\r\n",
        "q".repeat(3000)
    );
    let (client, server) = serve(vec![Step::WriteOwned(long_line.into_bytes())]);
    let mut body = client
        .with_timeouts(quick())
        .get("/x")
        .send()
        .unwrap()
        .into_body();
    assert_eq!(body.next_line(1000), Err(Error::TooLarge));
    server.join().unwrap();
}

#[test]
fn read_prefix_bounds_an_error_body_without_failing_on_length() {
    let (client, server) = serve(vec![Step::WriteOwned(
        format!(
            "HTTP/1.1 500 Internal Server Error\r\nContent-Length: 10000\r\n\r\n{}",
            "e".repeat(10000)
        )
        .into_bytes(),
    )]);
    let response = client.with_timeouts(quick()).get("/x").send().unwrap();
    assert_eq!(response.status(), 500);
    let prefix = response.into_body().read_prefix(100).unwrap();
    assert_eq!(prefix.len(), 100);
    server.join().unwrap();
}

#[test]
fn a_stalled_server_fails_after_the_idle_limit_not_forever() {
    let (client, server) = serve(vec![
        Step::Write(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n6\r\nhello\n\r\n"),
        Step::Sleep(Duration::from_secs(3)),
        Step::Write(b"0\r\n\r\n"),
    ]);
    let client = client.with_timeouts(Timeouts {
        connect: Duration::from_secs(2),
        idle: Duration::from_millis(300),
        total: None,
    });
    let mut body = client.get("/x").send().unwrap().into_body();
    assert_eq!(body.next_line(100).unwrap(), Some(b"hello".to_vec()));
    let started = Instant::now();
    assert_eq!(body.next_line(100), Err(Error::TimedOut));
    assert!(
        started.elapsed() < Duration::from_millis(1500),
        "{:?}",
        started.elapsed()
    );
    server.join().unwrap();
}

#[test]
fn a_total_deadline_caps_a_slow_but_live_stream() {
    let mut script = vec![Step::Write(
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n",
    )];
    for _ in 0..40 {
        script.push(Step::Write(b"1\r\nx\r\n"));
        script.push(Step::Sleep(Duration::from_millis(50)));
    }
    script.push(Step::Write(b"0\r\n\r\n"));
    let (client, server) = serve(script);
    let client = client.with_timeouts(Timeouts {
        connect: Duration::from_secs(2),
        idle: Duration::from_secs(5),
        total: Some(Duration::from_millis(400)),
    });
    let started = Instant::now();
    let result = client.get("/x").send().unwrap().read_to_end(1 << 20);
    assert_eq!(result, Err(Error::TimedOut));
    assert!(
        started.elapsed() < Duration::from_millis(1500),
        "{:?}",
        started.elapsed()
    );
    server.join().unwrap();
}

#[test]
fn abort_from_another_thread_unblocks_a_waiting_read_at_once() {
    let (client, server) = serve(vec![
        Step::Write(b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n6\r\nfirst\n\r\n"),
        Step::Sleep(Duration::from_secs(4)),
        Step::Write(b"0\r\n\r\n"),
    ]);
    let mut handle = None;
    let response = client
        .with_timeouts(quick())
        .get("/x")
        .on_connect(|abort| handle = Some(abort))
        .send()
        .unwrap();
    let handle = handle.expect("on_connect runs before the response is read");
    let mut body = response.into_body();
    assert_eq!(body.next_line(100).unwrap(), Some(b"first".to_vec()));
    let aborter = thread::spawn(move || {
        thread::sleep(Duration::from_millis(150));
        handle.abort();
    });
    let started = Instant::now();
    let result = body.next_line(100);
    assert_eq!(result, Err(Error::Closed), "Stop must never look like a clean EOF");
    assert!(
        started.elapsed() < Duration::from_millis(1500),
        "abort must not wait for the idle timeout: {:?}",
        started.elapsed()
    );
    assert!(!body.is_complete());
    aborter.join().unwrap();
    server.join().unwrap();
}

#[test]
fn abort_does_not_complete_an_until_close_response() {
    // A socket shutdown can surface as Ok(0) on Windows. With no declared
    // response length that looks like a legitimate HTTP end-of-body, unless
    // the transport remembers that this socket was explicitly aborted.
    let (client, server) = serve(vec![
        Step::Write(b"HTTP/1.1 200 OK\r\n\r\nfirst\n"),
        Step::Sleep(Duration::from_millis(350)),
        Step::Write(b"second\n"),
    ]);
    let mut handle = None;
    let response = client
        .with_timeouts(quick())
        .get("/x")
        .on_connect(|abort| handle = Some(abort))
        .send()
        .unwrap();
    let mut body = response.into_body();
    assert_eq!(body.next_line(100).unwrap(), Some(b"first".to_vec()));
    let aborter = thread::spawn(move || {
        thread::sleep(Duration::from_millis(70));
        handle.unwrap().abort();
    });
    let started = Instant::now();
    assert_eq!(body.next_line(100), Err(Error::Closed));
    assert!(!body.is_complete());
    assert!(
        started.elapsed() < Duration::from_millis(1000),
        "abort must unblock the read promptly"
    );
    aborter.join().unwrap();
    server.join().unwrap();
}

#[test]
fn nothing_listening_is_reported_as_unreachable_quickly() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let started = Instant::now();
    let result = Client::new(port)
        .unwrap()
        .with_timeouts(quick())
        .get("/x")
        .send();
    assert!(matches!(result, Err(Error::Unreachable(_))), "{result:?}");
    assert!(started.elapsed() < Duration::from_secs(3));
}

#[test]
fn redirects_are_returned_as_status_codes_never_followed() {
    let (client, server) = serve(vec![Step::Write(
        b"HTTP/1.1 302 Found\r\nLocation: http://192.0.2.1/v1/models\r\nContent-Length: 0\r\n\r\n",
    )]);
    let response = client
        .with_timeouts(quick())
        .get("/v1/models")
        .send()
        .unwrap();
    assert_eq!(response.status(), 302);
    assert_eq!(
        response.header("location"),
        Some("http://192.0.2.1/v1/models")
    );
    assert_eq!(response.read_to_end(10).unwrap(), b"");
    server.join().unwrap();
}

#[test]
fn malformed_and_unsupported_responses_are_named_not_guessed() {
    for (response, expect_malformed) in [
        (&b"<html>not http</html>"[..], true),
        (&b"HTTP/2 200 OK\r\n\r\n"[..], true),
        (&b"HTTP/1.1 200 OK\r\nbroken header\r\n\r\n"[..], true),
        (
            &b"HTTP/1.1 200 OK\r\nContent-Encoding: gzip\r\nContent-Length: 1\r\n\r\nx"[..],
            false,
        ),
        (
            &b"HTTP/1.1 200 OK\r\nTransfer-Encoding: gzip, chunked\r\n\r\n0\r\n\r\n"[..],
            false,
        ),
    ] {
        let (client, server) = serve(vec![Step::Write(response)]);
        let result = client.with_timeouts(quick()).get("/x").send();
        match result {
            Err(Error::Malformed(_)) => assert!(expect_malformed, "{result:?}"),
            Err(Error::Unsupported(_)) => assert!(!expect_malformed, "{result:?}"),
            other => panic!("expected a framing error, got {other:?}"),
        }
        server.join().unwrap();
    }

    let (client, server) = serve(vec![Step::Write(
        b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\nzz\r\n",
    )]);
    let result = client
        .with_timeouts(quick())
        .get("/x")
        .send()
        .unwrap()
        .read_to_end(100);
    assert!(matches!(result, Err(Error::Malformed(_))), "{result:?}");
    server.join().unwrap();
}

#[test]
fn interim_responses_are_skipped_and_empty_responses_complete() {
    let (client, server) = serve(vec![Step::Write(
        b"HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 204 No Content\r\nX-Note: done\r\n\r\n",
    )]);
    let response = client
        .with_timeouts(quick())
        .request(Method::Post, "/x")
        .body("text/plain", b"hi")
        .send()
        .unwrap();
    assert_eq!(response.status(), 204);
    assert_eq!(response.header("x-note"), Some("done"));
    let body = response.into_body();
    assert!(body.is_complete());
    server.join().unwrap();
}

#[test]
fn ipv6_loopback_is_tried_when_ipv4_refuses() {
    let Ok(listener) = TcpListener::bind("[::1]:0") else {
        eprintln!("IPv6 loopback unavailable on this host; fallback not exercised");
        return;
    };
    let port = listener.local_addr().unwrap().port();
    let server = thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(Duration::from_secs(10)))
            .unwrap();
        let request = read_request(&mut socket);
        socket
            .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nv6")
            .unwrap();
        request
    });
    let response = Client::new(port)
        .unwrap()
        .with_timeouts(quick())
        .get("/x")
        .send()
        .unwrap();
    assert_eq!(response.read_to_end(10).unwrap(), b"v6");
    let request = String::from_utf8(server.join().unwrap()).unwrap();
    assert!(
        request.contains(&format!("\r\nHost: [::1]:{port}\r\n")),
        "{request}"
    );
}
