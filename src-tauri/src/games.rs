//! Base de datos de juegos soportados. Los datos viven en `data/games.json`
//! y se incrustan en el binario en tiempo de compilación.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SensRange {
    pub min: f64,
    pub max: f64,
    pub decimals: u32,
    pub default: f64,
    /// Nombre del ajuste tal como aparece en el menú del juego.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// Cómo organiza el menú del juego la sensibilidad del mouse.
/// Los valores que el usuario escribe llegan como [`SensValues`].
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(tag = "kind")]
pub enum SensLayout {
    /// Un único valor para ambos ejes (`main`).
    #[default]
    #[serde(rename = "single")]
    Single,
    /// Sensibilidad X (`main`) e Y (`y`) independientes, en la misma escala (Fortnite).
    #[serde(rename = "separateXY")]
    SeparateXY,
    /// Base (`main`) × eje / `reference`, con ejes `x` e `y` (DOOM Eternal:
    /// "Escalado de la sensibilidad" × "Sensibilidad horizontal/vertical" / 40).
    #[serde(rename = "scaledXY")]
    ScaledXY { reference: f64, axis: SensRange },
}

/// Valores tal como se escriben en el menú del juego; su significado depende de [`SensLayout`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SensValues {
    pub main: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub x: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub y: Option<f64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FovType {
    /// FOV horizontal medido en `base_aspect` (Hor+ en otros aspect ratios).
    Horizontal,
    /// FOV vertical (independiente del aspect ratio).
    Vertical,
}

/// Cómo cambia el FOV cuando el aspect ratio no es `base_aspect`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub enum FovScaling {
    /// Se conserva el FOV vertical; pantallas más anchas ven más a los lados.
    #[default]
    HorPlus,
    /// Se conserva el FOV horizontal en cualquier aspect ratio.
    FixedHorizontal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FovSpec {
    #[serde(rename = "type")]
    pub kind: FovType,
    pub base_aspect: [f64; 2],
    #[serde(default)]
    pub scaling: FovScaling,
    pub default: f64,
    pub min: f64,
    pub max: f64,
    pub editable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Verified {
    pub yaw: bool,
    pub sens_range: bool,
    pub fov: bool,
    /// Si la fórmula de `layout` (cómo se combinan los valores) está comprobada.
    #[serde(default = "default_true")]
    pub layout: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Game {
    pub id: String,
    pub name: String,
    /// Grados que gira la cámara por cada "count" del mouse con sensibilidad 1.0.
    pub yaw: f64,
    /// Rango del valor principal (`SensValues::main`).
    pub sens: SensRange,
    #[serde(default)]
    pub layout: SensLayout,
    pub fov: FovSpec,
    pub verified: Verified,
    pub notes: String,
    /// Unidad en que el juego muestra la sensibilidad (p. ej. "%" en Fortnite).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
}

static GAMES: OnceLock<Vec<Game>> = OnceLock::new();

pub fn all() -> &'static [Game] {
    GAMES.get_or_init(|| {
        serde_json::from_str(include_str!("../data/games.json"))
            .expect("data/games.json inválido")
    })
}

pub fn find(id: &str) -> Option<&'static Game> {
    all().iter().find(|g| g.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn games_json_is_valid() {
        let games = all();
        assert!(games.len() >= 2);
        for g in games {
            assert!(g.yaw > 0.0, "{} yaw", g.id);
            assert!(g.sens.min < g.sens.max, "{} sens range", g.id);
            assert!(g.fov.min <= g.fov.default && g.fov.default <= g.fov.max, "{} fov", g.id);
        }
    }

    #[test]
    fn ids_are_unique() {
        let mut ids: Vec<_> = all().iter().map(|g| g.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), all().len());
    }
}
