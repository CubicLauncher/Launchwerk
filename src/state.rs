use crate::models::{Loader, MCVersion};
use dashmap::DashMap;
use std::process::Child;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// # `Launchwerk`
///
/// Estructura principal de la lib la cual mantiene el estado
///
/// ## Campos
///
/// - `launch_handles` (`DashMap<Uuid, LaunchInst>`): Handle de cada instancia guardado en dashmasp, usando como clave el Uuid
struct Launchwerk {
    launch_handles: DashMap<Uuid, LaunchInst>,
}

/// # `LaunchInst`
///
/// Son la representacion de una instancia lanzada
///
/// ## Campos
///
/// - `uuid` (`Uuid`): ID unico del lanzamiento, sirve para obtner los handles.
/// - `data` (`LaunchData`): Mantiene los datos los cuales no son reescribibles.
/// - `runtime` (`Arc<RwLock<LaunchRuntime>>`): Mantiene los datos los cuales son necesarios que tengan un Lock.
struct LaunchInst {
    uuid: Uuid,
    data: LaunchData,
    runtime: Arc<RwLock<LaunchRuntime>>,
}

struct LaunchRuntime {
    process: Child,
}
struct LaunchData {
    version: MCVersion,
    loader: Loader,
}
