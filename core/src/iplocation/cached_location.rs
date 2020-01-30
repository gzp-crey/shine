use super::{IpLocation, IpLocationError, IpLocationProvider};
use azure_sdk_storage_core::client::Client as AZClient;
use azure_sdk_storage_table::{
    table::{TableService, TableStorage},
    TableEntry,
};
use std::future::Future;
use std::net::IpAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct IpCachedLocationConfig {
    pub storage_account: String,
    pub storage_account_key: String,
    pub table_name: String,

    pub time_to_live: Duration,
}

struct Inner {
    provider: Box<dyn IpLocationProvider>,
    ttl: Duration,
    cache: TableStorage,
}

#[derive(Clone)]
pub struct IpCachedLocation(Arc<Inner>);

impl IpCachedLocation {
    pub async fn new<P: 'static + IpLocationProvider>(
        provider: P,
        config: IpCachedLocationConfig,
    ) -> Result<Self, IpLocationError> {
        let client = AZClient::new(&config.storage_account, &config.storage_account_key)?;
        let table_service = TableService::new(client.clone());
        let cache = TableStorage::new(table_service.clone(), config.table_name);

        cache.create_if_not_exists().await?;

        Ok(IpCachedLocation(Arc::new(Inner {
            provider: Box::new(provider),
            ttl: config.time_to_live,
            cache,
        })))
    }
}

impl IpLocationProvider for IpCachedLocation {
    fn get_location<'s>(&'s self, ip: IpAddr) -> Pin<Box<dyn Future<Output = Result<IpLocation, IpLocationError>> + 's>> {
        unimplemented!()
    }
}
