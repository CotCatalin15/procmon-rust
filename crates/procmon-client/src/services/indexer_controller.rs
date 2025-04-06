use std::sync::Arc;

use flume::bounded;

use crate::{event_storage::EventStorage, filters::SimpleFilter, notifier::NotificationBus};

use super::{
    event_filter_service::EventFilterService,
    event_indexer_service::{EventIndexerService, IndexData},
};

pub struct IndexerController {
    bus: NotificationBus,
    num_filter_threads: usize,
    storage: Arc<EventStorage>,

    indexer: EventIndexerService,
    filter: EventFilterService,
}

impl IndexerController {
    pub fn new(
        bus: NotificationBus,
        storage: Arc<EventStorage>,
        num_filter_threads: usize,
    ) -> Self {
        //TODO: Maybe bounded?

        let (indexer, filter) = Self::create_indexer_and_filter(
            bus.clone(),
            storage.clone(),
            num_filter_threads,
            vec![],
        );

        Self {
            bus: bus.clone(),
            num_filter_threads,
            storage: storage.clone(),

            indexer,
            filter,
        }
    }

    pub fn num_events(&self) -> usize {
        self.indexer.len()
    }

    pub fn collect_indicies_into(&self, start: usize, end: usize, collection: &mut Vec<IndexData>) {
        self.indexer.collect_indicies_into(start, end, collection)
    }

    pub fn change_filters(&mut self, new_filters: Vec<SimpleFilter>) {
        (self.indexer, self.filter) = Self::create_indexer_and_filter(
            self.bus.clone(),
            self.storage.clone(),
            self.num_filter_threads,
            new_filters,
        );
    }

    fn create_indexer_and_filter(
        bus: NotificationBus,
        storage: Arc<EventStorage>,
        num_filter_threads: usize,
        filters: Vec<SimpleFilter>,
    ) -> (EventIndexerService, EventFilterService) {
        let (index_sender, index_receiver) = bounded(100_000);

        (
            EventIndexerService::new(index_receiver),
            EventFilterService::new(bus, index_sender, num_filter_threads, storage, filters),
        )
    }
}
