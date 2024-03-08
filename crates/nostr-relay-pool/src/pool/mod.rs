// Copyright (c) 2022-2023 Yuki Kishimoto
// Copyright (c) 2023-2024 Rust Nostr Developers
// Distributed under the MIT software license

//! Relay Pool

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use atomic_destructor::AtomicDestructor;
use nostr::{
    ClientMessage, Event, EventId, Filter, RelayMessage, SubscriptionId, Timestamp, TryIntoUrl, Url,
};
use nostr_database::{DynNostrDatabase, IntoNostrDatabase, MemoryDatabase};
use tokio::sync::broadcast;

mod internal;
pub mod options;

pub use self::internal::Error;
use self::internal::InternalRelayPool;
pub use self::options::RelayPoolOptions;
use crate::relay::options::{FilterOptions, NegentropyOptions, RelayOptions, RelaySendOptions};
use crate::relay::{Relay, RelayStatus};

/// Relay Pool Notification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RelayPoolNotification {
    /// Received an [`Event`]. Does not include events sent by this client.
    Event {
        /// Relay url
        relay_url: Url,
        /// Subscription ID
        subscription_id: SubscriptionId,
        /// Event
        event: Box<Event>,
    },
    /// Received a [`RelayMessage`]. Includes messages wrapping events that were sent by this client.
    Message {
        /// Relay url
        relay_url: Url,
        /// Relay Message
        message: RelayMessage,
    },
    /// Relay status changed
    RelayStatus {
        /// Relay url
        relay_url: Url,
        /// Relay Status
        status: RelayStatus,
    },
    /// Stop
    Stop,
    /// Shutdown
    Shutdown,
}

/// Relay Pool
#[derive(Debug, Clone)]
pub struct RelayPool {
    inner: AtomicDestructor<InternalRelayPool>,
}

impl Default for RelayPool {
    fn default() -> Self {
        Self::new(RelayPoolOptions::default())
    }
}

impl RelayPool {
    /// Create new `RelayPool`
    pub fn new(opts: RelayPoolOptions) -> Self {
        Self::with_database(opts, Arc::new(MemoryDatabase::default()))
    }

    /// New with database
    pub fn with_database<D>(opts: RelayPoolOptions, database: D) -> Self
    where
        D: IntoNostrDatabase,
    {
        Self {
            inner: AtomicDestructor::new(InternalRelayPool::with_database(opts, database)),
        }
    }

    /// Stop
    ///
    /// Call `connect` to re-start relays connections
    pub async fn stop(&self) -> Result<(), Error> {
        self.inner.stop().await
    }

    /// Completely shutdown pool
    pub async fn shutdown(self) -> Result<(), Error> {
        self.inner.shutdown().await
    }

    /// Get new **pool** notification listener
    pub fn notifications(&self) -> broadcast::Receiver<RelayPoolNotification> {
        self.inner.notifications()
    }

    /// Get database
    pub fn database(&self) -> Arc<DynNostrDatabase> {
        self.inner.database()
    }

    /// Get relays
    pub async fn relays(&self) -> HashMap<Url, Relay> {
        self.inner.relays().await
    }

    /// Get [`Relay`]
    pub async fn relay<U>(&self, url: U) -> Result<Relay, Error>
    where
        U: TryIntoUrl,
        Error: From<<U as TryIntoUrl>::Err>,
    {
        self.inner.relay(url).await
    }

    /// Get subscriptions
    pub async fn subscriptions(&self) -> HashMap<SubscriptionId, Vec<Filter>> {
        self.inner.subscriptions().await
    }

    /// Get subscription
    pub async fn subscription(&self, id: &SubscriptionId) -> Option<Vec<Filter>> {
        self.inner.subscription(id).await
    }

    /// Add new relay
    pub async fn add_relay<U>(&self, url: U, opts: RelayOptions) -> Result<bool, Error>
    where
        U: TryIntoUrl,
        Error: From<<U as TryIntoUrl>::Err>,
    {
        self.inner.add_relay(url, opts).await
    }

