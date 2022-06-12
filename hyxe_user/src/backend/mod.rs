use std::sync::Arc;
use std::ops::Deref;
use async_trait::async_trait;
use crate::misc::{AccountError, CNACMetadata};
use crate::client_account::{ClientNetworkAccount, MutualPeer};
use std::path::PathBuf;
use hyxe_fs::env::DirectoryStore;
use crate::prelude::NetworkAccount;
use std::collections::HashMap;
use hyxe_crypt::hyper_ratchet::{Ratchet, HyperRatchet};
use hyxe_crypt::fcm::fcm_ratchet::FcmRatchet;
use hyxe_crypt::fcm::keys::FcmKeys;
#[cfg(feature = "sql")]
use crate::backend::mysql_backend::SqlConnectionOptions;
use parking_lot::RwLock;
#[cfg(feature = "redis")]
use crate::backend::redis_backend::RedisConnectionOptions;

#[cfg(feature = "sql")]
/// Implementation for the SQL backend
pub mod mysql_backend;
#[cfg(feature = "redis")]
/// Implementation for the redis backend
pub mod redis_backend;
/// Implementation for the default filesystem backend
pub mod filesystem_backend;

/// Used when constructing the account manager
#[derive(Clone, Debug)]
pub enum BackendType {
    /// Synchronization will occur on the filesystem
    Filesystem,
    #[cfg(feature = "sql")]
    /// Synchronization will occur on a remote SQL database
    SQLDatabase(String, SqlConnectionOptions),
    #[cfg(feature = "redis")]
    /// Synchronization will occur on a remote redis database
    Redis(String, RedisConnectionOptions)
}

impl Default for BackendType {
    fn default() -> Self {
        Self::Filesystem
    }
}

impl BackendType {
    /// For requesting the use of the FilesystemBackend driver
    pub const fn filesystem() -> BackendType {
        BackendType::Filesystem
    }

    #[cfg(feature = "redis")]
    /// For requesting the use of the redis backend driver.
    /// URL format: redis://[<username>][:<password>@]<hostname>[:port][/<db>]
    /// If unix socket support is available:
    /// URL format: redis+unix:///<path>[?db=<db>[&pass=<password>][&user=<username>]]
    pub fn redis<T: Into<String>>(url: T) -> BackendType {
        Self::redis_with(url, Default::default())
    }

    #[cfg(feature = "redis")]
    /// Like [`Self::redis`], but with custom options
    pub fn redis_with<T: Into<String>>(url: T, opts: RedisConnectionOptions) -> BackendType {
        BackendType::Redis(url.into(), opts)
    }

    /// For requesting the use of the SqlBackend driver. Url should be in the form:
    /// "mysql://username:password@ip/database"
    /// "postgres:// [...]"
    /// "sqlite:// [...]"
    ///
    /// PostgreSQL, MySQL, SqLite supported
    #[cfg(feature = "sql")]
    pub fn sql<T: Into<String>>(url: T) -> BackendType {
        BackendType::SQLDatabase(url.into(), Default::default())
    }

    /// Like [`Self::sql`], but with custom options
    #[cfg(feature = "sql")]
    pub fn sql_with<T: Into<String>>(url: T, opts: SqlConnectionOptions) -> BackendType {
        BackendType::SQLDatabase(url.into(), opts)
    }
}

