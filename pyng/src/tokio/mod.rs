mod bedrock;
mod java;

use std::sync::OnceLock;

use hickory_resolver::{
    TokioAsyncResolver,
    config::{ResolverConfig, ResolverOpts},
};

use crate::Error;

/// Represents a pingable entity.
pub trait AsyncPingable {
    /// The type of response that is expected in reply to the ping.
    type Response;

    /// Ping the entity, gathering the latency and response.
    fn ping(self)
    -> impl std::future::Future<Output = Result<(u64, Self::Response), Error>> + Send;
}

/// Retrieve the status of a given Minecraft server using a `AsyncPingable` configuration.
///
///
/// Returns `(latency_ms, response)` where response is a response type of the `Pingable` configuration.
///
/// # Examples
///
/// Ping a Java Server with no timeout:
///
/// ```no_run
/// # async {
/// use std::time::Duration;
///
/// let (latency, response) = pyng::tokio::get_status(pyng::Java {
///     server_address: "mc.hypixel.net".into(),
///     timeout: None,
/// }).await?;
/// # Ok::<(), pyng::Error>(())
/// # };
/// ```
///
/// Ping a Bedrock server with no timeout, trying 3 times:
///
/// ```no_run
/// # async {
/// use std::time::Duration;
///
/// let (latency, response) = pyng::tokio::get_status(pyng::Bedrock {
///     server_address: "play.nethergames.org".into(),
///     timeout: None,
///     tries: 3,
///     ..Default::default()
/// }).await?;
/// # Ok::<(), pyng::Error>(())
/// # };
/// ```
///
/// # Errors
/// If the server status cannot be recieved
pub async fn get_status<P: AsyncPingable + Send>(pingable: P) -> Result<(u64, P::Response), Error> {
    pingable.ping().await
}

fn new_resolver() -> TokioAsyncResolver {
    let config = ResolverConfig::cloudflare();
    let mut opts = ResolverOpts::default();
    opts.cache_size = 64;
    opts.attempts = 3;
    TokioAsyncResolver::tokio(config, opts)
}

pub fn resolver() -> &'static TokioAsyncResolver {
    static RESOLVER: OnceLock<TokioAsyncResolver> = OnceLock::new();
    RESOLVER.get_or_init(new_resolver)
}
