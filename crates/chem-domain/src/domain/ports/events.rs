//! Event Publishing Ports
//!
//! Contracts for publishing domain events to external systems.
//! Enables loose coupling between domain logic and event infrastructure.

use crate::domain::events::DomainEvent;
use crate::DomainError;
use async_trait::async_trait;
use std::collections::HashMap;

/// Event Publisher port for domain events
///
/// This port allows the domain layer to publish events without coupling
/// to specific messaging infrastructure.
#[async_trait]
pub trait EventPublisher: Send + Sync {
  /// Publish a single domain event
  async fn publish(&self, event: Box<dyn DomainEvent>) -> Result<(), DomainError>;

  /// Publish multiple domain events
  async fn publish_batch(&self, events: Vec<Box<dyn DomainEvent>>) -> Result<(), DomainError>;
}

/// Event store contract for event sourcing
///
/// Specialized interface for storing and retrieving events.
/// Can be used for event sourcing or audit trails.
#[async_trait]
pub trait EventStore: Send + Sync {
  /// Store an event with metadata
  async fn store_event<E: DomainEvent>(&self, event: E, metadata: HashMap<String, String>) -> Result<(), DomainError>;

  /// Retrieve events for a specific aggregate
  async fn get_events_for_aggregate(&self,
                                    aggregate_id: &uuid::Uuid,
                                    from_version: Option<u64>)
                                    -> Result<Vec<Box<dyn DomainEvent>>, DomainError>;

  /// Retrieve all events of a specific type
  async fn get_events_by_type(&self,
                              event_type: &str,
                              from_timestamp: Option<chrono::DateTime<chrono::Utc>>)
                              -> Result<Vec<Box<dyn DomainEvent>>, DomainError>;
}

/// Event handler contract
///
/// Handlers process events and can trigger side effects.
/// Multiple handlers can process the same event.
#[async_trait]
pub trait EventHandler<E: DomainEvent>: Send + Sync {
  /// Handle a domain event
  async fn handle(&self, event: E) -> Result<(), DomainError>;

  /// Optional: Return the name of this handler for logging/debugging
  fn handler_name(&self) -> &'static str {
    std::any::type_name::<Self>()
  }
}

/// Event dispatcher coordinates event handling
///
/// Routes events to appropriate handlers and manages error handling.
#[async_trait]
pub trait EventDispatcher: Send + Sync {
  /// Register an event handler for a specific event type
  async fn register_handler<E: DomainEvent + 'static>(&mut self,
                                                      handler: Box<dyn EventHandler<E>>)
                                                      -> Result<(), DomainError>;

  /// Dispatch an event to all registered handlers
  async fn dispatch<E: DomainEvent>(&self, event: E) -> Result<(), DomainError>;

  /// Dispatch multiple events
  async fn dispatch_batch(&self, events: Vec<Box<dyn DomainEvent>>) -> Result<(), DomainError>;
}

/// Outbox pattern implementation
///
/// Ensures reliable event publishing by storing events in the same
/// transaction as domain changes, then publishing them separately.
#[async_trait]
pub trait EventOutbox: Send + Sync {
  /// Add event to outbox (same transaction as domain operation)
  async fn add_event<E: DomainEvent>(&self, event: E) -> Result<(), DomainError>;

  /// Get unpublished events from outbox
  async fn get_unpublished_events(&self) -> Result<Vec<Box<dyn DomainEvent>>, DomainError>;

  /// Mark events as published
  async fn mark_as_published(&self, event_ids: Vec<uuid::Uuid>) -> Result<(), DomainError>;

  /// Clean up old published events
  async fn cleanup_published_events(&self, older_than: chrono::DateTime<chrono::Utc>) -> Result<usize, DomainError>;
}

/// Event subscription management
///
/// Manages event subscriptions for external systems or other bounded contexts.
#[async_trait]
pub trait EventSubscriptionManager: Send + Sync {
  /// Subscribe to specific event types
  async fn subscribe(&self, subscriber_id: &str, event_types: Vec<String>) -> Result<(), DomainError>;

  /// Unsubscribe from event types
  async fn unsubscribe(&self, subscriber_id: &str, event_types: Vec<String>) -> Result<(), DomainError>;

  /// Get all subscribers for an event type
  async fn get_subscribers(&self, event_type: &str) -> Result<Vec<String>, DomainError>;
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::domain::events::*;
  use crate::domain::value_objects::*;
  use std::sync::Arc;
  use uuid::Uuid;

  // Mock implementations for testing
  struct MockEventPublisher;

  #[async_trait]
  impl EventPublisher for MockEventPublisher {
    async fn publish(&self, _event: Box<dyn DomainEvent>) -> Result<(), DomainError> {
      Ok(())
    }

    async fn publish_batch(&self, _events: Vec<Box<dyn DomainEvent>>) -> Result<(), DomainError> {
      Ok(())
    }
  }

  struct MockMoleculeCreatedHandler;

  #[async_trait]
  impl EventHandler<MoleculeCreated> for MockMoleculeCreatedHandler {
    async fn handle(&self, _event: MoleculeCreated) -> Result<(), DomainError> {
      // Simulate handling (e.g., send notification, update search index)
      Ok(())
    }

    fn handler_name(&self) -> &'static str {
      "MockMoleculeCreatedHandler"
    }
  }

  #[tokio::test]
  async fn event_publisher_interface() {
    let publisher: Arc<dyn EventPublisher> = Arc::new(MockEventPublisher);

    let event = MoleculeCreated::new(Uuid::new_v4(),
                                     InChIKey::new("LFQSCWFLJHTTHZ-UHFFFAOYSA-N").unwrap(),
                                     Smiles::new("CCO").unwrap(),
                                     serde_json::json!({}));

    let result = publisher.publish(Box::new(event)).await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn event_handler_interface() {
    let handler = MockMoleculeCreatedHandler;

    let event = MoleculeCreated::new(Uuid::new_v4(),
                                     InChIKey::new("LFQSCWFLJHTTHZ-UHFFFAOYSA-N").unwrap(),
                                     Smiles::new("CCO").unwrap(),
                                     serde_json::json!({}));

    let result = handler.handle(event).await;
    assert!(result.is_ok());
    assert_eq!(handler.handler_name(), "MockMoleculeCreatedHandler");
  }
}
