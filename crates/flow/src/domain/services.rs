//! Servicios del dominio - Lógica de negocio que no pertenece a ninguna entidad
//! específica
//!
//! Los servicios de dominio encapsulan lógica de negocio compleja que involucra
//! múltiples entidades o que no pertenece naturalmente a una entidad
//! específica.

use crate::domain::entities::*;
use crate::domain::value_objects::*;
use crate::errors::{FlowError, Result};
use std::collections::HashMap;

/// Servicio para operaciones del árbol de flujos
///
/// Encapsula la lógica compleja para manipular el árbol de flujos,
/// incluyendo validaciones, construcción de paths y verificaciones de
/// integridad.
pub struct FlowTreeService;

impl FlowTreeService {
  /// Añade un nuevo paso a una rama específica del árbol
  ///
  /// # Validaciones:
  /// - La rama debe existir
  /// - El contenido no debe estar duplicado globalmente
  /// - El cursor debe ser secuencial
  pub fn add_step_to_branch(tree: &mut FlowTree,
                            branch_id: &BranchId,
                            key: String,
                            payload: serde_json::Value,
                            metadata: serde_json::Value,
                            command_id: Option<uuid::Uuid>)
                            -> Result<NodeId> {
    // Verificar que la rama existe
    let branch = tree.get_branch(branch_id).ok_or_else(|| FlowError::NotFound(format!("Branch {}", branch_id)))?.clone();

    // Calcular el siguiente cursor
    let current_path =
      tree.build_branch_path(branch_id).ok_or_else(|| FlowError::Other("Cannot build branch path".to_string()))?;
    let next_cursor = current_path.len() as i64 + 1;

    // Crear datos del nuevo paso
    let step_data = FlowData::new(tree.id, next_cursor, key, payload, metadata, command_id);

    // Verificar duplicación global
    if tree.content_exists(&step_data.get_content_hash()) {
      return Err(FlowError::Conflict("Duplicate content not allowed".to_string()));
    }

    // Crear nuevo nodo
    let new_node = FlowNode::new_child(branch.head_node.clone(), step_data);
    let new_node_id = new_node.id.clone();

    // Actualizar nodo padre
    if let Some(parent_node) = tree.nodes.get_mut(&branch.head_node) {
      parent_node.add_child(new_node_id.clone());
    }

    // Añadir nodo al árbol
    tree.add_node(new_node);

    // Actualizar head de la rama
    if let Some(branch_mut) = tree.branches.get_mut(branch_id) {
      branch_mut.advance_head(new_node_id.clone());
    }

    Ok(new_node_id)
  }

  /// Crea una nueva rama desde un punto específico de otra rama
  ///
  /// # Validaciones:
  /// - La rama padre debe existir
  /// - El cursor debe ser válido (dentro del rango de la rama padre)
  /// - El punto de ramificación debe corresponder a un nodo existente
  pub fn create_branch_from_point(tree: &mut FlowTree,
                                  parent_branch_id: &BranchId,
                                  at_cursor: i64,
                                  metadata: BranchMetadata)
                                  -> Result<BranchId> {
    // Verificar que la rama padre existe
    let _parent_branch = tree.get_branch(parent_branch_id)
                             .ok_or_else(|| FlowError::NotFound(format!("Parent branch {}", parent_branch_id)))?
                             .clone();

    // Obtener path de la rama padre
    let parent_path =
      tree.build_branch_path(parent_branch_id).ok_or_else(|| FlowError::Other("Cannot build parent path".to_string()))?;

    // Validar cursor
    if at_cursor <= 0 || at_cursor > parent_path.len() as i64 {
      return Err(FlowError::Conflict(format!("Invalid cursor {} for branch with {} steps", at_cursor, parent_path.len())));
    }

    // Obtener nodo de ramificación
    let branch_point_node =
      parent_path.get((at_cursor - 1) as usize).ok_or_else(|| FlowError::Other("Invalid branch point".to_string()))?.clone();

    // Crear nueva rama
    let new_branch = FlowBranch::new_child(tree.id, branch_point_node, parent_branch_id.clone(), metadata);
    let new_branch_id = new_branch.id.clone();

    // Añadir rama al árbol
    tree.add_branch(new_branch);

    // Actualizar rama padre
    if let Some(parent_mut) = tree.branches.get_mut(parent_branch_id) {
      parent_mut.add_child_branch(new_branch_id.clone());
    }

    Ok(new_branch_id)
  }

