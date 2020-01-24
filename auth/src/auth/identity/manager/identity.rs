use super::super::{error::IdentityError, IdentityManager};
use argon2;
use azure_sdk_storage_table::TableEntry;
use azure_utils::table_storage::EmptyData;
use percent_encoding::{self, utf8_percent_encode};
use rand::{self, seq::SliceRandom};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use shine_core::session::UserId;
use shine_core::{
    azure_utils,
    backoff::{self, Backoff, BackoffError},
};
use std::{str, time::Duration};
use validator::validate_email;

const ID_LEN: usize = 8;
const ID_ABC: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";

const MAX_SALT_LEN: usize = 32;
const SALT_ABC: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

/// Data associated to each identity
pub trait IdentityData: Serialize + DeserializeOwned {
    /// Return the identity core, the common properties for all type of identites
    fn core(&self) -> &IdentityCore;
}

#[derive(Debug, Serialize, Deserialize)]
pub enum IdentityCategory {
    User,
}

/// Identity
pub trait Identity {
    type Data: IdentityData;

    /// Generate partition and row keys from the id of an identity
    fn entity_keys(id: &str) -> (String, String);

    /// Create Self from the stored table entity
    fn from_entity(data: TableEntry<Self::Data>) -> Self
    where
        Self: Sized;

    /// Create a the table entity to store from Self
    fn into_entity(self) -> TableEntry<Self::Data>;

    /// Return the associated data
    fn into_data(self) -> Self::Data;

    /// Return the data associated to an identity
    fn data(&self) -> &Self::Data;

    /// Return the mutable data associated to an identity
    fn data_mut(&mut self) -> &mut Self::Data;

    /// Return the identity core, the common properties for all type of identites
    fn core(&self) -> &IdentityCore {
        self.data().core()
    }

    /// Return the (unique) id of the identity
    fn id(&self) -> &str {
        &self.core().id
    }

    /// Return the (unique) name of an identity
    fn name(&self) -> &str {
        &self.core().name
    }
}

/// Data associated to each identity
pub trait IdentityIndexData: Serialize + DeserializeOwned {
    /// Id of the associated identity
    fn id(&self) -> &str;
}

pub trait IdentityIndex {
    type Index: IdentityIndexData;

    /// Generate partition and row keys from the key to use as the indexed for an identity
    fn entity_keys(key: &str) -> (String, String);

    /// Create Self from the stored table entity
    fn from_entity(data: TableEntry<Self::Index>) -> Self
    where
        Self: Sized;

    /// Create a the table entity to store from Self
    fn into_entity(self) -> TableEntry<Self::Index>;

    /// Return the associated data
    fn into_data(self) -> Self::Index;

    /// Return the data associated to the index (and not to the identity)
    fn data(&self) -> &Self::Index;

    /// Return the mutable data associated to the index (and not to the identity)
    fn data_mut(&mut self) -> &mut Self::Index;

    /// The unique key to index
    fn index_key(&self) -> &str;

    /// Return the (unique) id of the identity
    fn id(&self) -> &str {
        self.data().id()
    }
}

/// General index data for identity indices
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct IdentityIndexedId {
    pub identity_id: String,
}

/// Common data associated to each identity
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct IdentityCore {
    pub id: String,
    pub sequence_id: u64,
    pub salt: String,
    pub category: IdentityCategory,
    pub name: String,
}

/// Data associated to a user identity
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct UserIdentityData {
    #[serde(flatten)]
    pub core: IdentityCore,

    pub email: Option<String>,
    pub email_validated: bool,
    pub password_hash: String,
}

impl IdentityData for UserIdentityData {
    fn core(&self) -> &IdentityCore {
        &self.core
    }
}

/// Identity assigned to a user
#[derive(Debug)]
pub struct UserIdentity(TableEntry<UserIdentityData>);

impl UserIdentity {
    pub fn new(
        id: String,
        sequence_id: u64,
        salt: String,
        name: String,
        email: Option<String>,
        password_hash: String,
    ) -> UserIdentity {
        let (partition_key, row_key) = Self::entity_keys(&id);
        UserIdentity(TableEntry {
            partition_key,
            row_key,
            etag: None,
            payload: UserIdentityData {
                core: IdentityCore {
                    id,
                    sequence_id,
                    salt,
                    name,
                    category: IdentityCategory::User,
                },
                email,
                email_validated: false,
                password_hash,
            },
        })
    }
}

