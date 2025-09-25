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
