#[derive(Debug, Clone)]
pub struct BuildInfo {
    pub version: &'static str,
    pub git_commit: Option<&'static str>,
    pub build_ts_ms: Option<&'static str>,
}

impl BuildInfo {
    pub fn current() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION"),
            git_commit: option_env!("VPNNODE_GIT_COMMIT"),
            build_ts_ms: option_env!("VPNNODE_BUILD_TS_MS"),
        }
    }

    pub fn short(&self) -> String {
        let mut s = format!("vpnnode v{}", self.version);
        if let Some(c) = self.git_commit {
            let short = c.get(0..8).unwrap_or(c);
            s.push_str(&format!(" commit={short}"));
        }
        if let Some(ts) = self.build_ts_ms {
            s.push_str(&format!(" build_ts_ms={ts}"));
        }
        s
    }
}

