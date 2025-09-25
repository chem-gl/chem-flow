use chem_domain::DomainRepository;
use chem_domain::{DomainError, Molecule, MoleculeFamily, OwnedMolecularProperty};
use chem_persistence::new_domain_from_env;
use serde_json::json;
use serde_json::Value as JsonValue;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::io::{self, Write};
use std::sync::Arc;
use uuid::Uuid;
fn main() -> Result<(), Box<dyn Error>> {
  // Create repository using crate-provided helper that chooses and validates
  // the backend (Postgres vs Sqlite) based on build features and
  // environment variables. Abort on any error because the demo requires a
  // persistent DB.
  let repo: Arc<dyn chem_domain::DomainRepository> = match new_domain_from_env() {
    Ok(d) => Arc::new(d),
    Err(e) => {
      eprintln!("Error creando repositorio desde CHEM_DB_URL: {:?}. El demo requiere DB.", e);
      std::process::exit(1);
    }
  };
  loop {
    print_menu();
    let choice = prompt("Seleccione una opción: ")?;
    match choice.trim() {
      "1" => create_from_parts(repo.as_ref())?,
      "2" => create_from_smiles(repo.as_ref())?,
      "3" => add_property_interactive(repo.as_ref())?,
      "4" => create_family_from_one(repo.as_ref())?,
      "5" => add_molecule_to_family(repo.as_ref())?,
      "6" => show_molecule_and_props(repo.as_ref())?,
      "7" => list_families_show_molecules(repo.as_ref())?,
      "8" => show_family(repo.as_ref())?,
      "9" => remove_molecule_from_family(repo.as_ref())?,
      "10" => delete_molecule(repo.as_ref())?,
      "11" => delete_family(repo.as_ref())?,
      "12" => list_all_molecules(repo.as_ref())?,
      "q" | "Q" => {
        println!("Saliendo...");
        break;
      }
      other => println!("Opción inválida: {}", other),
    }
  }
  Ok(())
}
fn print_menu() {
  println!("\n=== Ejemplo: chem-domain (dominio) ===");
  println!("1) Crear molécula desde partes (inchikey, smiles, inchi)");
  println!("2) Crear molécula desde SMILES (usa el motor químico si está disponible)");
  println!("3) Agregar propiedad molecular a una molécula existente");
  println!("4) Crear familia a partir de UNA molécula existente (familia no puede estar vacía)");
  println!("5) Agregar molécula a una familia existente (crea nueva versión de la familia)");
  println!("6) Mostrar molécula y propiedades");
  println!("7) Listar familias (solo mostrar moléculas)");
  println!("8) Mostrar familia por ID (detallada)");
  println!("9) Quitar molécula de una familia (crea nueva versión)");
  println!("10) Eliminar molécula (fallará si pertenece a alguna familia)");
  println!("11) Eliminar familia (borra propiedades y mappings)");
  println!("12) Listar todas las moléculas (para selección en otras opciones)");
  println!("q) Salir");
}
fn prompt(msg: &str) -> io::Result<String> {
  print!("{}", msg);
  io::stdout().flush()?;
  let mut s = String::new();
  io::stdin().read_line(&mut s)?;
  Ok(s.trim_end().to_string())
}
fn list_all_molecules(repo: &dyn chem_domain::DomainRepository) -> Result<(), Box<dyn Error>> {
  match repo.list_molecules() {
    Ok(mols) => {
      println!("Moléculas encontradas: {}", mols.len());
      for (i, m) in mols.iter().enumerate() {
        println!("{}: InChIKey={} SMILES={} metadata={}",
                 i + 1,
                 m.inchikey(),
                 m.smiles(),
                 m.metadata());
      }
    }
    Err(e) => println!("Error listando moléculas: {:?}", e),
  }
  Ok(())
}

