use anyhow::{anyhow, Result};
#[cfg(unix)]
use std::net::Ipv4Addr;
#[cfg(unix)]
use tokio::io::{AsyncReadExt, AsyncWriteExt};
#[cfg(unix)]
use tracing::info;

/// Minimal wrapper around a TUN device used by the client in TUN mode.
pub struct TunDevice {
    #[cfg(unix)]
    inner: tokio_tun::Tun,
}

impl TunDevice {
    #[cfg(unix)]
    pub async fn create(
        name: &str,
        address: &str,
        netmask: &str,
        mtu: i32,
    ) -> Result<Self> {
        use tokio_tun::TunBuilder;

        let addr: Ipv4Addr = address.parse()?;
        let mask: Ipv4Addr = netmask.parse()?;

        let devs = TunBuilder::new()
            .name(name)
            .address(addr)
            .netmask(mask)
            .mtu(mtu)
            .up()
            .build()?;

        let tun = devs
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("tun builder returned no devices"))?;

        info!(name, address, netmask, mtu, "tun device created");

        Ok(Self { inner: tun })
    }

    #[cfg(not(unix))]
    pub async fn create(
        _name: &str,
        _address: &str,
        _netmask: &str,
        _mtu: i32,
    ) -> Result<Self> {
        Err(anyhow!(
            "TUN devices are only supported on Unix-like targets (e.g. Linux) in Stage 2. \
On non-Unix platforms, use tcp mode instead of tun mode. \
See README: Stage 2 support matrix."
        ))
    }

    #[cfg(unix)]
    pub async fn read_packet(&mut self, buf: &mut [u8]) -> Result<usize> {
        let n = self.inner.read(buf).await?;
        Ok(n)
    }

    #[cfg(unix)]
    pub async fn write_packet(&mut self, buf: &[u8]) -> Result<()> {
        self.inner.write_all(buf).await?;
        Ok(())
    }

    #[cfg(not(unix))]
    pub async fn read_packet(&mut self, _buf: &mut [u8]) -> Result<usize> {
        Err(anyhow!(
            "TunDevice::read_packet is only supported on Unix-like targets (e.g. Linux) in Stage 2"
        ))
    }

    #[cfg(not(unix))]
    pub async fn write_packet(&mut self, _buf: &[u8]) -> Result<()> {
        Err(anyhow!(
            "TunDevice::write_packet is only supported on Unix-like targets (e.g. Linux) in Stage 2"
        ))
    }
}

#[cfg(test)]
mod tests {
    // Only build simple constructor test on non-Windows CI where TUN is expected
    // to be available/configurable. On other platforms this is exercised via
    // manual testing.
    #[cfg(target_os = "linux")]
    #[tokio::test]
    async fn create_tun_device_compiles() {
        let _ = super::TunDevice::create(
            "tun-test0",
            "10.0.0.1",
            "255.255.255.0",
            1500,
        )
        .await;
    }

    // On non-Unix targets we expect TUN creation to fail early with a clear message.
    #[cfg(not(unix))]
    #[tokio::test]
    async fn tun_create_fails_on_non_unix_with_clear_message() {
        let res = super::TunDevice::create(
            "tun-test-non-unix",
            "10.0.0.1",
            "255.255.255.0",
            1500,
        )
        .await;
        assert!(
            res.is_err(),
            "TunDevice::create should fail on non-Unix targets"
        );
        let msg = format!("{:?}", res.err().unwrap());
        assert!(
            msg.contains("Unix-like targets"),
            "error message should clearly communicate Unix-only support, got: {msg}"
        );
    }
}


