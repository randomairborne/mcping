mod bedrock;
mod java;

use hickory_resolver::{
    TokioResolver, config::ResolverConfig, name_server::TokioConnectionProvider,
    proto::runtime::TokioRuntimeProvider,
};

use crate::Error;

/// Represents a pingable entity.
pub trait AsyncPingable {
    /// The type of response that is expected in reply to the ping.
    type Response;

    /// Ping the entity, gathering the latency and response.
    fn ping(
        self,
        pinger: &Pinger,
    ) -> impl std::future::Future<Output = Result<(u64, Self::Response), Error>> + Send;
}

pub struct Pinger {
    resolver: TokioResolver,
}

impl Pinger {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Ping a server
    ///
    /// # Errors
    /// When a server cannot be connected to
    pub async fn ping<P: AsyncPingable + Send>(
        &self,
        ping: P,
    ) -> Result<(u64, P::Response), Error> {
        ping.ping(self).await
    }
}

impl Default for Pinger {
    fn default() -> Self {
        let config = ResolverConfig::cloudflare();
        let conn_provider = TokioConnectionProvider::new(TokioRuntimeProvider::new());
        let mut resolver = TokioResolver::builder_with_config(config, conn_provider);
        resolver.options_mut().attempts = 3;
        resolver.options_mut().cache_size = 1024;
        let resolver = resolver.build();
        Self { resolver }
    }
}
