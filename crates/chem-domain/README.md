# Chem Domain
> 🧬 Pure Domain Core for Chemical Information Management
[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
## Overview
`chem-domain` is the pure domain core of the FlowChem chemical information management system, implementing **Hexagonal Architecture** (Ports and Adapters) and **SOLID principles**. This crate contains all business logic, domain entities, and contracts without any external dependencies.
## 🏗️ Architecture
### Hexagonal Architecture
```
                    🏛️ DOMAIN CORE (chem-domain)
                    ┌─────────────────────────────┐
                    │                             │
     📝 Application │  🎯 Use Cases               │  📊 Domain Services
     ────────────── │  - CreateMoleculeUseCase    │  ─────────────────
     Web API    ⟶  │  - GetFamilyUseCase         │  ⟵  MoleculeService
     CLI        ⟶  │  - CalculatePropsUseCase    │  ⟵  FamilyService
     GraphQL    ⟶  │                             │
                    │  📋 Domain Entities         │
                    │  - Molecule                 │
                    │  - MoleculeFamily           │
                    │  - Properties               │
                    │                             │
                    │  🔌 PORTS (Interfaces)     │
                    └─────────────────────────────┘
                           ⟶ ⟶ ⟶ │ ⟵ ⟵ ⟵
                    
     🗄️ Persistence           📊 External Services
     ─────────────            ──────────────────
     PostgreSQL               RDKit Provider
     MongoDB                  ChemAxon API
     InMemory                 PubChem API
```
### SOLID Principles Implementation
#### 🎯 Single Responsibility Principle (SRP)
- **Entities**: Each entity has one reason to change
  - `Molecule`: Chemical structure representation
  - `MoleculeFamily`: Grouping and aggregation logic
  - `Properties`: Property value management
- **Services**: Each service handles one domain concept
- **Use Cases**: Each use case represents one business operation
#### 🔓 Open/Closed Principle (OCP)
- **Strategy Pattern**: Property providers are extensible
- **Polymorphism**: New repository implementations without code changes
- **Events**: Domain events allow extension without modification
#### 🔄 Liskov Substitution Principle (LSP)
- **Port Contracts**: All implementations are interchangeable
- **Value Objects**: Immutable and well-defined contracts
#### 🔌 Interface Segregation Principle (ISP)
- **Focused Ports**: Small, cohesive interfaces
  - `MoleculeReader` vs `MoleculeWriter` (CQRS)
  - `PropertyProvider` vs `PropertyRepository`
- **Client-Specific**: Clients depend only on methods they use
#### ⬇️ Dependency Inversion Principle (DIP)
- **Abstractions**: High-level modules depend on ports, not implementations
- **Injection**: Dependencies injected via constructors
- **Isolation**: Domain layer has no outward dependencies
## 📁 Project Structure
```
src/
├── domain/                 # 🎯 Core Domain Entities
│   ├── entities/
│   │   ├── molecule.rs     # Molecule aggregate root
│   │   ├── family.rs       # MoleculeFamily aggregate
│   │   └── properties.rs   # Property entities
│   ├── value_objects/      # 💎 Immutable Value Objects
│   │   ├── inchikey.rs     # InChIKey validation
│   │   ├── smiles.rs       # SMILES representation
│   │   └── formula.rs      # Molecular formula
│   ├── events/             # 📡 Domain Events
│   │   ├── molecule_events.rs
│   │   └── family_events.rs
│   └── services/           # 📊 Domain Services
│       ├── molecule_service.rs
│       └── family_service.rs
├── application/            # 🎯 Use Cases (Application Services)
│   ├── commands/           # Command operations
│   ├── queries/            # Query operations
│   └── use_cases.rs        # Orchestration logic
├── ports/                  # 🔌 External Interfaces
│   ├── repositories/       # Persistence contracts
│   ├── providers/          # External service contracts
│   └── events/            # Event publishing contracts
└── shared/                 # 🛠️ Shared Utilities
    ├── errors.rs           # Domain error types
    └── traits.rs           # Common traits
```
## 🧩 Core Concepts
### Domain Entities
#### `Molecule`
```rust
use chem_domain::domain::entities::Molecule;
use chem_domain::domain::value_objects::{InChIKey, Smiles};
// Create molecule with value objects
let inchikey = InChIKey::try_from("OTMSDBZUPAUEDD-UHFFFAOYSA-N")?;
let smiles = Smiles::try_from("CC")?;
let molecule = Molecule::new(inchikey, smiles, metadata)?;
```
#### `MoleculeFamily`
```rust
use chem_domain::domain::entities::MoleculeFamily;
// Create family with business rules
let family = MoleculeFamily::builder()
    .name("Alkanes")
    .description("Saturated hydrocarbons")
    .add_molecules(molecules)
    .build()?;
```
### Use Cases (Application Layer)
```rust
use chem_domain::application::use_cases::CreateMoleculeUseCase;
// Dependency injection
let use_case = CreateMoleculeUseCase::new(
    molecule_writer,
    property_provider,
    event_publisher,
);
// Execute use case
let result = use_case.execute(CreateMoleculeCommand {
    smiles: "CCO".to_string(),
    metadata: json!({"source": "user_input"}),
}).await?;
```
### Domain Services
```rust
use chem_domain::domain::services::FamilyService;
// Business logic coordination
let service = FamilyService::new(family_repo, molecule_repo);
let diversity = service.calculate_diversity_metrics(&family_id)?;
```
## 🔌 Ports (Interfaces)
### Repository Ports (Persistence)
```rust
#[async_trait]
pub trait MoleculeRepository: Send + Sync {
    async fn save(&self, molecule: Molecule) -> Result<MoleculeId, DomainError>;
    async fn find_by_id(&self, id: &MoleculeId) -> Result<Option<Molecule>, DomainError>;
    async fn find_by_inchikey(&self, key: &InChIKey) -> Result<Option<Molecule>, DomainError>;
}
```
### Provider Ports (External Services)
```rust
#[async_trait]
pub trait PropertyProvider: Send + Sync {
    async fn calculate_properties(
        &self,
        smiles: &Smiles,
        properties: &[PropertyType],
    ) -> Result<PropertySet, DomainError>;
}
```
### Event Ports (Integration)
```rust
#[async_trait]
pub trait DomainEventPublisher: Send + Sync {
    async fn publish<E: DomainEvent>(&self, event: E) -> Result<(), DomainError>;
}
```
## 🎯 Use Cases Examples
### Command Operations
```rust
// Create molecule from SMILES
let command = CreateMoleculeCommand::new("CCO", metadata);
let molecule_id = create_molecule_use_case.execute(command).await?;
// Add molecule to family
let command = AddMoleculeToFamilyCommand::new(family_id, molecule_id);
add_molecule_use_case.execute(command).await?;
```
### Query Operations
```rust
// Get molecule details
let query = GetMoleculeQuery::by_inchikey("LFQSCWFLJHTTHZ-UHFFFAOYSA-N");
let molecule = get_molecule_use_case.execute(query).await?;
// Search families
let query = SearchFamiliesQuery::by_criteria(criteria);
let families = search_families_use_case.execute(query).await?;
```
## 🧪 Domain Events
### Event-Driven Architecture
```rust
use chem_domain::domain::events::MoleculeCreated;
// Events are automatically published by use cases
#[derive(Debug, Clone)]
pub struct MoleculeCreated {
    pub molecule_id: MoleculeId,
    pub inchikey: InChIKey,
    pub created_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
}
// Event handlers in other bounded contexts can react
impl DomainEventHandler<MoleculeCreated> for PropertyCalculationHandler {
    async fn handle(&self, event: MoleculeCreated) -> Result<(), DomainError> {
        // Calculate properties asynchronously
        self.calculate_basic_properties(event.molecule_id).await
    }
}
```
## ⚡ Value Objects
### Type Safety with Value Objects
```rust
use chem_domain::domain::value_objects::*;
// Compile-time safety
let inchikey = InChIKey::try_from("INVALID")?; // ❌ Validation error
let smiles = Smiles::try_from("CC")?;           // ✅ Valid
let formula = MolecularFormula::try_from("C2H6")?; // ✅ Valid
// No primitive obsession
fn process_molecule(inchikey: InChIKey, smiles: Smiles) {
    // Types guarantee validity
}
```
## 🔄 Error Handling
### Exhaustive Error Types
```rust
use chem_domain::shared::errors::DomainError;
#[derive(Debug, Error)]
pub enum DomainError {
    #[error("Validation failed: {field} - {reason}")]
    ValidationError { field: String, reason: String },
    
    #[error("Entity not found: {entity_type} with id {id}")]
    NotFound { entity_type: String, id: String },
    
    #[error("Business rule violated: {rule} - {context}")]
    BusinessRuleViolation { rule: String, context: String },
    
    #[error("Provider error: {provider} - {details}")]
    ProviderError { provider: String, details: String },
}
```
## 🧪 Testing Strategy
### Unit Tests (Domain)
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chem_domain::testing::builders::*;
    #[test]
    fn molecule_creation_validates_inchikey() {
        let result = Molecule::new(
            InChIKey::try_from("INVALID").unwrap_err(),
            valid_smiles(),
            json!({}),
        );
        
        assert!(matches!(result, Err(DomainError::ValidationError { .. })));
    }
}
```
### Integration Tests (Use Cases)
```rust
#[tokio::test]
async fn create_molecule_use_case_integration() {
    let deps = TestDependencies::new();
    let use_case = CreateMoleculeUseCase::new(
        deps.molecule_repo(),
        deps.property_provider(),
        deps.event_publisher(),
    );
    
    let command = CreateMoleculeCommand::new("CCO", json!({}));
    let result = use_case.execute(command).await;
    
    assert!(result.is_ok());
    deps.verify_molecule_saved().await;
    deps.verify_event_published::<MoleculeCreated>().await;
}
```
## 🚀 Getting Started
### Add to Cargo.toml
```toml
[dependencies]
chem-domain = { path = "../chem-domain" }
serde = { version = "1.0", features = ["derive"] }
tokio = { version = "1.0", features = ["full"] }
```
### Basic Usage
```rust
use chem_domain::prelude::*;
#[tokio::main]
async fn main() -> Result<(), DomainError> {
    // Set up dependencies (would come from DI container)
    let molecule_repo = InMemoryMoleculeRepository::new();
    let property_provider = MockPropertyProvider::new();
    let event_publisher = InMemoryEventPublisher::new();
    
    // Create use case
    let create_molecule = CreateMoleculeUseCase::new(
        molecule_repo,
        property_provider,
        event_publisher,
    );
    
    // Execute business operation
    let command = CreateMoleculeCommand::new("CCO", json!({"name": "Ethanol"}));
    let molecule_id = create_molecule.execute(command).await?;
    
    println!("Created molecule: {}", molecule_id);
    Ok(())
}
```
## 🔧 Configuration
### Feature Flags
```toml
[features]
default = ["async"]
async = ["tokio", "async-trait"]
sync = []
testing = ["mock-providers"]
```
### Environment Setup
```bash
# Development
cargo test --all-features
# Production
cargo build --release --no-default-features --features async
```
## 📊 Metrics & Observability
### Domain Metrics
- Molecule creation rate
- Family aggregation performance
- Property calculation success rate
- Domain event processing latency
### Monitoring Integration
```rust
// Metrics are exposed via domain events
#[derive(Debug)]
pub struct MoleculeProcessed {
    pub processing_time: Duration,
    pub property_count: usize,
    pub success: bool,
}
```
## 🤝 Contributing
### Development Guidelines
1. **Domain Purity**: No external dependencies in domain layer
2. **Test Coverage**: >90% for domain logic
3. **Documentation**: Every public API documented
4. **Performance**: Benchmark critical paths
5. **Type Safety**: Leverage Rust's type system
### Architecture Decisions
- **Immutability**: All entities are immutable
- **Value Objects**: Primitive obsession avoided
- **Events**: Eventually consistent via domain events
- **CQRS**: Command/Query separation in repositories
- **Hexagonal**: Clear port/adapter boundaries
## 📚 Resources
### Architecture References
- [Hexagonal Architecture by Alistair Cockburn](https://alistair.cockburn.us/hexagonal-architecture/)
- [Domain-Driven Design by Eric Evans](https://domainlanguage.com/)
- [Clean Architecture by Robert Martin](https://blog.cleancoder.com/uncle-bob/2012/08/13/the-clean-architecture.html)
### Chemical Informatics
- [InChI Technical Manual](https://www.inchi-trust.org/)
- [SMILES Tutorial](https://www.daylight.com/smiles/)
- [RDKit Documentation](https://www.rdkit.org/docs/)
## 📄 License
This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
---
> 🧬 **Built with pure domain-driven design principles for chemical information management**
