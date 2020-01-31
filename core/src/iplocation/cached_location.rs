use super::{IpLocation, IpLocationError, IpLocationProvider};
use crate::azure_utils::serde::datetime;
use azure_sdk_storage_core::client::Client as AZClient;
use azure_sdk_storage_table::{
    table::{TableService, TableStorage},
    TableEntry,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
struct CachedData {
    #[serde(with = "datetime")]
    issued: DateTime<Utc>,

    country: String,
    continent: String,
    raw: String,
}

impl CachedData {
    fn from_location(loc: IpLocation) -> CachedData {
        CachedData {
            issued: Utc::now(),
            country: loc.country,
            continent: loc.continent,
            raw: loc.extended.unwrap_or_default(),
        }
    }

    fn into_location(self) -> IpLocation {
        IpLocation {
            country: self.country,
            continent: self.continent,
            extended: if self.raw == "" { None } else { Some(self.raw) },
        }
    }
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

    async fn find_location(&self, ip: IpAddr) -> Result<IpLocation, IpLocationError> {
        //find entry
        let r_key = ip.to_string();
        let p_key = format!("{}", &r_key[0..2]);

        // look up the cache
        if let Ok(Some(loc)) = self.0.cache.get_entry::<CachedData>(&p_key, &r_key).await {
            let age = (Utc::now() - loc.payload.issued).to_std().unwrap_or(self.0.ttl);
            if age < self.0.ttl {
                return Ok(loc.payload.into_location());
            }
        }

        // query form the provider
        let loc = self.0.provider.get_location(ip).await?;
        let loc_entity = TableEntry {
            partition_key: p_key,
            row_key: r_key,
            etag: Some("*".to_string()),
            payload: CachedData::from_location(loc.clone()),
        };

        // update cache
        if let Err(err) = self.0.cache.insert_entry::<CachedData>(loc_entity).await {
            log::warn!("Could not cache ip: {:?}", err);
        }

        Ok(loc)
    }
}

impl IpLocationProvider for IpCachedLocation {
    fn get_location<'s>(&'s self, ip: IpAddr) -> Pin<Box<dyn Future<Output = Result<IpLocation, IpLocationError>> + 's>> {
        Box::pin(self.find_location(ip))
    }
}