// Helper: devuelve valores mock por método y propiedad (usado en el ejemplo)
fn mock_value_for_method(method: &str, prop: &str) -> Option<f64> {
  match method {
    "Random1" => match prop {
      "LogP" => Some(2.5),
      "PSA" => Some(45.0),
      "AtX" => Some(24.0),
      "HBA" => Some(3.0),
      "HBD" => Some(1.0),
      "RB" => Some(5.0),
      "MR" => Some(60.0),
      _ => None,
    },
    "Random2" => match prop {
      "LD50" => Some(350.0),
      "Mutagenicity" => Some(0.0),
      "DevelopmentalToxicity" => Some(0.0),
      "SyntheticAccessibility" => Some(3.2),
      _ => None,
    },
    "Random3" => match prop {
      "HBD" => Some(2.0),
      "RB" => Some(3.0),
      "MR" => Some(72.0),
      "LD50" => Some(250.0),
      "Mutagenicity" => Some(1.0),
      _ => None,
    },
    _ => None,
  }
}
fn create_from_parts(repo: &dyn chem_domain::DomainRepository) -> Result<(), Box<dyn Error>> {
  let inchikey = prompt("InChIKey: ")?;
  let smiles = prompt("SMILES: ")?;
  let inchi = prompt("InChI: ")?;
  let meta_s = prompt("Metadatos (JSON, opcional, enter para vacio): ")?;
  let metadata: JsonValue =
    if meta_s.trim().is_empty() { json!({}) } else { serde_json::from_str(&meta_s).unwrap_or(json!({"raw": meta_s})) };
  match Molecule::from_parts(&inchikey, &smiles, &inchi, metadata) {
    Ok(m) => match repo.save_molecule(m.clone()) {
      Ok(key) => println!("Molécula guardada con InChIKey={}", key),
      Err(e) => println!("Error guardando molécula: {:?}", e),
    },
    Err(e) => println!("Error creando molécula: {:?}", e),
  }
  Ok(())
}
fn create_from_smiles(repo: &dyn chem_domain::DomainRepository) -> Result<(), Box<dyn Error>> {
  let smiles = prompt("SMILES: ")?;
  println!("Creando molécula desde SMILES (puede fallar si no hay motor químico)...");
  match Molecule::from_smiles(&smiles) {
    Ok(m) => match repo.save_molecule(m.clone()) {
      Ok(key) => println!("Molécula generada y guardada con InChIKey={}", key),
      Err(e) => println!("Falló al guardar: {:?}", e),
    },
    Err(e) => match e {
      DomainError::ValidationError(msg) => println!("SMILES inválido/validación: {}", msg),
      DomainError::ExternalError(msg) => println!("Error externo (motor químico no disponible?): {}", msg),
      other => println!("Error al crear molécula: {:?}", other),
    },
  }
  Ok(())
}
fn add_property_interactive(repo: &dyn chem_domain::DomainRepository) -> Result<(), Box<dyn Error>> {
  let inchikey = prompt("InChIKey de la molécula a la que agregar la propiedad: ")?;
  match repo.get_molecule(&inchikey) {
    Ok(Some(_m)) => {
      // Nuevo flujo interactivo: elegir modo de entrada
      println!("Modo de entrada para propiedades:");
      println!("1) Ingresar UNA propiedad (modo clásico)");
      println!("2) Generar propiedades por MÉTODO mock (Random1/Random2/Random3/Random4)");
      println!("3) Ingresar MANUALMENTE valores para TODAS las propiedades requeridas");
      let mode = prompt("Seleccione modo (1/2/3): ")?;

      // listado de propiedades (coincide con ADMETSA)
      let properties = vec!["LogP",
                            "PSA",
                            "AtX",
                            "HBA",
                            "HBD",
                            "RB",
                            "MR",
                            "LD50",
                            "Mutagenicity",
                            "DevelopmentalToxicity",
                            "SyntheticAccessibility"];

      match mode.trim() {
        "1" => {
          // Modo clásico: una propiedad
          let prop_type = prompt("Tipo de propiedad (ej. LogP): ")?;
          let val_s = prompt("Valor (si es JSON puede ingresarlo, ej. 1.23 o \"text\" o { \"a\":1 }): ")?;
          let value: JsonValue = serde_json::from_str(&val_s).unwrap_or(json!(val_s));
          let quality = prompt("Quality (opcional): ")?;
          let quality_opt = if quality.trim().is_empty() { None } else { Some(quality) };
          let pref = prompt("Preferred? (y/N): ")?;
          let preferred = matches!(pref.trim().to_lowercase().as_str(), "y" | "yes");
          let meta_s = prompt("Metadatos para la propiedad (JSON, opcional): ")?;
          let metadata: JsonValue = if meta_s.trim().is_empty() {
            json!({})
          } else {
            serde_json::from_str(&meta_s).unwrap_or(json!({"raw": meta_s}))
          };
          // compute simple value_hash using sha256
          let value_raw = value.to_string();
          let mut hasher = Sha256::new();
          hasher.update(value_raw.as_bytes());
          let value_hash = format!("{:x}", hasher.finalize());
          let prop = OwnedMolecularProperty { id: Uuid::new_v4(),
                                              molecule_inchikey: inchikey.clone(),
                                              property_type: prop_type,
                                              value,
                                              quality: quality_opt,
                                              preferred,
                                              value_hash,
                                              metadata };
          match repo.save_molecular_property(prop) {
            Ok(id) => println!("Propiedad guardada con id={}", id),
            Err(e) => println!("Error guardando propiedad: {:?}", e),
          }
        }
        "2" => {
          // Generar por método mock
          println!("Métodos disponibles: 1=Random1 2=Random2 3=Random3 4=Random4");
          let m = prompt("Seleccione método (1-4): ")?;
          let method_name = match m.trim() {
            "1" => "Random1",
            "2" => "Random2",
            "3" => "Random3",
            "4" => "Random4",
            other => {
              println!("Opción inválida: {}", other);
              return Ok(());
            }
          };

          // Generar y guardar todas las propiedades que el método mock produce
          for &prop in properties.iter() {
            // calcular mock
            let val = match mock_value_for_method(method_name, prop) {
              Some(v) => json!(v),
              None => continue, // este método no genera esa propiedad
            };
            let value_raw = val.to_string();
            let mut hasher = Sha256::new();
            hasher.update(value_raw.as_bytes());
            let value_hash = format!("{:x}", hasher.finalize());
            let meta = json!({"method": method_name});
            let prop_obj = OwnedMolecularProperty { id: Uuid::new_v4(),
                                                    molecule_inchikey: inchikey.clone(),
                                                    property_type: prop.to_string(),
                                                    value: val,
                                                    quality: Some("calculated".to_string()),
                                                    preferred: true,
                                                    value_hash,
                                                    metadata: meta };
            match repo.save_molecular_property(prop_obj) {
              Ok(id) => println!("Guardada {} -> id={}", prop, id),
              Err(e) => println!("Error guardando {}: {:?}", prop, e),
            }
          }
        }
        "3" => {
          // Manual: pedir valor para cada propiedad en un for
          println!("Ingresar valores manuales para todas las propiedades (enter para omitir una)");
          for &prop in properties.iter() {
            let prompt_msg = format!("Valor para {} (enter para omitir): ", prop);
            let v = prompt(&prompt_msg)?;
            if v.trim().is_empty() {
              continue;
            }
            // tratar de parsear JSON; si falla usar string
            let value: JsonValue = serde_json::from_str(&v).unwrap_or(json!(v));
            let mut hasher = Sha256::new();
            hasher.update(value.to_string().as_bytes());
            let value_hash = format!("{:x}", hasher.finalize());
            let meta = json!({"method": "Manual"});
            let prop_obj = OwnedMolecularProperty { id: Uuid::new_v4(),
                                                    molecule_inchikey: inchikey.clone(),
                                                    property_type: prop.to_string(),
                                                    value,
                                                    quality: Some("manual".to_string()),
                                                    preferred: true,
                                                    value_hash,
                                                    metadata: meta };
            match repo.save_molecular_property(prop_obj) {
              Ok(id) => println!("Guardada {} -> id={}", prop, id),
              Err(e) => println!("Error guardando {}: {:?}", prop, e),
            }
          }
        }
        other => println!("Modo desconocido: {}", other),
      }
    }
    Ok(None) => println!("No existe la molécula con InChIKey={}", inchikey),
    Err(e) => println!("Error accediendo al repo: {:?}", e),
  }
  Ok(())
}
fn show_molecule_and_props(repo: &dyn DomainRepository) -> Result<(), Box<dyn Error>> {
  let inchikey = prompt("InChIKey: ")?;
  match repo.get_molecule(&inchikey) {
    Ok(Some(m)) => {
      println!("Molécula: InChIKey={} SMILES={} InChI={} metadata={}",
               m.inchikey(),
               m.smiles(),
               m.inchi(),
               m.metadata());
      match repo.get_molecular_properties(&inchikey) {
        Ok(props) => {
          println!("Propiedades ({}):", props.len());
          for p in props {
            println!("- id={} type={} value={} quality={:?} preferred={} metadata={}",
                     p.id, p.property_type, p.value, p.quality, p.preferred, p.metadata);
          }
        }
        Err(e) => println!("Error obteniendo propiedades: {:?}", e),
      }
    }
    Ok(None) => println!("No encontrada"),
    Err(e) => println!("Error: {:?}", e),
  }
  Ok(())
}
fn create_family_from_one(repo: &dyn chem_domain::DomainRepository) -> Result<(), Box<dyn Error>> {
  let inchikey = prompt("InChIKey de la molécula inicial para la familia: ")?;
  let meta_s = prompt("Metadatos de la familia (JSON, opcional): ")?;
  let metadata: JsonValue =
    if meta_s.trim().is_empty() { json!({}) } else { serde_json::from_str(&meta_s).unwrap_or(json!({"raw": meta_s})) };
  match repo.get_molecule(&inchikey) {
    Ok(Some(m)) => match MoleculeFamily::new(vec![m.clone()], metadata) {
      Ok(fam) => match repo.save_family(fam.clone()) {
        Ok(id) => println!("Familia creada con id={}", id),
        Err(e) => println!("Error guardando familia: {:?}", e),
      },
      Err(e) => println!("Error creando familia: {:?}", e),
    },
    Ok(None) => println!("No existe la molécula con InChIKey={}", inchikey),
    Err(e) => println!("Error accediendo al repo: {:?}", e),
  }
  Ok(())
}
fn add_molecule_to_family(repo: &dyn chem_domain::DomainRepository) -> Result<(), Box<dyn Error>> {
  let fam_id_s = prompt("ID de la familia a la que agregar la molécula: ")?;
  let inchikey = prompt("InChIKey de la molécula a agregar: ")?;
  let fam_id = match Uuid::parse_str(&fam_id_s) {
    Ok(u) => u,
    Err(_) => {
      println!("ID de familia inválido");
      return Ok(());
    }
  };
  match repo.get_family(&fam_id) {
    Ok(Some(fam)) => match repo.get_molecule(&inchikey) {
      Ok(Some(m)) => match fam.add_molecule(m) {
        Ok(new_fam) => match repo.save_family(new_fam.clone()) {
          Ok(new_id) => println!("Nueva versión de familia guardada con id={}", new_id),
          Err(e) => println!("Error guardando nueva familia: {:?}", e),
        },
        Err(e) => println!("No se puede agregar la molécula a la familia: {:?}", e),
      },
      Ok(None) => println!("Molécula no encontrada: {}", inchikey),
      Err(e) => println!("Error al buscar molécula: {:?}", e),
    },
    Ok(None) => println!("Familia no encontrada: {}", fam_id_s),
    Err(e) => println!("Error al obtener familia: {:?}", e),
  }
  Ok(())
}
fn list_families_show_molecules(repo: &dyn chem_domain::DomainRepository) -> Result<(), Box<dyn Error>> {
  match repo.list_families() {
    Ok(fams) => {
      println!("Familias encontradas: {}", fams.len());
      for f in fams {
        println!("- Family id={} molecules_count={}", f.id(), f.len());
        for m in f.molecules() {
          println!("   - {}", m.inchikey());
        }
      }
    }
    Err(e) => println!("Error listando familias: {:?}", e),
  }
  Ok(())
}
fn show_family(repo: &dyn chem_domain::DomainRepository) -> Result<(), Box<dyn Error>> {
  let fam_id_s = prompt("ID de la familia a mostrar: ")?;
  let fam_id = match Uuid::parse_str(&fam_id_s) {
    Ok(u) => u,
    Err(_) => {
      println!("ID de familia inválido");
      return Ok(());
    }
  };
  match repo.get_family(&fam_id) {
    Ok(Some(f)) => {
      println!("Familia id={} tamaño={} provenance={}", f.id(), f.len(), f.provenance());
      // list family-level properties if any
      match repo.get_family_properties(&fam_id) {
        Ok(props) => {
          println!("Propiedades de la familia ({}):", props.len());
          for p in props {
            println!("- id={} type={} value={} metadata={}",
                     p.id, p.property_type, p.value, p.metadata);
          }
        }
        Err(e) => println!("Error obteniendo propiedades de la familia: {:?}", e),
      }
      println!("Moléculas en la familia:");
      for m in f.molecules() {
        println!("- InChIKey={} SMILES={} InChI={} metadata={}",
                 m.inchikey(),
                 m.smiles(),
                 m.inchi(),
                 m.metadata());
        match repo.get_molecular_properties(m.inchikey()) {
          Ok(props) => {
            for p in props {
              println!("    - prop id={} type={} value={} metadata={}",
                       p.id, p.property_type, p.value, p.metadata);
            }
          }
          Err(e) => println!("    Error obteniendo propiedades moleculares: {:?}", e),
        }
      }
    }
    Ok(None) => println!("Familia no encontrada: {}", fam_id_s),
    Err(e) => println!("Error al obtener familia: {:?}", e),
  }
  Ok(())
}
fn remove_molecule_from_family(repo: &dyn chem_domain::DomainRepository) -> Result<(), Box<dyn Error>> {
  let fam_id_s = prompt("ID de la familia de la que quitar la molécula: ")?;
  let inchikey = prompt("InChIKey de la molécula a quitar: ")?;
  let fam_id = match Uuid::parse_str(&fam_id_s) {
    Ok(u) => u,
    Err(_) => {
      println!("ID inválido");
      return Ok(());
    }
  };
  match repo.remove_molecule_from_family(&fam_id, &inchikey) {
    Ok(new_id) => println!("Se creó nueva versión de la familia con id={}", new_id),
    Err(e) => println!("Error removiendo molécula de la familia: {:?}", e),
  }
  Ok(())
}
fn delete_molecule(repo: &dyn chem_domain::DomainRepository) -> Result<(), Box<dyn Error>> {
  let inchikey = prompt("InChIKey de la molécula a eliminar: ")?;
  match repo.delete_molecule(&inchikey) {
    Ok(()) => println!("Molécula eliminada: {}", inchikey),
    Err(e) => println!("Error eliminando molécula: {:?}", e),
  }
  Ok(())
}
fn delete_family(repo: &dyn chem_domain::DomainRepository) -> Result<(), Box<dyn Error>> {
  let fam_id_s = prompt("ID de la familia a eliminar: ")?;
  let fam_id = match Uuid::parse_str(&fam_id_s) {
    Ok(u) => u,
    Err(_) => {
      println!("ID inválido");
      return Ok(());
    }
  };
  match repo.delete_family(&fam_id) {
    Ok(()) => println!("Familia eliminada: {}", fam_id),
    Err(e) => println!("Error eliminando familia: {:?}", e),
  }
  Ok(())
}
