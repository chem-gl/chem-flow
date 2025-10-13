//! Entidades del dominio - Objetos con identidad y ciclo de vida
//!
//! Las entidades representan conceptos centrales del dominio con identidad
//! única y capacidad para cambiar estado a lo largo del tiempo.

use crate::domain::value_objects::*;
use chrono::{DateTime, Utc};
use std::collections::{HashMap, VecDeque};
use uuid::Uuid;

/// Nodo individual en el árbol de flujos
///
/// Representa un punto específico en el árbol donde se almacena contenido.
/// Cada nodo tiene una identidad única y mantiene referencias a su padre e
/// hijos.
#[derive(Debug, Clone, PartialEq)]
pub struct FlowNode {
  /// Identificador único del nodo
  pub id: NodeId,
  /// Referencia al nodo padre (None solo para la raíz)
  pub parent: Option<NodeId>,
  /// Lista de nodos hijos
  pub children: Vec<NodeId>,
  /// Datos del paso almacenados en este nodo
  pub data: FlowData,
  /// Timestamp de creación del nodo
  pub created_at: DateTime<Utc>,
}

impl FlowNode {
  /// Crea un nuevo nodo hijo de un padre específico
  pub fn new_child(parent_id: NodeId, data: FlowData) -> Self {
    Self { id: NodeId::new(), parent: Some(parent_id), children: Vec::new(), data, created_at: Utc::now() }
  }

  /// Crea el nodo raíz del árbol
  pub fn new_root(data: FlowData) -> Self {
    Self { id: NodeId::new(), parent: None, children: Vec::new(), data, created_at: Utc::now() }
  }

  /// Añade un hijo a este nodo
  pub fn add_child(&mut self, child_id: NodeId) {
    if !self.children.contains(&child_id) {
      self.children.push(child_id);
    }
  }

  /// Remueve un hijo de este nodo
  pub fn remove_child(&mut self, child_id: &NodeId) {
    self.children.retain(|id| id != child_id);
  }

  /// Verifica si este nodo es la raíz
  pub fn is_root(&self) -> bool {
    self.parent.is_none()
  }

  /// Verifica si este nodo es una hoja (sin hijos)
  pub fn is_leaf(&self) -> bool {
    self.children.is_empty()
  }
}

/// Rama (Branch) en el árbol de flujos
///
/// Representa una vista lineal del árbol desde la raíz hasta un nodo
/// específico. Las ramas permiten navegación y gestión de secuencias de pasos.
#[derive(Debug, Clone, PartialEq)]
pub struct FlowBranch {
  /// Identificador único de la rama
  pub id: BranchId,
  /// Identificador del flujo al que pertenece
  pub flow_id: Uuid,
  /// Nodo donde termina esta rama (head)
  pub head_node: NodeId,
  /// Nodo donde se ramificó desde el padre
  pub start_node: NodeId,
  /// Rama padre de la que se originó esta rama
  pub parent_branch: Option<BranchId>,
  /// Ramas hijas que se originaron desde esta rama
  pub child_branches: Vec<BranchId>,
  /// Metadatos de la rama
  pub metadata: BranchMetadata,
  /// Timestamp de creación
  pub created_at: DateTime<Utc>,
}

impl FlowBranch {
  /// Crea una nueva rama principal (sin padre)
  pub fn new_main(flow_id: Uuid, head_node: NodeId, metadata: BranchMetadata) -> Self {
    Self { id: BranchId::new(),
           flow_id,
           head_node: head_node.clone(),
           start_node: head_node, // Para rama principal, start = head inicialmente
           parent_branch: None,
           child_branches: Vec::new(),
           metadata,
           created_at: Utc::now() }
  }

  /// Crea una nueva rama derivada de otra rama
  pub fn new_child(flow_id: Uuid, start_node: NodeId, parent_branch: BranchId, metadata: BranchMetadata) -> Self {
    Self { id: BranchId::new(),
           flow_id,
           head_node: start_node.clone(), // Inicia en el punto de ramificación
           start_node,
           parent_branch: Some(parent_branch),
           child_branches: Vec::new(),
           metadata,
           created_at: Utc::now() }
  }

  /// Actualiza el head de la rama a un nuevo nodo
  pub fn advance_head(&mut self, new_head: NodeId) {
    self.head_node = new_head;
  }

  /// Añade una rama hija
  pub fn add_child_branch(&mut self, child_id: BranchId) {
    if !self.child_branches.contains(&child_id) {
      self.child_branches.push(child_id);
    }
  }

