# chem-workflow

Crate inicial para definir el trait `ChemicalFlowEngine` y la estructura de
carpetas para implementar motores de flujo quimicos (por ejemplo `CadmaFlow`).
Este crate depende de `flow` y `chem-domain` del workspace y ofrece los
traits y tipos base. Los ficheros creados aqui son esqueletos para arrancar el
desarrollo.

Ver tambien los READMEs de `crates/chem-domain` y `crates/chem-persistence` para
contexto de persistencia y modelos de dominio.

## StepContext (ayuda para autores de pasos)

El helper `StepContext` expone utilidades convenientes para los autores de
pasos:

- `get_typed_output_by_name<T>(&self, step_name)` — lee el último payload
  persistido para `step_name` y lo deserializa en `T` (case-insensitive
  lookup sobre la clave `step_state:{step_name}`).
- `get_typed_output_by_type<T>(&self)` — intenta encontrar el último payload
  que pueda deserializarse en `T` y lo devuelve; útil cuando el tipo ya
  identifica el dato buscado y evita tener que pasar el nombre del paso.
- `save_typed_result(&self, step_name, info, expected_version, command_id)` —
  persiste un `StepInfo` usando la convención `step_state:{step_name}`.

Recomendación para pasos:

- Implementar `execute_with_context(&self, ctx: &StepContext, input: &JsonValue)`
  cuando necesites acceder a repositorios o a outputs tipados de pasos
  previos. En el ejemplo `CadmaFlow`, `execute_current_step` construye un
  `StepContext` y llama a `execute_with_context`.

### Formato de clave y deduplicación

Para garantizar rehidratación consistente a través de ramas, los resultados de
pasos se almacenan en `FlowData.key` siguiendo la convención:

- `step_state:<STEP_NAME>` — donde `<STEP_NAME>` es el identificador lógico del
	paso. Esta clave se genera con `step::constants::key_for_step_state(step_name)`.

El método `StepContext::save_typed_result` aplica una guarda de deduplicación:

- Antes de insertar, recorre los `FlowData` existentes del flujo y si encuentra
	un registro con la misma clave (`eq_ignore_ascii_case`) y un `payload` idéntico,
  no inserta un nuevo registro y devuelve `PersistResult::Ok` con la versión
	actual. Esto evita duplicaciones y mantiene historia limpia.

Esta convención permite:

- Rehidratación simple con `get_step_payload_by_name[_typed]`, que busca desde
  el final hacia atrás la última coincidencia de `step_state:<NAME>`.
- Consistencia entre ramas: al crear una rama, los `FlowData` hasta el cursor
  se copian preservando las claves y payloads, por lo que los pasos siguientes
  pueden rehidratar sin lógica adicional.
