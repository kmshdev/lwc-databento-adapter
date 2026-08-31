use std::{env, io, net::IpAddr};

#[derive(Debug, Clone)]
pub struct GatewayConfig {
    pub bind_host: String,
    pub bind_port: u16,
    pub allow_unauthenticated_localhost: bool,
    pub auth_integration_enabled: bool,
    pub allowed_origins: Vec<String>,
    pub allowed_datasets: Vec<String>,
    pub history_max_intervals: usize,
    pub symbol_max_bytes: usize,
    pub symbol_search_max_results: usize,
    pub http_body_max_bytes: usize,
    pub ws_frame_max_bytes: usize,
    pub max_clients: usize,
    pub max_subscriptions_per_client: usize,
    pub max_dataset_sessions: usize,
    pub upstream_queue_capacity: usize,
    pub canonical_queue_capacity: usize,
    pub outbound_queue_capacity: usize,
    pub handoff_buffer_capacity: usize,
    pub reconnect_base_delay_ms: u64,
    pub reconnect_max_delay_ms: u64,
    pub reconnect_max_attempts: usize,
}

impl Default for GatewayConfig {
    fn default() -> Self {
        Self::loopback_localhost()
    }
}

impl GatewayConfig {
    pub fn loopback_localhost() -> Self {
        Self {
            bind_host: "127.0.0.1".to_string(),
            bind_port: 8080,
            allow_unauthenticated_localhost: true,
            auth_integration_enabled: false,
            allowed_origins: vec!["http://127.0.0.1:5173".to_string()],
            allowed_datasets: vec!["GLBX.MDP3".to_string(), "XNAS.ITCH".to_string()],
            history_max_intervals: 10_000,
            symbol_max_bytes: 128,
            symbol_search_max_results: 100,
            http_body_max_bytes: 64 * 1024,
            ws_frame_max_bytes: 64 * 1024,
            max_clients: 16,
            max_subscriptions_per_client: 16,
            max_dataset_sessions: 4,
            upstream_queue_capacity: 4_096,
            canonical_queue_capacity: 4_096,
            outbound_queue_capacity: 256,
            handoff_buffer_capacity: 4_096,
            reconnect_base_delay_ms: 250,
            reconnect_max_delay_ms: 8_000,
            reconnect_max_attempts: 8,
        }
    }

    pub fn allowed_dataset(&self, dataset: &str) -> bool {
        self.allowed_datasets.iter().any(|entry| entry == dataset)
    }