  /// Remueve una rama hija
  pub fn remove_child_branch(&mut self, child_id: &BranchId) {
    self.child_branches.retain(|id| id != child_id);
  }

  /// Verifica si es la rama principal
  pub fn is_main_branch(&self) -> bool {
    self.parent_branch.is_none()
  }
}

/// Árbol completo de flujos
///
/// Entidad raíz que encapsula todo el árbol y proporciona operaciones
/// de alto nivel para gestionar nodos y ramas.
#[derive(Debug, Clone)]
pub struct FlowTree {
  /// Identificador único del árbol/flujo
  pub id: Uuid,
  /// Metadatos del flujo completo
  pub metadata: FlowMetadata,
  /// Nodos indexados por ID
  pub nodes: HashMap<NodeId, FlowNode>,
  /// Ramas indexadas por ID
  pub branches: HashMap<BranchId, FlowBranch>,
  /// ID del nodo raíz
  pub root_node: NodeId,
  /// ID de la rama principal
  pub main_branch: BranchId,
  /// Contador de versión para concurrencia optimista
  pub version: i64,
  /// Timestamp de creación
  pub created_at: DateTime<Utc>,
  /// Timestamp de última modificación
  pub updated_at: DateTime<Utc>,
}

impl FlowTree {
  /// Crea un nuevo árbol de flujos con nodo raíz
  pub fn new(metadata: FlowMetadata, root_data: FlowData) -> Self {
    let id = Uuid::new_v4();
    let now = Utc::now();

    // Crear nodo raíz
    let root_node = FlowNode::new_root(root_data);
    let root_id = root_node.id.clone();

    // Crear rama principal
    let branch_metadata = BranchMetadata::new(Some("main".to_string()),
                                              Some("active".to_string()),
                                              serde_json::json!({"type": "main"}));
    let main_branch = FlowBranch::new_main(id, root_id.clone(), branch_metadata);
    let main_branch_id = main_branch.id.clone();

    // Construir mapas
    let mut nodes = HashMap::new();
    nodes.insert(root_id.clone(), root_node);

    let mut branches = HashMap::new();
    branches.insert(main_branch_id.clone(), main_branch);

    Self { id,
           metadata,
           nodes,
           branches,
           root_node: root_id,
           main_branch: main_branch_id,
           version: 0,
           created_at: now,
           updated_at: now }
  }

  /// Obtiene un nodo por ID
  pub fn get_node(&self, node_id: &NodeId) -> Option<&FlowNode> {
    self.nodes.get(node_id)
  }

  /// Obtiene una rama por ID
  pub fn get_branch(&self, branch_id: &BranchId) -> Option<&FlowBranch> {
    self.branches.get(branch_id)
  }

  /// Añade un nuevo nodo al árbol
  pub fn add_node(&mut self, node: FlowNode) {
    self.nodes.insert(node.id.clone(), node);
    self.increment_version();
  }

  /// Añade una nueva rama al árbol
  pub fn add_branch(&mut self, branch: FlowBranch) {
    self.branches.insert(branch.id.clone(), branch);
    self.increment_version();
  }

  /// Remueve un nodo del árbol
  pub fn remove_node(&mut self, node_id: &NodeId) -> Option<FlowNode> {
    let removed = self.nodes.remove(node_id);
    if removed.is_some() {
      self.increment_version();
    }
    removed
  }

  /// Remueve una rama del árbol
  pub fn remove_branch(&mut self, branch_id: &BranchId) -> Option<FlowBranch> {
    let removed = self.branches.remove(branch_id);
    if removed.is_some() {
      self.increment_version();
    }
    removed
  }

  /// Construye el path desde la raíz hasta un nodo específico
  pub fn build_path_to_node(&self, target_node: &NodeId) -> Option<Vec<NodeId>> {
    let mut path = VecDeque::new();
    let mut current = Some(target_node.clone());

    // Recorrer hacia atrás hasta la raíz
    while let Some(node_id) = current {
      path.push_front(node_id.clone());
      current = self.nodes.get(&node_id)?.parent.clone();
    }

    Some(path.into())
  }

  /// Construye el path de una rama específica
  pub fn build_branch_path(&self, branch_id: &BranchId) -> Option<Vec<NodeId>> {
    let branch = self.get_branch(branch_id)?;
    self.build_path_to_node(&branch.head_node)
  }

  /// Verifica si un contenido ya existe en el árbol
  pub fn content_exists(&self, content_hash: &str) -> bool {
    self.nodes.values().any(|node| node.data.get_content_hash() == content_hash)
  }