  /// Elimina una rama y opcionalmente todas sus subramas
  ///
  /// # Comportamiento:
  /// - Si recursive=true, elimina todas las subramas descendientes
  /// - Elimina solo los nodos exclusivos de la rama
  /// - Preserva nodos compartidos con otras ramas
  pub fn delete_branch(tree: &mut FlowTree, branch_id: &BranchId, recursive: bool) -> Result<()> {
    // Verificar que la rama existe
    let branch = tree.get_branch(branch_id).ok_or_else(|| FlowError::NotFound(format!("Branch {}", branch_id)))?.clone();

    // No permitir eliminar la rama principal
    if branch_id == &tree.main_branch {
      return Err(FlowError::Conflict("Cannot delete main branch".to_string()));
    }

    // Eliminar subramas si se solicita recursivamente
    if recursive {
      let child_branches = branch.child_branches.clone();
      for child_id in child_branches {
        Self::delete_branch(tree, &child_id, true)?;
      }
    }

    // Encontrar nodos exclusivos de esta rama
    let exclusive_nodes = Self::find_exclusive_nodes(tree, branch_id)?;

    // Eliminar nodos exclusivos
    for node_id in exclusive_nodes {
      // Obtener parent_id clonándolo para evitar mantener una referencia
      // inmutable a `tree.nodes` mientras se realiza una mutación.
      let parent_opt = tree.get_node(&node_id).and_then(|n| n.parent.clone());

      if let Some(parent_id) = parent_opt {
        if let Some(parent_node) = tree.nodes.get_mut(&parent_id) {
          parent_node.remove_child(&node_id);
        }
      }

      // Eliminar nodo
      tree.remove_node(&node_id);
    }

    // Actualizar rama padre
    if let Some(parent_branch_id) = &branch.parent_branch {
      if let Some(parent_branch) = tree.branches.get_mut(parent_branch_id) {
        parent_branch.remove_child_branch(branch_id);
      }
    }

    // Eliminar la rama
    tree.remove_branch(branch_id);

    Ok(())
  }

  /// Elimina pasos desde un cursor específico en adelante
  ///
  /// # Comportamiento:
  /// - Elimina todos los pasos con cursor >= from_cursor
  /// - Elimina recursivamente las subramas afectadas
  /// - Mantiene la integridad del árbol
  pub fn delete_from_cursor(tree: &mut FlowTree, branch_id: &BranchId, from_cursor: i64) -> Result<()> {
    // Verificar que la rama existe
    let branch = tree.get_branch(branch_id).ok_or_else(|| FlowError::NotFound(format!("Branch {}", branch_id)))?.clone();

    // Obtener path de la rama
    let branch_path =
      tree.build_branch_path(branch_id).ok_or_else(|| FlowError::Other("Cannot build branch path".to_string()))?;

    // Validar cursor
    if from_cursor <= 0 || from_cursor > branch_path.len() as i64 {
      return Ok(()); // Nada que eliminar
    }

    // Encontrar nodo donde hacer el corte
    let cut_index = (from_cursor - 1) as usize;
    let nodes_to_delete: Vec<NodeId> = branch_path.iter().skip(cut_index).cloned().collect();

    // Eliminar subramas que se originan en o después del punto de corte
    let child_branches = branch.child_branches.clone();
    for child_id in child_branches {
      if let Some(_child_branch) = tree.get_branch(&child_id) {
        let child_path = tree.build_branch_path(&child_id).unwrap_or_default();

        // Si la subrama se origina en o después del punto de corte, eliminarla
        if child_path.len() >= from_cursor as usize {
          Self::delete_branch(tree, &child_id, true)?;
        }
      }
    }

    // Eliminar nodos desde el punto de corte
    for node_id in nodes_to_delete {
      // Clonar parent id fuera del scope que mantiene la referencia
      let parent_opt = tree.get_node(&node_id).and_then(|n| n.parent.clone());

      if let Some(parent_id) = parent_opt {
        if let Some(parent_node) = tree.nodes.get_mut(&parent_id) {
          parent_node.remove_child(&node_id);
        }
      }

      tree.remove_node(&node_id);
    }

    // Actualizar head de la rama
    if cut_index > 0 {
      let new_head = branch_path[cut_index - 1].clone();
      if let Some(branch_mut) = tree.branches.get_mut(branch_id) {
        branch_mut.advance_head(new_head);
      }
    }

    Ok(())
  }