    pub fn from_env() -> io::Result<Self> {
        let mut config = Self::loopback_localhost();
        if let Ok(value) = env::var("DATABENTO_LWC_BIND_ADDR") {
            let (host, port) = value.rsplit_once(':').ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "bind address must be IP:port")
            })?;
            config.bind_host = host.to_string();
            config.bind_port = parse_value("DATABENTO_LWC_BIND_ADDR port", port)?;
        }
        if let Ok(value) = env::var("DATABENTO_LWC_ALLOWED_ORIGINS") {
            config.allowed_origins = parse_list(&value);
        }
        if let Ok(value) = env::var("DATABENTO_LWC_ALLOWED_DATASETS") {
            config.allowed_datasets = parse_list(&value);
        }
        if let Ok(value) = env::var("DATABENTO_LWC_ALLOW_UNAUTHENTICATED_LOCALHOST") {
            config.allow_unauthenticated_localhost =
                parse_value("DATABENTO_LWC_ALLOW_UNAUTHENTICATED_LOCALHOST", &value)?;
        }
        if let Ok(value) = env::var("DATABENTO_LWC_AUTH_INTEGRATION_ENABLED") {
            config.auth_integration_enabled =
                parse_value("DATABENTO_LWC_AUTH_INTEGRATION_ENABLED", &value)?;
        }

        macro_rules! read_bound {
            ($field:ident, $name:literal) => {
                if let Ok(value) = env::var($name) {
                    config.$field = parse_value($name, &value)?;
                }
            };
        }
        read_bound!(history_max_intervals, "DATABENTO_LWC_HISTORY_MAX_INTERVALS");
        read_bound!(symbol_max_bytes, "DATABENTO_LWC_SYMBOL_MAX_BYTES");
        read_bound!(
            symbol_search_max_results,
            "DATABENTO_LWC_SYMBOL_SEARCH_MAX_RESULTS"
        );
        read_bound!(http_body_max_bytes, "DATABENTO_LWC_HTTP_BODY_MAX_BYTES");
        read_bound!(ws_frame_max_bytes, "DATABENTO_LWC_WS_FRAME_MAX_BYTES");
        read_bound!(max_clients, "DATABENTO_LWC_MAX_CLIENTS");
        read_bound!(
            max_subscriptions_per_client,
            "DATABENTO_LWC_MAX_SUBSCRIPTIONS_PER_CLIENT"
        );
        read_bound!(max_dataset_sessions, "DATABENTO_LWC_MAX_DATASET_SESSIONS");
        read_bound!(
            upstream_queue_capacity,
            "DATABENTO_LWC_UPSTREAM_QUEUE_CAPACITY"
        );
        read_bound!(
            canonical_queue_capacity,
            "DATABENTO_LWC_CANONICAL_QUEUE_CAPACITY"
        );
        read_bound!(
            outbound_queue_capacity,
            "DATABENTO_LWC_OUTBOUND_QUEUE_CAPACITY"
        );
        read_bound!(
            handoff_buffer_capacity,
            "DATABENTO_LWC_HANDOFF_BUFFER_CAPACITY"
        );
        read_bound!(
            reconnect_base_delay_ms,
            "DATABENTO_LWC_RECONNECT_BASE_DELAY_MS"
        );
        read_bound!(
            reconnect_max_delay_ms,
            "DATABENTO_LWC_RECONNECT_MAX_DELAY_MS"
        );
        read_bound!(
            reconnect_max_attempts,
            "DATABENTO_LWC_RECONNECT_MAX_ATTEMPTS"
        );
        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> io::Result<()> {
        let host: IpAddr = self.bind_host.parse().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "bind host must be an IP address",
            )
        })?;
        if host.is_loopback() {
            if !self.allow_unauthenticated_localhost && !self.auth_integration_enabled {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "loopback requires explicit unauthenticated-localhost or auth integration",
                ));
            }
        } else if !self.auth_integration_enabled || self.allow_unauthenticated_localhost {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "non-loopback bind requires auth integration and forbids unauthenticated mode",
            ));
        }
        if self.allowed_origins.is_empty() || self.allowed_datasets.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "origin and dataset allowlists must be non-empty",
            ));
        }
        let positive = [
            self.history_max_intervals,
            self.symbol_max_bytes,
            self.symbol_search_max_results,
            self.http_body_max_bytes,
            self.ws_frame_max_bytes,
            self.max_clients,
            self.max_subscriptions_per_client,
            self.max_dataset_sessions,
            self.upstream_queue_capacity,
            self.canonical_queue_capacity,
            self.outbound_queue_capacity,
            self.handoff_buffer_capacity,
            self.reconnect_max_attempts,
        ];
        if positive.contains(&0)
            || self.reconnect_base_delay_ms == 0
            || self.reconnect_max_delay_ms < self.reconnect_base_delay_ms
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "all capacity bounds must be positive and reconnect max must be at least base",
            ));
        }
        Ok(())
    }
}

fn parse_value<T>(name: &str, value: &str) -> io::Result<T>
where
    T: std::str::FromStr,
{
    value.parse().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid value for {name}"),
        )
    })
}

fn parse_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_defaults_are_valid() {
        GatewayConfig::loopback_localhost().validate().unwrap();
    }

    #[test]
    fn non_loopback_fails_closed_without_auth() {
        let mut config = GatewayConfig::loopback_localhost();
        config.bind_host = "0.0.0.0".to_string();
        assert_eq!(
            config.validate().unwrap_err().kind(),
            io::ErrorKind::PermissionDenied
        );
    }

    #[test]
    fn zero_capacity_is_rejected() {
        let mut config = GatewayConfig::loopback_localhost();
        config.outbound_queue_capacity = 0;
        assert_eq!(
            config.validate().unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }
}