  /// Incrementa la versión para control de concurrencia
  pub fn increment_version(&mut self) {
    self.version += 1;
    self.updated_at = Utc::now();
  }

  /// Obtiene todos los IDs de nodos
  pub fn node_ids(&self) -> Vec<NodeId> {
    self.nodes.keys().cloned().collect()
  }

  /// Obtiene todos los IDs de ramas
  pub fn branch_ids(&self) -> Vec<BranchId> {
    self.branches.keys().cloned().collect()
  }

  /// Cuenta el número total de nodos
  pub fn node_count(&self) -> usize {
    self.nodes.len()
  }

  /// Cuenta el número total de ramas
  pub fn branch_count(&self) -> usize {
    self.branches.len()
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  fn create_test_flow_data(content: &str) -> FlowData {
    FlowData::new(Uuid::new_v4(), 1, "test_key", json!({"content": content}), json!({}), None)
  }

  #[test]
  fn test_flow_node_creation() {
    let data = create_test_flow_data("test content");
    let root = FlowNode::new_root(data.clone());

    assert!(root.is_root());
    assert!(root.is_leaf());
    assert_eq!(root.parent, None);
    assert_eq!(root.children.len(), 0);
  }

  #[test]
  fn test_flow_node_child_operations() {
    let root_data = create_test_flow_data("root");
    let mut root = FlowNode::new_root(root_data);

    let child_data = create_test_flow_data("child");
    let child = FlowNode::new_child(root.id.clone(), child_data);
    let child_id = child.id.clone();

    root.add_child(child_id.clone());

    assert!(!root.is_leaf());
    assert_eq!(root.children.len(), 1);
    assert_eq!(root.children[0], child_id);
    assert_eq!(child.parent, Some(root.id.clone()));
  }

  #[test]
  fn test_flow_branch_creation() {
    let flow_id = Uuid::new_v4();
    let node_id = NodeId::new();
    let metadata = BranchMetadata::new(Some("test_branch".to_string()), Some("active".to_string()), json!({}));

    let branch = FlowBranch::new_main(flow_id, node_id.clone(), metadata);

    assert!(branch.is_main_branch());
    assert_eq!(branch.flow_id, flow_id);
    assert_eq!(branch.head_node, node_id);
    assert_eq!(branch.start_node, node_id);
  }

  #[test]
  fn test_flow_tree_creation() {
    let metadata = FlowMetadata::new(Some("test_flow".to_string()),
                                     Some("active".to_string()),
                                     Some("test_user".to_string()),
                                     None,
                                     None,
                                     json!({}));

    let root_data = create_test_flow_data("root content");
    let tree = FlowTree::new(metadata, root_data);

    assert_eq!(tree.node_count(), 1);
    assert_eq!(tree.branch_count(), 1);
    assert!(tree.get_node(&tree.root_node).is_some());
    assert!(tree.get_branch(&tree.main_branch).is_some());
  }

  #[test]
  fn test_flow_tree_path_building() {
    let metadata = FlowMetadata::new(Some("test_flow".to_string()),
                                     Some("active".to_string()),
                                     Some("test_user".to_string()),
                                     None,
                                     None,
                                     json!({}));

    let root_data = create_test_flow_data("root");
    let mut tree = FlowTree::new(metadata, root_data);

    // Añadir un nodo hijo
    let child_data = create_test_flow_data("child");
    let child_node = FlowNode::new_child(tree.root_node.clone(), child_data);
    let child_id = child_node.id.clone();

    tree.add_node(child_node);

    // Actualizar las referencias padre-hijo
    if let Some(root) = tree.nodes.get_mut(&tree.root_node) {
      root.add_child(child_id.clone());
    }

    // Construir path
    let path = tree.build_path_to_node(&child_id).unwrap();
    assert_eq!(path.len(), 2);
    assert_eq!(path[0], tree.root_node);
    assert_eq!(path[1], child_id);
  }

  #[test]
  fn test_content_uniqueness_check() {
    let metadata = FlowMetadata::new(Some("test_flow".to_string()),
                                     Some("active".to_string()),
                                     Some("test_user".to_string()),
                                     None,
                                     None,
                                     json!({}));

    let root_data = create_test_flow_data("unique content");
    let tree = FlowTree::new(metadata, root_data.clone());

    // El contenido del nodo raíz debería existir
    assert!(tree.content_exists(&root_data.get_content_hash()));

    // Un contenido diferente no debería existir
    let different_data = create_test_flow_data("different content");
    assert!(!tree.content_exists(&different_data.get_content_hash()));
  }
}