/// An interface for synchronizing information do differing target
#[async_trait]
pub trait BackendConnection<R: Ratchet, Fcm: Ratchet>: Send + Sync {
    /// This should be run for handling any types of underlying connect operations
    async fn connect(&mut self, directory_store: &DirectoryStore) -> Result<(), AccountError>;
    /// This is called once the PersistenceHandler is loaded
    async fn post_connect(&self, persistence_handler: &PersistenceHandler<R, Fcm>) -> Result<(), AccountError>;
    /// Determines if connected or not
    async fn is_connected(&self) -> Result<bool, AccountError>;
    /// Saves the entire cnac to the DB
    async fn save_cnac(&self, cnac: ClientNetworkAccount<R, Fcm>) -> Result<(), AccountError>;
    /// Find a CNAC by cid
    async fn get_cnac_by_cid(&self, cid: u64) -> Result<Option<ClientNetworkAccount<R, Fcm>>, AccountError>;
    /// Gets the client by username
    async fn get_client_by_username(&self, username: &str) -> Result<Option<ClientNetworkAccount<R, Fcm>>, AccountError>;
    /// Determines if a CID is registered
    async fn cid_is_registered(&self, cid: u64) -> Result<bool, AccountError>;
    /// deletes a CNAC
    async fn delete_cnac(&self, cnac: ClientNetworkAccount<R, Fcm>) -> Result<(), AccountError>;
    /// Removes a CNAC by cid
    async fn delete_cnac_by_cid(&self, cid: u64) -> Result<(), AccountError>;
    /// Saves all CNACs and local NACs
    async fn save_all(&self) -> Result<(), AccountError>;
    /// Removes all CNACs
    async fn purge(&self) -> Result<usize, AccountError>;
    /// Returns the number of clients
    async fn client_count(&self) -> Result<usize, AccountError>;
    /// Maybe generates a local save path, only if required by the implementation
    fn maybe_generate_cnac_local_save_path(&self, cid: u64, is_personal: bool) -> Option<PathBuf>;
    /// Returns a list of unused CIDS
    async fn client_only_generate_possible_cids(&self) -> Result<Vec<u64>, AccountError>;
    /// Searches the internal database/nac for the first cid that is unused
    async fn find_first_valid_cid(&self, possible_cids: &Vec<u64>) -> Result<Option<u64>, AccountError>;
    /// Determines if a username exists
    async fn username_exists(&self, username: &str) -> Result<bool, AccountError>;
    /// Registers a CID to the db/fs, preventing future registrants from using the same values
    async fn register_cid_in_nac(&self, cid: u64, username: &str) -> Result<(), AccountError>;
    /// Returns a list of impersonal cids
    async fn get_registered_impersonal_cids(&self, limit: Option<i32>) -> Result<Option<Vec<u64>>, AccountError>;
    /// Gets the username by CID
    async fn get_username_by_cid(&self, cid: u64) -> Result<Option<String>, AccountError>;
    /// Gets the CID by username
    async fn get_cid_by_username(&self, username: &str) -> Result<Option<u64>, AccountError>;
    /// Deletes a client by username
    async fn delete_client_by_username(&self, username: &str) -> Result<(), AccountError>;
    /// Registers two peers together
    async fn register_p2p_as_server(&self, cid0: u64, cid1: u64) -> Result<(), AccountError>;
    /// registers p2p as client
    async fn register_p2p_as_client(&self, implicated_cid: u64, peer_cid: u64, peer_username: String) -> Result<(), AccountError>;
    /// Deregisters two peers from each other
    async fn deregister_p2p_as_server(&self, cid0: u64, cid1: u64) -> Result<(), AccountError>;
    /// Deregisters two peers from each other
    async fn deregister_p2p_as_client(&self, implicated_cid: u64, peer_cid: u64) -> Result<Option<MutualPeer>, AccountError>;
    /// Gets the FCM keys for a peer
    async fn get_fcm_keys_for_as_server(&self, implicated_cid: u64, peer_cid: u64) -> Result<Option<FcmKeys>, AccountError>;
    /// Updates the FCM keys
    async fn update_fcm_keys(&self, cnac: &ClientNetworkAccount<R, Fcm>, new_keys: FcmKeys) -> Result<(), AccountError>;
    /// Returns a list of hyperlan peers for the client
    async fn get_hyperlan_peer_list(&self, implicated_cid: u64) -> Result<Option<Vec<u64>>, AccountError>;
    /// Returns the metadata for a client
    async fn get_client_metadata(&self, implicated_cid: u64) -> Result<Option<CNACMetadata>, AccountError>;
    /// Gets all the metadata for many clients
    async fn get_clients_metadata(&self, limit: Option<i32>) -> Result<Vec<CNACMetadata>, AccountError>;
    /// Gets hyperlan peer
    async fn get_hyperlan_peer_by_cid(&self, implicated_cid: u64, peer_cid: u64) -> Result<Option<MutualPeer>, AccountError>;
    /// Determines if the peer exists or not
    async fn hyperlan_peer_exists(&self, implicated_cid: u64, peer_cid: u64) -> Result<bool, AccountError>;
    /// Determines if the input cids are mutual to the implicated cid in order
    async fn hyperlan_peers_are_mutuals(&self, implicated_cid: u64, peers: &Vec<u64>) -> Result<Vec<bool>, AccountError>;
    /// Returns a set of PeerMutual containers
    async fn get_hyperlan_peers(&self, implicated_cid: u64, peers: &Vec<u64>) -> Result<Vec<MutualPeer>, AccountError>;
    /// Returns a set of PeerMutual containers. Should only be called on the endpoints
    async fn get_hyperlan_peers_with_fcm_keys_as_client(&self, implicated_cid: u64, peers: &Vec<u64>) -> Result<Vec<(MutualPeer, Option<FcmKeys>)>, AccountError> {
        if peers.is_empty() {
            return Ok(Vec::new())
        }

        let cnac = self.get_cnac_by_cid(implicated_cid).await?.ok_or(AccountError::ClientNonExists(implicated_cid))?;
        Ok(cnac.get_hyperlan_peers_with_fcm_keys(peers).ok_or(AccountError::Generic("No peers exist locally".into()))?)
    }
    /// Gets hyperland peer by username
    async fn get_hyperlan_peer_by_username(&self, implicated_cid: u64, username: &str) -> Result<Option<MutualPeer>, AccountError>;
    /// Gets all peer cids with fcm keys
    async fn get_hyperlan_peer_list_with_fcm_keys_as_server(&self, implicated_cid: u64) -> Result<Option<Vec<(u64, Option<String>, Option<FcmKeys>)>>, AccountError>;
    /// Synchronizes the list locally. Returns true if needs to be saved
    async fn synchronize_hyperlan_peer_list_as_client(&self, cnac: &ClientNetworkAccount<R, Fcm>, peers: Vec<(u64, Option<String>, Option<FcmKeys>)>) -> Result<bool, AccountError>;
    /// Returns a vector of bytes from the byte map
    async fn get_byte_map_value(&self, implicated_cid: u64, peer_cid: u64, key: &str, sub_key: &str) -> Result<Option<Vec<u8>>, AccountError>;
    /// Removes a value from the byte map, returning the previous value
    async fn remove_byte_map_value(&self, implicated_cid: u64, peer_cid: u64, key: &str, sub_key: &str) -> Result<Option<Vec<u8>>, AccountError>;
    /// Stores a value in the byte map, either creating or overwriting any pre-existing value
    async fn store_byte_map_value(&self, implicated_cid: u64, peer_cid: u64, key: &str, sub_key: &str, value: Vec<u8>) -> Result<Option<Vec<u8>>, AccountError>;
    /// Obtains a list of K,V pairs such that they reside inside `key`
    async fn get_byte_map_values_by_key(&self, implicated_cid: u64, peer_cid: u64, key: &str) -> Result<HashMap<String, Vec<u8>>, AccountError>;
    /// Obtains a list of K,V pairs such that `needle` is a subset of the K value
    async fn remove_byte_map_values_by_key(&self, implicated_cid: u64, peer_cid: u64, key: &str) -> Result<HashMap<String, Vec<u8>>, AccountError>;
    /// Stores the CNAC inside the hashmap, if possible (may be no-op on database)
    fn store_cnac(&self, cnac: ClientNetworkAccount<R, Fcm>);
    /// Determines if a remote db is used
    fn uses_remote_db(&self) -> bool;
    /// Returns the filesystem list
    fn get_local_map(&self) -> Option<Arc<RwLock<HashMap<u64, ClientNetworkAccount<R, Fcm>>>>>;
    /// Returns the local nac
    fn local_nac(&self) -> &NetworkAccount<R, Fcm>;
}

/// This is what every C/NAC gets. This gets called before making I/O operations
pub struct PersistenceHandler<R: Ratchet = HyperRatchet, Fcm: Ratchet = FcmRatchet> {
    inner: Arc<dyn BackendConnection<R, Fcm>>,
    directory_store: DirectoryStore
}

impl<R: Ratchet, Fcm: Ratchet> PersistenceHandler<R, Fcm> {
    /// Creates a new persistence handler
    pub fn new<T: BackendConnection<R, Fcm> + 'static>(inner: T, directory_store: DirectoryStore) -> Self {
        Self { inner: Arc::new(inner), directory_store }
    }

    /// Returns the inner directory store
    pub fn directory_store(&self) -> &DirectoryStore {
        &self.directory_store
    }
}

impl<R: Ratchet, Fcm: Ratchet> Deref for PersistenceHandler<R, Fcm> {
    type Target = Arc<dyn BackendConnection<R, Fcm>>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<R: Ratchet, Fcm: Ratchet> Clone for PersistenceHandler<R, Fcm> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone(), directory_store: self.directory_store.clone() }
    }
}