impl Identity for UserIdentity {
    type Data = UserIdentityData;

    fn entity_keys(id: &str) -> (String, String) {
        (id[0..2].to_string(), id.to_string())
    }

    fn from_entity(entity: TableEntry<UserIdentityData>) -> Self {
        Self(entity)
    }

    fn into_entity(self) -> TableEntry<UserIdentityData> {
        self.0
    }

    fn into_data(self) -> UserIdentityData {
        self.0.payload
    }

    fn data(&self) -> &UserIdentityData {
        &self.0.payload
    }

    fn data_mut(&mut self) -> &mut UserIdentityData {
        &mut self.0.payload
    }
}

impl From<UserIdentity> for UserId {
    fn from(user: UserIdentity) -> Self {
        let data = user.into_data();
        UserId::new(data.core.id, data.core.name, vec![] /*user.roles*/)
    }
}

/// Data associated to an identity index by name
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct NameIndexData {
    #[serde(flatten)]
    pub indexed_id: IdentityIndexedId,
}

impl IdentityIndexData for NameIndexData {
    fn id(&self) -> &str {
        &self.indexed_id.identity_id
    }
}

/// Index identity by name
#[derive(Debug)]
struct NameIndex(TableEntry<NameIndexData>);

impl NameIndex {
    pub fn from_identity<T>(identity: &T) -> Self
    where
        T: Identity,
    {
        let name = &identity.name();
        let (partition_key, row_key) = <Self as IdentityIndex>::entity_keys(name);
        Self(TableEntry {
            partition_key,
            row_key,
            etag: None,
            payload: NameIndexData {
                indexed_id: IdentityIndexedId {
                    identity_id: identity.id().to_owned(),
                },
            },
        })
    }
}

impl IdentityIndex for NameIndex {
    type Index = NameIndexData;

    fn entity_keys(id: &str) -> (String, String) {
        (format!("name-{}", &id[0..2]), id.to_string())
    }

    fn from_entity(entity: TableEntry<NameIndexData>) -> Self {
        Self(entity)
    }

    fn into_entity(self) -> TableEntry<NameIndexData> {
        self.0
    }

    fn into_data(self) -> NameIndexData {
        self.0.payload
    }

    fn data(&self) -> &NameIndexData {
        &self.0.payload
    }

    fn data_mut(&mut self) -> &mut NameIndexData {
        &mut self.0.payload
    }

    fn index_key(&self) -> &str {
        &self.0.row_key
    }
}

/// Storage type for index by email
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct EmailIndexData {
    #[serde(flatten)]
    pub indexed_id: IdentityIndexedId,
}

impl IdentityIndexData for EmailIndexData {
    fn id(&self) -> &str {
        &self.indexed_id.identity_id
    }
}

#[derive(Debug)]
pub struct EmailIndex(TableEntry<EmailIndexData>);

impl EmailIndex {
    pub fn entity_keys(email: &str) -> (String, String) {
        (format!("email-{}", &email[0..2]), email.to_string())
    }

    pub fn from_identity(identity: &UserIdentity) -> Option<Self> {
        if let Some(ref email) = identity.data().email {
            let (partition_key, row_key) = Self::entity_keys(email);
            Some(EmailIndex(TableEntry {
                partition_key,
                row_key,
                etag: None,
                payload: EmailIndexData {
                    indexed_id: IdentityIndexedId {
                        identity_id: identity.id().to_owned(),
                    },
                },
            }))
        } else {
            None
        }
    }
}

impl IdentityIndex for EmailIndex {
    type Index = EmailIndexData;

    fn entity_keys(id: &str) -> (String, String) {
        let id = id.splitn(2, '-').skip(1).next().unwrap();
        (id[0..2].to_string(), id.to_string())
    }

    fn from_entity(entity: TableEntry<EmailIndexData>) -> Self {
        Self(entity)
    }

    fn into_entity(self) -> TableEntry<EmailIndexData> {
        self.0
    }