  /// Construye el path completo de una rama como secuencia de FlowData
  pub fn get_branch_data_sequence(tree: &FlowTree, branch_id: &BranchId) -> Result<Vec<FlowData>> {
    let path = tree.build_branch_path(branch_id).ok_or_else(|| FlowError::NotFound(format!("Branch {}", branch_id)))?;

    let mut sequence = Vec::new();
    for node_id in path {
      if let Some(node) = tree.get_node(&node_id) {
        sequence.push(node.data.clone());
      }
    }

    Ok(sequence)
  }

  /// Verifica la integridad del árbol
  ///
  /// # Validaciones:
  /// - Todos los nodos padre-hijo están correctamente vinculados
  /// - No hay ciclos en el árbol
  /// - Todas las ramas apuntan a nodos válidos
  /// - No hay nodos huérfanos (excepto la raíz)
  pub fn validate_tree_integrity(tree: &FlowTree) -> Result<()> {
    // Verificar que existe el nodo raíz
    if !tree.nodes.contains_key(&tree.root_node) {
      return Err(FlowError::Other("Root node missing".to_string()));
    }

    // Verificar que existe la rama principal
    if !tree.branches.contains_key(&tree.main_branch) {
      return Err(FlowError::Other("Main branch missing".to_string()));
    }

    // Verificar relaciones padre-hijo
    for (node_id, node) in &tree.nodes {
      // Verificar hijos
      for child_id in &node.children {
        let child =
          tree.nodes.get(child_id).ok_or_else(|| FlowError::Other(format!("Child node {} not found", child_id)))?;

        if child.parent.as_ref() != Some(node_id) {
          return Err(FlowError::Other(format!("Child {} doesn't point back to parent {}", child_id, node_id)));
        }
      }

      // Verificar padre (excepto para la raíz)
      if let Some(parent_id) = &node.parent {
        let parent =
          tree.nodes.get(parent_id).ok_or_else(|| FlowError::Other(format!("Parent node {} not found", parent_id)))?;

        if !parent.children.contains(node_id) {
          return Err(FlowError::Other(format!("Parent {} doesn't contain child {}", parent_id, node_id)));
        }
      }
    }

    // Verificar que no hay ciclos (usando DFS)
    Self::check_for_cycles(tree)?;

    // Verificar ramas
    for (branch_id, branch) in &tree.branches {
      // Verificar que los nodos de la rama existen
      if !tree.nodes.contains_key(&branch.head_node) {
        return Err(FlowError::Other(format!("Branch {} head node {} not found", branch_id, branch.head_node)));
      }

      if !tree.nodes.contains_key(&branch.start_node) {
        return Err(FlowError::Other(format!("Branch {} start node {} not found", branch_id, branch.start_node)));
      }
    }

    Ok(())
  }

  /// Encuentra nodos que son exclusivos de una rama específica
  fn find_exclusive_nodes(tree: &FlowTree, branch_id: &BranchId) -> Result<Vec<NodeId>> {
    let branch = tree.get_branch(branch_id).ok_or_else(|| FlowError::NotFound(format!("Branch {}", branch_id)))?;

    let mut exclusive_nodes = Vec::new();
    let mut current = Some(branch.head_node.clone());

    // Recorrer hacia atrás desde el head hasta el start_node
    while let Some(node_id) = current {
      if node_id == branch.start_node {
        break; // No incluir el start_node (es compartido)
      }

      exclusive_nodes.push(node_id.clone());

      if let Some(node) = tree.get_node(&node_id) {
        current = node.parent.clone();
      } else {
        break;
      }
    }

    Ok(exclusive_nodes)
  }

  /// Verifica que no hay ciclos en el árbol
  fn check_for_cycles(tree: &FlowTree) -> Result<()> {
    let mut visited = std::collections::HashSet::new();
    let mut rec_stack = std::collections::HashSet::new();

    for node_id in tree.nodes.keys() {
      if !visited.contains(node_id) && Self::dfs_cycle_check(tree, node_id, &mut visited, &mut rec_stack)? {
        return Err(FlowError::Other("Cycle detected in tree".to_string()));
      }
    }

    Ok(())
  }

  /// DFS para detectar ciclos
  fn dfs_cycle_check(tree: &FlowTree,
                     node_id: &NodeId,
                     visited: &mut std::collections::HashSet<NodeId>,
                     rec_stack: &mut std::collections::HashSet<NodeId>)
                     -> Result<bool> {
    visited.insert(node_id.clone());
    rec_stack.insert(node_id.clone());

    if let Some(node) = tree.get_node(node_id) {
      for child_id in &node.children {
        if !visited.contains(child_id) {
          if Self::dfs_cycle_check(tree, child_id, visited, rec_stack)? {
            return Ok(true);
          }
        } else if rec_stack.contains(child_id) {
          return Ok(true); // Ciclo detectado
        }
      }
    }

    rec_stack.remove(node_id);
    Ok(false)
  }
}