    /// Disconnect and remove relay
    pub async fn remove_relay<U>(&self, url: U) -> Result<(), Error>
    where
        U: TryIntoUrl,
        Error: From<<U as TryIntoUrl>::Err>,
    {
        self.inner.remove_relay(url).await
    }

    /// Disconnect and remove all relays
    pub async fn remove_all_relays(&self) -> Result<(), Error> {
        self.inner.remove_all_relays().await
    }

    /// Send client message
    pub async fn send_msg(&self, msg: ClientMessage, opts: RelaySendOptions) -> Result<(), Error> {
        self.inner.send_msg(msg, opts).await
    }

    /// Send multiple client messages at once
    pub async fn batch_msg(
        &self,
        msgs: Vec<ClientMessage>,
        opts: RelaySendOptions,
    ) -> Result<(), Error> {
        self.inner.batch_msg(msgs, opts).await
    }

    /// Send client message to specific relays
    ///
    /// Note: **the relays must already be added!**
    pub async fn send_msg_to<I, U>(
        &self,
        urls: I,
        msg: ClientMessage,
        opts: RelaySendOptions,
    ) -> Result<(), Error>
    where
        I: IntoIterator<Item = U>,
        U: TryIntoUrl,
        Error: From<<U as TryIntoUrl>::Err>,
    {
        self.inner.send_msg_to(urls, msg, opts).await
    }

    /// Send multiple client messages at once to specific relays
    ///
    /// Note: **the relays must already be added!**
    pub async fn batch_msg_to<I, U>(
        &self,
        urls: I,
        msgs: Vec<ClientMessage>,
        opts: RelaySendOptions,
    ) -> Result<(), Error>
    where
        I: IntoIterator<Item = U>,
        U: TryIntoUrl,
        Error: From<<U as TryIntoUrl>::Err>,
    {
        self.inner.batch_msg_to(urls, msgs, opts).await
    }

    /// Send event and wait for `OK` relay msg
    pub async fn send_event(&self, event: Event, opts: RelaySendOptions) -> Result<EventId, Error> {
        self.inner.send_event(event, opts).await
    }

    /// Send multiple [`Event`] at once
    pub async fn batch_event(
        &self,
        events: Vec<Event>,
        opts: RelaySendOptions,
    ) -> Result<(), Error> {
        self.inner.batch_event(events, opts).await
    }

    /// Send event to a specific relays
    pub async fn send_event_to<I, U>(
        &self,
        urls: I,
        event: Event,
        opts: RelaySendOptions,
    ) -> Result<EventId, Error>
    where
        I: IntoIterator<Item = U>,
        U: TryIntoUrl,
        Error: From<<U as TryIntoUrl>::Err>,
    {
        self.inner.send_event_to(urls, event, opts).await
    }

    /// Send event to a specific relays
    pub async fn batch_event_to<I, U>(
        &self,
        urls: I,
        events: Vec<Event>,
        opts: RelaySendOptions,
    ) -> Result<(), Error>
    where
        I: IntoIterator<Item = U>,
        U: TryIntoUrl,
        Error: From<<U as TryIntoUrl>::Err>,
    {
        self.inner.batch_event_to(urls, events, opts).await
    }

    /// Subscribe to filters
    pub async fn subscribe(&self, filters: Vec<Filter>, opts: RelaySendOptions) -> SubscriptionId {
        self.inner.subscribe(filters, opts).await
    }

    /// Subscribe to filters with custom [SubscriptionId]
    pub async fn subscribe_with_id(
        &self,
        id: SubscriptionId,
        filters: Vec<Filter>,
        opts: RelaySendOptions,
    ) {
        self.inner.subscribe_with_id(id, filters, opts).await
    }

    /// Unsubscribe from filters
    pub async fn unsubscribe(&self, id: SubscriptionId, opts: RelaySendOptions) {
        self.inner.unsubscribe(id, opts).await
    }