    fn into_data(self) -> EmailIndexData {
        self.0.payload
    }

    fn data(&self) -> &EmailIndexData {
        &self.0.payload
    }

    fn data_mut(&mut self) -> &mut EmailIndexData {
        &mut self.0.payload
    }

    fn index_key(&self) -> &str {
        &self.0.row_key
    }
}

fn validate_username(name: &str) -> bool {
    name.chars().all(char::is_alphanumeric)
}

// Handling identites
impl IdentityManager {
    pub(crate) async fn remove_index<T>(&self, index: T)
    where
        T: IdentityIndex,
    {
        let index = index.into_entity();
        self.indices
            .delete_entry(&index.partition_key, &index.row_key, index.etag.as_deref())
            .await
            .unwrap_or_else(|e| log::error!("Failed to delete index: {}", e));
    }

    pub(crate) async fn find_identity_by_index<T>(&self, query: &str) -> Result<T, IdentityError>
    where
        T: Identity,
    {
        let mut index = self.indices.query_entries::<IdentityIndexedId>(Some(&query)).await?;
        assert!(index.len() <= 1);
        let index = index.pop().ok_or(IdentityError::IdentityNotFound)?;

        let identity_id = &index.payload.identity_id;
        let (p, r) = T::entity_keys(&identity_id);
        let identity = self.identities.get_entry(&p, &r).await?;
        let identity = identity.map(T::from_entity).ok_or(IdentityError::IdentityNotFound)?;

        Ok(identity)
    }

    pub(crate) async fn find_user_by_index(&self, query: &str, password: Option<&str>) -> Result<UserIdentity, IdentityError> {
        let identity = self.find_identity_by_index::<UserIdentity>(query).await?;

        if let Some(password) = password {
            // check password if provided
            if !argon2::verify_encoded(&identity.data().password_hash, password.as_bytes())? {
                return Err(IdentityError::PasswordNotMatching);
            }
        }

        Ok(identity)
    }

    async fn remove_identity<T>(&self, identity: T)
    where
        T: Identity,
    {
        let identity = identity.into_entity();
        self.identities
            .delete_entry(&identity.partition_key, &identity.row_key, identity.etag.as_deref())
            .await
            .unwrap_or_else(|e| {
                log::error!(
                    "Failed to delete identity([{}]/[{}]): {}",
                    identity.partition_key,
                    identity.row_key,
                    e
                )
            });
    }

    /// Return if the given name can be used as a new identity name
    pub async fn is_name_available(&self, name: &str) -> Result<bool, IdentityError> {
        let (partition_key, row_key) = NameIndex::entity_keys(name);
        Ok(self.indices.get_entry::<EmptyData>(&partition_key, &row_key).await?.is_none())
    }

    async fn insert_name_index(&self, identity: &UserIdentity) -> Result<NameIndex, IdentityError> {
        let name_index = NameIndex::from_identity(identity);
        match self.indices.insert_entry(name_index.into_entity()).await {
            Ok(name_index) => Ok(NameIndex::from_entity(name_index)),
            Err(e) => {
                if azure_utils::is_precodition_error(&e) {
                    Err(IdentityError::NameTaken)
                } else {
                    Err(IdentityError::from(e))
                }
            }
        }
    }

    /// Return if the given email can be used.
    pub async fn is_email_available(&self, email: &str) -> Result<bool, IdentityError> {
        let (partition_key, row_key) = EmailIndex::entity_keys(email);
        Ok(self.indices.get_entry::<EmptyData>(&partition_key, &row_key).await?.is_none())
    }

    async fn insert_email_index(&self, identity: &UserIdentity) -> Result<Option<EmailIndex>, IdentityError> {
        if let Some(email_index) = EmailIndex::from_identity(&identity) {
            match self.indices.insert_entry(email_index.into_entity()).await {
                Ok(email_index) => Ok(Some(EmailIndex::from_entity(email_index))),
                Err(err) => {
                    if azure_utils::is_precodition_error(&err) {
                        Err(IdentityError::EmailTaken)
                    } else {
                        Err(IdentityError::from(err))
                    }
                }
            }
        } else {
            Ok(None)
        }
    }

