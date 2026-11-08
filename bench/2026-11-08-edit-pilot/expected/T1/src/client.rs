use crate::transport::{open, Conn, Error};

pub fn connect(url: &str) -> Result<Conn, Error> {
    let conn = with_retry(3, || open(url))?;
    conn.handshake()?;
    Ok(conn)
}

pub fn probe(url: &str) -> bool {
    open(url).is_ok()
}

pub fn reconnect(conn: &mut Conn, url: &str) -> Result<(), Error> {
    let fresh = open(url)?;
    *conn = fresh;
    Ok(())
}