    /// Unsubscribe from filters
    pub async fn unsubscribe_all(&self, opts: RelaySendOptions) {
        self.inner.unsubscribe_all(opts).await
    }

    /// Get events of filters
    ///
    /// Get events both from **local database** and **relays**
    pub async fn get_events_of(
        &self,
        filters: Vec<Filter>,
        timeout: Duration,
        opts: FilterOptions,
    ) -> Result<Vec<Event>, Error> {
        let relays = self.relays().await;
        self.get_events_from(relays.into_keys(), filters, timeout, opts)
            .await
    }

    /// Get events of filters from specific relays
    ///
    /// Get events both from **local database** and **relays**
    ///
    /// If no relay is specified, will be queried only the database.
    pub async fn get_events_from<I, U>(
        &self,
        urls: I,
        filters: Vec<Filter>,
        timeout: Duration,
        opts: FilterOptions,
    ) -> Result<Vec<Event>, Error>
    where
        I: IntoIterator<Item = U>,
        U: TryIntoUrl,
        Error: From<<U as TryIntoUrl>::Err>,
    {
        self.inner
            .get_events_from(urls, filters, timeout, opts)
            .await
    }

    /// Request events of filter.
    ///
    /// If the events aren't already stored in the database, will be sent to notification listener
    /// until the EOSE "end of stored events" message is received from the relay.
    pub async fn req_events_of(
        &self,
        filters: Vec<Filter>,
        timeout: Duration,
        opts: FilterOptions,
    ) {
        let relays = self.relays().await;
        for relay in relays.values() {
            relay.req_events_of(filters.clone(), timeout, opts);
        }
    }

    /// Request events of filter from specific relays.
    ///
    /// If the events aren't already stored in the database, will be sent to notification listener
    /// until the EOSE "end of stored events" message is received from the relay.
    pub async fn req_events_from<I, U>(
        &self,
        urls: I,
        filters: Vec<Filter>,
        timeout: Duration,
        opts: FilterOptions,
    ) -> Result<(), Error>
    where
        I: IntoIterator<Item = U>,
        U: TryIntoUrl,
        Error: From<<U as TryIntoUrl>::Err>,
    {
        let urls: HashSet<Url> = urls
            .into_iter()
            .map(|u| u.try_into_url())
            .collect::<Result<_, _>>()?;
        let relays: HashMap<Url, Relay> = self.relays().await;
        for (_, relay) in relays.into_iter().filter(|(url, ..)| urls.contains(url)) {
            relay.req_events_of(filters.clone(), timeout, opts);
        }
        Ok(())
    }

    /// Connect to all added relays and keep connection alive
    pub async fn connect(&self, connection_timeout: Option<Duration>) {
        self.inner.connect(connection_timeout).await
    }

    /// Disconnect from all relays
    pub async fn disconnect(&self) -> Result<(), Error> {
        self.inner.disconnect().await
    }

    /// Connect to relay
    pub async fn connect_relay<U>(
        &self,
        url: U,
        connection_timeout: Option<Duration>,
    ) -> Result<(), Error>
    where
        U: TryIntoUrl,
        Error: From<<U as TryIntoUrl>::Err>,
    {
        let relay = self.relay(url).await?;
        self.inner.connect_relay(&relay, connection_timeout).await;
        Ok(())
    }

    /// Negentropy reconciliation
    pub async fn reconcile(&self, filter: Filter, opts: NegentropyOptions) -> Result<(), Error> {
        self.inner.reconcile(filter, opts).await
    }

    /// Negentropy reconciliation with custom items
    pub async fn reconcile_with_items(
        &self,
        filter: Filter,
        items: Vec<(EventId, Timestamp)>,
        opts: NegentropyOptions,
    ) -> Result<(), Error> {
        self.inner.reconcile_with_items(filter, items, opts).await
    }
}