    async fn try_create_user_identity(
        &self,
        sequence_id: u64,
        name: &str,
        password: &str,
        email: Option<&str>,
    ) -> Result<UserIdentity, BackoffError<IdentityError>> {
        let mut rng = rand::thread_rng();
        let salt = String::from_utf8(SALT_ABC.choose_multiple(&mut rng, MAX_SALT_LEN).cloned().collect::<Vec<_>>()).unwrap();
        let id = String::from_utf8(ID_ABC.choose_multiple(&mut rng, ID_LEN).cloned().collect::<Vec<_>>()).unwrap();
        let password_config = argon2::Config::default();
        let password_hash = argon2::hash_encoded(password.as_bytes(), salt.as_bytes(), &password_config)
            .map_err(IdentityError::from)
            .map_err(IdentityError::into_backoff)?;

        log::info!("Created new user id:{}, pwh:{}", id, password_hash);
        let identity = UserIdentity::new(
            id,
            sequence_id,
            salt,
            name.to_owned(),
            email.map(|e| e.to_owned()),
            password_hash,
        );

        let identity = self
            .identities
            .insert_entry(identity.into_entity())
            .await
            .map_err(|err| {
                if azure_utils::is_precodition_error(&err) {
                    IdentityError::IdentityIdConflict
                } else {
                    IdentityError::from(err)
                }
            })
            .map_err(IdentityError::into_backoff)?;

        Ok(UserIdentity::from_entity(identity))
    }

    /// Creates a new user identity.
    pub async fn create_user(
        &self,
        name: String,
        email: Option<String>,
        password: String,
    ) -> Result<UserIdentity, IdentityError> {
        // validate input
        if !validate_username(&name) {
            log::info!("Invalid user name: {}", name);
            return Err(IdentityError::InvalidName);
        }
        if let Some(ref email) = email {
            if !validate_email(email) {
                log::info!("Invalid email: {}", email);
                return Err(IdentityError::InvalidEmail);
            }
        }

        // preliminary db checks (reduce the number of rollbacks)
        if !self.is_name_available(&name).await? {
            log::info!("User name {} already taken", name);
            return Err(IdentityError::NameTaken);
        }
        if let Some(ref email) = email {
            if !self.is_email_available(email).await? {
                log::info!("Email {} already taken", email);
                return Err(IdentityError::EmailTaken);
            }
        }

        let identity = {
            let sequence_id = self.identity_id_generator.get().await?;
            backoff::Exponential::new(3, Duration::from_micros(10))
                .async_execute(|_| self.try_create_user_identity(sequence_id, &name, &password, email.as_deref()))
                .await?
        };

        let name_index = match self.insert_name_index(&identity).await {
            Ok(index) => index,
            Err(e) => {
                log::info!("Creating user failed (name_index): {:?}, {:?}", identity, e);
                self.remove_identity(identity).await;
                return Err(e);
            }
        };

        let email_index = match self.insert_email_index(&identity).await {
            Ok(index) => index,
            Err(e) => {
                log::info!("Creating user failed (email_index): {:?}, {:?}", identity, e);
                self.remove_identity(identity).await;
                self.remove_index(name_index).await;
                return Err(e);
            }
        };

        log::info!("New user registered: {:?}", identity);
        log::debug!("Name index: {:?}", name_index);
        log::debug!("Email index: {:?}", email_index);
        Ok(identity)
    }

    /// Find a user identity by email or name.
    /// If a password it is also checked.
    pub async fn find_user_by_name_email(&self, name_email: &str, password: Option<&str>) -> Result<UserIdentity, IdentityError> {
        let query_name = {
            let (p, r) = NameIndex::entity_keys(name_email);
            format!("PartitionKey eq '{}' and RowKey eq '{}'", p, r)
        };
        let query_email = {
            let (p, r) = EmailIndex::entity_keys(name_email);
            format!("PartitionKey eq '{}' and RowKey eq '{}'", p, r)
        };
        let query = format!("(({}) or ({}))", query_name, query_email);
        let query = format!("$filter={}", utf8_percent_encode(&query, percent_encoding::NON_ALPHANUMERIC));

        self.find_user_by_index(&query, password).await
    }
}
