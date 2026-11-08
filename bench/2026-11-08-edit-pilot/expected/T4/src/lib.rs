// imports begin
use crate::config::Config;
use crate::policy::Policy;
use crate::transport::Conn;
// imports end

pub mod client;
pub mod report;

pub fn start(config: &Config) -> Option<Conn> {
    client::connect(&config.url).ok()
}

#[cfg(test)]
mod tests {
    use crate::config::Config;

    #[test]
    fn start_without_server_is_none() {
        assert!(super::start(&Config::default()).is_none());
    }
}