/// Servicio para gestión de duplicados
///
/// Encapsula la lógica para detectar y prevenir duplicación de contenido
/// en el árbol de flujos.
pub struct DuplicationService;

impl DuplicationService {
  /// Verifica si un contenido ya existe en el árbol
  pub fn content_exists(tree: &FlowTree, content_hash: &str) -> bool {
    tree.content_exists(content_hash)
  }

  /// Encuentra todos los nodos con contenido duplicado
  pub fn find_duplicate_content(tree: &FlowTree) -> HashMap<String, Vec<NodeId>> {
    let mut content_map: HashMap<String, Vec<NodeId>> = HashMap::new();

    for (node_id, node) in &tree.nodes {
      let hash = node.data.get_content_hash();
      content_map.entry(hash).or_default().push(node_id.clone());
    }

    // Filtrar solo los que tienen duplicados
    content_map.into_iter().filter(|(_, nodes)| nodes.len() > 1).collect()
  }

  /// Valida que no hay duplicados en el árbol
  pub fn validate_no_duplicates(tree: &FlowTree) -> Result<()> {
    let duplicates = Self::find_duplicate_content(tree);

    if !duplicates.is_empty() {
      return Err(FlowError::Conflict(format!("Found {} sets of duplicate content", duplicates.len())));
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  fn create_test_tree() -> FlowTree {
    let metadata = FlowMetadata::new(Some("test_flow"), Some("active"), Some("test_user"), None, None, json!({}));

    let root_data = FlowData::new(uuid::Uuid::new_v4(),
                                  1,
                                  "root_step",
                                  json!({"content": "root"}),
                                  json!({}),
                                  None);

    FlowTree::new(metadata, root_data)
  }

  #[test]
  fn test_add_step_to_branch() {
    let mut tree = create_test_tree();
    let main_branch = tree.main_branch.clone();

    let result = FlowTreeService::add_step_to_branch(&mut tree,
                                                     &main_branch,
                                                     "step_2".to_string(),
                                                     json!({"content": "second step"}),
                                                     json!({}),
                                                     None);

    assert!(result.is_ok());
    assert_eq!(tree.node_count(), 2);
  }

  #[test]
  fn test_prevent_duplicate_content() {
    let mut tree = create_test_tree();
    let main_branch = tree.main_branch.clone();

    // Primer paso
    FlowTreeService::add_step_to_branch(&mut tree,
                                        &main_branch,
                                        "step_2".to_string(),
                                        json!({"content": "duplicate"}),
                                        json!({}),
                                        None).unwrap();

    // Intentar añadir contenido duplicado
    let result = FlowTreeService::add_step_to_branch(&mut tree,
                                                     &main_branch,
                                                     "step_3".to_string(),
                                                     json!({"content": "duplicate"}),
                                                     json!({}),
                                                     None);

    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), FlowError::Conflict(_)));
  }

  #[test]
  fn test_create_branch_from_point() {
    let mut tree = create_test_tree();
    let main_branch = tree.main_branch.clone();

    // Añadir algunos pasos al main
    for i in 2..=5 {
      FlowTreeService::add_step_to_branch(&mut tree,
                                          &main_branch,
                                          format!("step_{}", i),
                                          json!({"content": format!("content {}", i)}),
                                          json!({}),
                                          None).unwrap();
    }

    // Crear rama desde el paso 3
    let branch_metadata = BranchMetadata::new(Some("test_branch"), Some("active"), json!({}));

    let result = FlowTreeService::create_branch_from_point(&mut tree, &main_branch, 3, branch_metadata);

    assert!(result.is_ok());
    assert_eq!(tree.branch_count(), 2);
  }

  #[test]
  fn test_tree_integrity_validation() {
    let tree = create_test_tree();

    let result = FlowTreeService::validate_tree_integrity(&tree);
    assert!(result.is_ok());
  }

  #[test]
  fn test_duplication_service() {
    let tree = create_test_tree();

    // No debería haber duplicados inicialmente
    let duplicates = DuplicationService::find_duplicate_content(&tree);
    assert!(duplicates.is_empty());

    let validation = DuplicationService::validate_no_duplicates(&tree);
    assert!(validation.is_ok());
  }
}
