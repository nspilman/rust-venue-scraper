use serde::{Deserialize, Serialize};

use crate::domain::{Artist, Event, Venue};
use crate::pipeline::processing::conflation::{EntityId, EntityType};

/// Represents an entity that is a candidate for cataloging
/// This is the intermediate stage between conflation and persistence
#[derive(Debug, Clone)]
pub struct CatalogCandidate {
    /// The proposed state to persist
    pub proposed_state: ProposedEntity,
    
    /// The current state in storage (if it exists)
    pub current_state: Option<PersistedEntity>,
    
    /// What changes were detected
    pub changes: ChangeSet,
    
    /// Whether this should be persisted
    pub should_persist: bool,
}

/// The entity proposed for persistence
#[derive(Debug, Clone)]
pub enum ProposedEntity {
    Venue(Venue),
    Event(Event),
    Artist(Artist),
}

/// The current persisted state (if it exists)
#[derive(Debug, Clone)]
pub enum PersistedEntity {
    Venue,
    Event,
    Artist,
}

/// Describes what changes were detected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeSet {
    /// Whether this is a new entity
    pub is_new: bool,
    
    /// Whether there are any changes
    pub has_changes: bool,
    
    /// List of fields that changed
    pub changed_fields: Vec<FieldChange>,
    
    /// Human-readable summary of changes
    pub change_summary: String,
}

/// Represents a single field change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldChange {
    /// Name of the field that changed
    pub field_name: String,
    
    /// Previous value (as string for simplicity)
    pub old_value: Option<String>,
    
    /// New value (as string for simplicity)
    pub new_value: Option<String>,
}

impl CatalogCandidate {
    /// Create a new candidate for a brand new entity
    pub fn new_entity(
        _entity_type: EntityType,
        _canonical_id: EntityId,
        proposed: ProposedEntity,
    ) -> Self {
        let change_summary = match &proposed {
            ProposedEntity::Venue(v) => format!("New venue: {}", v.name),
            ProposedEntity::Event(e) => format!("New event: {}", e.title),
            ProposedEntity::Artist(a) => format!("New artist: {}", a.name),
        };

        Self {
            proposed_state: proposed,
            current_state: None,
            changes: ChangeSet {
                is_new: true,
                has_changes: true,
                changed_fields: vec![],
                change_summary,
            },
            should_persist: true,
        }
    }

    /// Create a candidate for an existing entity with potential updates
    pub fn existing_entity(
        _entity_type: EntityType,
        _canonical_id: EntityId,
        proposed: ProposedEntity,
        current: PersistedEntity,
        changes: ChangeSet,
    ) -> Self {
        Self {
            proposed_state: proposed,
            current_state: Some(current),
            changes: changes.clone(),
            should_persist: changes.has_changes,
        }
    }

    /// Check if this candidate represents a new entity
    pub fn is_new(&self) -> bool {
        self.current_state.is_none()
    }

    /// Check if this candidate has changes worth persisting
    pub fn has_changes(&self) -> bool {
        self.changes.has_changes
    }
}

impl ChangeSet {
    /// Create an empty changeset (no changes)
    pub fn no_changes() -> Self {
        Self {
            is_new: false,
            has_changes: false,
            changed_fields: vec![],
            change_summary: "No changes detected".to_string(),
        }
    }

    /// Add a field change
    pub fn add_change(&mut self, field_name: impl Into<String>, old: Option<String>, new: Option<String>) {
        self.changed_fields.push(FieldChange {
            field_name: field_name.into(),
            old_value: old,
            new_value: new,
        });
        self.has_changes = true;
    }

    /// Create a summary of changes
    pub fn summarize(&self) -> String {
        if !self.has_changes {
            return "No changes".to_string();
        }
        
        if self.is_new {
            return self.change_summary.clone();
        }

        let field_names: Vec<String> = self.changed_fields
            .iter()
            .map(|f| f.field_name.clone())
            .collect();
            
        format!("Updated fields: {}", field_names.join(", "))
    }
}
