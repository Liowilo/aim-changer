//! Matemática de conversión de sensibilidad.
//!
//! Idea central: cada juego gira `yaw × sens_efectiva` grados por cada count del mouse,
//! por eje. La sens efectiva sale de los valores del menú según [`SensLayout`].
//! Convertir = encontrar los valores del menú del juego destino que produzcan el mismo
//! movimiento físico para una referencia dada (360°, o un punto de la pantalla).

use crate::games::{FovScaling, FovSpec, FovType, Game, SensLayout, SensRange, SensValues};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Method {
    /// Misma distancia física para un giro de 360°. Independiente del FOV.
    /// Es el método recomendado para memoria muscular de giros/flicks grandes.
    Distance360,
    /// Monitor Distance Match: misma distancia para llevar la mira a un punto
    /// que está al `percent`% entre el centro y el borde horizontal de la pantalla.
    /// 0% = coincidencia de "focal length" (tracking cerca de la mira).
    MonitorDistance { percent: f64 },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertRequest {
    pub from_game: String,
    pub to_game: String,
    /// Valores del menú del juego origen.
    pub sens: SensValues,
    pub dpi: f64,
    pub method: Method,
    /// FOV configurado en cada juego (en sus propias unidades). `None` = default.
    pub from_fov: Option<f64>,
    pub to_fov: Option<f64>,
    /// Aspect ratio del monitor del usuario, p. ej. [16, 9].
    pub aspect: [f64; 2],
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConvertResult {
    /// Valores a escribir en el menú del juego destino (redondeados y dentro de rango).
    pub values: SensValues,
    /// Sens efectiva exacta que necesitaría el destino, por eje.
    pub exact_x: f64,
    pub exact_y: f64,
    /// Sens efectiva que se logra con `values`.
    pub achieved_x: f64,
    pub achieved_y: f64,
    /// Diferencia (%) entre lo logrado y lo exacto en el eje horizontal.
    pub error_percent: f64,
    pub out_of_range: bool,
    pub cm_per_360_from: f64,
    pub cm_per_360_to: f64,
    pub inch_per_360_from: f64,
    pub hfov_from: f64,
    pub hfov_to: f64,
    pub warnings: Vec<String>,
}

const CM_PER_INCH: f64 = 2.54;

/// `true` si es un número finito mayor que 0 (rechaza NaN e infinito).
fn is_positive(x: f64) -> bool {
    x.is_finite() && x > 0.0
}

fn positive(x: f64, what: &str) -> Result<f64, String> {
    if is_positive(x) {
        Ok(x)
    } else {
        Err(format!("{what} debe ser mayor que 0"))
    }
}

/// Centímetros de alfombrilla necesarios para un giro completo.
pub fn cm_per_360(yaw: f64, sens: f64, dpi: f64) -> f64 {
    360.0 / (yaw * sens * dpi) * CM_PER_INCH
}

/// Sens efectiva (x, y) a partir de los valores del menú. Multiplicada por `game.yaw`
/// da los grados por count de cada eje.
pub fn effective(game: &Game, v: &SensValues) -> Result<(f64, f64), String> {
    let main = positive(v.main, "La sensibilidad")?;
    match &game.layout {
        SensLayout::Single => Ok((main, main)),
        SensLayout::SeparateXY => Ok((main, positive(v.y.unwrap_or(main), "La sensibilidad Y")?)),
        SensLayout::ScaledXY { reference, axis } => {
            let x = positive(v.x.unwrap_or(axis.default), "La sensibilidad horizontal")?;
            let y = positive(v.y.unwrap_or(axis.default), "La sensibilidad vertical")?;
            Ok((main * x / reference, main * y / reference))
        }
    }
}

fn round_to(value: f64, decimals: u32) -> f64 {
    let f = 10f64.powi(decimals as i32);
    (value * f).round() / f
}

/// Redondea a la precisión del ajuste y lo limita a su rango. Devuelve (valor, se_limitó).
fn fit(value: f64, range: &SensRange) -> (f64, bool) {
    let rounded = round_to(value, range.decimals);
    let clamped = rounded.clamp(range.min, range.max);
    (clamped, clamped != rounded)
}

/// Todos los valores que acepta un ajuste (según rango y decimales).
fn steps(range: &SensRange) -> impl Iterator<Item = f64> {
    let f = 10f64.powi(range.decimals as i32);
    let lo = (range.min * f).ceil() as i64;
    let hi = (range.max * f).floor() as i64;
    (lo..=hi).map(move |i| i as f64 / f)
}

/// Valores del menú del destino que más se acercan a la sens efectiva (ex, ey).
/// Devuelve (valores, algún_valor_quedó_fuera_de_rango).
pub fn decompose(game: &Game, ex: f64, ey: f64) -> (SensValues, bool) {
    match &game.layout {
        SensLayout::Single => {
            let (main, out) = fit(ex, &game.sens);
            (SensValues { main, x: None, y: None }, out)
        }
        SensLayout::SeparateXY => {
            let (main, out_x) = fit(ex, &game.sens);
            let (y, out_y) = fit(ey, &game.sens);
            (SensValues { main, x: None, y: Some(y) }, out_x || out_y)
        }
        SensLayout::ScaledXY { reference, axis } => best_scaled(&game.sens, axis, *reference, ex, ey),
    }
}

/// Busca la combinación base × eje/reference más cercana a `ex`. Con la precisión de
/// estos menús (p. ej. base con 1 decimal y ejes enteros) casi nunca hay un valor exacto;
/// entre las combinaciones casi igual de precisas se prefiere el eje más cercano a su
/// valor por defecto, para no dejar ejes extremos.
fn best_scaled(base: &SensRange, axis: &SensRange, reference: f64, ex: f64, ey: f64) -> (SensValues, bool) {
    /// Tolerancia relativa para considerar dos combinaciones "igual de precisas".
    const TIE: f64 = 0.0005;
    let candidates: Vec<(f64, f64, f64, bool)> = steps(base)
        .map(|b| {
            let (x, out) = fit(ex * reference / b, axis);
            let err = (b * x / reference - ex).abs() / ex;
            (b, x, err, out)
        })
        .collect();
    let min_err = candidates.iter().map(|c| c.2).fold(f64::INFINITY, f64::min);
    let (b, x, _, out_x) = candidates
        .into_iter()
        .filter(|c| c.2 <= min_err + TIE)
        .min_by(|a, b| {
            let da = (a.1 - axis.default).abs();
            let db = (b.1 - axis.default).abs();
            da.total_cmp(&db).then(a.2.total_cmp(&b.2))
        })
        .expect("el rango base no puede estar vacío");
    let (y, out_y) = fit(ey * reference / b, axis);
    (SensValues { main: b, x: Some(x), y: Some(y) }, out_x || out_y)
}

/// FOV horizontal real (grados) en el aspect ratio del usuario.
pub fn horizontal_fov(spec: &FovSpec, value: f64, aspect: [f64; 2]) -> f64 {
    if spec.kind == FovType::Horizontal && spec.scaling == FovScaling::FixedHorizontal {
        return value;
    }
    let target = aspect[0] / aspect[1];
    let vfov_rad = match spec.kind {
        FovType::Vertical => value.to_radians(),
        FovType::Horizontal => {
            let base = spec.base_aspect[0] / spec.base_aspect[1];
            2.0 * ((value.to_radians() / 2.0).tan() / base).atan()
        }
    };
    (2.0 * ((vfov_rad / 2.0).tan() * target).atan()).to_degrees()
}

/// Ángulo (rad) desde el centro hasta un punto al `p` (0..1) del semiancho de pantalla.
fn angle_at_fraction(hfov_deg: f64, p: f64) -> f64 {
    (p * (hfov_deg.to_radians() / 2.0).tan()).atan()
}

/// Factor por el que se multiplica la relación de yaws según el método.
fn fov_factor(method: Method, hfov_from: f64, hfov_to: f64) -> f64 {
    match method {
        Method::Distance360 => 1.0,
        Method::MonitorDistance { percent } => {
            let p = percent / 100.0;
            if p <= 1e-9 {
                // Límite p -> 0: relación de tangentes (focal length).
                (hfov_to.to_radians() / 2.0).tan() / (hfov_from.to_radians() / 2.0).tan()
            } else {
                angle_at_fraction(hfov_to, p) / angle_at_fraction(hfov_from, p)
            }
        }
    }
}

pub fn convert(from: &Game, to: &Game, req: &ConvertRequest) -> Result<ConvertResult, String> {
    positive(req.dpi, "El DPI")?;
    if !(is_positive(req.aspect[0]) && is_positive(req.aspect[1])) {
        return Err("Aspect ratio inválido".into());
    }
    let (src_x, src_y) = effective(from, &req.sens)?;

    let fov_from_val = req.from_fov.unwrap_or(from.fov.default);
    let fov_to_val = req.to_fov.unwrap_or(to.fov.default);
    let hfov_from = horizontal_fov(&from.fov, fov_from_val, req.aspect);
    let hfov_to = horizontal_fov(&to.fov, fov_to_val, req.aspect);

    // El mismo factor se aplica a ambos ejes: conserva la relación vertical/horizontal.
    let factor = (from.yaw / to.yaw) * fov_factor(req.method, hfov_from, hfov_to);
    let (exact_x, exact_y) = (src_x * factor, src_y * factor);
    let (values, out_of_range) = decompose(to, exact_x, exact_y);
    let (achieved_x, achieved_y) = effective(to, &values)?;

    let mut warnings = Vec::new();
    if out_of_range {
        warnings.push(format!(
            "El resultado queda fuera del rango que permite {}. Cambia el DPI del mouse para compensar.",
            to.name
        ));
    }
    let ratio = src_y / src_x;
    if matches!(to.layout, SensLayout::Single) && (ratio - 1.0).abs() > 0.005 {
        warnings.push(format!(
            "{} no tiene sensibilidad vertical separada: tu vertical en {} es {:.0}% de la horizontal y se perderá esa diferencia.",
            to.name,
            from.name,
            ratio * 100.0
        ));
    }
    for g in [from, to] {
        if !g.verified.yaw {
            warnings.push(format!("El yaw de {} no está verificado; haz una prueba de 360°.", g.name));
        }
        if !g.verified.layout {
            warnings.push(format!(
                "La forma en que {} combina sus ajustes de sensibilidad no está verificada; confírmala con la Prueba 360°.",
                g.name
            ));
        }
    }

    Ok(ConvertResult {
        values,
        exact_x,
        exact_y,
        achieved_x,
        achieved_y,
        error_percent: (achieved_x - exact_x) / exact_x * 100.0,
        out_of_range,
        cm_per_360_from: cm_per_360(from.yaw, src_x, req.dpi),
        cm_per_360_to: cm_per_360(to.yaw, achieved_x, req.dpi),
        inch_per_360_from: cm_per_360(from.yaw, src_x, req.dpi) / CM_PER_INCH,
        hfov_from,
        hfov_to,
        warnings,
    })
}

// ---------------------------------------------------------------------------
// Calibración (prueba de 360°)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalibrateRequest {
    /// Juego de la base de datos con el que comparar (`None` para juegos nuevos,
    /// en cuyo caso solo se usa `sens.main`).
    pub game_id: Option<String>,
    pub sens: SensValues,
    pub dpi: f64,
    /// Número de vueltas completas que dio el usuario.
    pub turns: f64,
    /// Counts horizontales netos capturados durante la prueba.
    pub counts: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalibrateResult {
    pub measured_yaw: f64,
    pub counts_per_360: f64,
    pub cm_per_360: f64,
    pub expected_yaw: Option<f64>,
    pub expected_counts: Option<f64>,
    /// Diferencia porcentual entre el yaw medido y el de la base de datos.
    pub error_percent: Option<f64>,
}

/// Yaw (grados por count con sens efectiva 1) a partir de una medición.
pub fn yaw_from_counts(turns: f64, counts: i64, sens: f64) -> f64 {
    360.0 * turns / (counts.unsigned_abs() as f64 * sens)
}

pub fn calibrate(game: Option<&Game>, req: &CalibrateRequest) -> Result<CalibrateResult, String> {
    positive(req.turns, "El número de vueltas")?;
    positive(req.dpi, "El DPI")?;
    let sens_x = match game {
        Some(g) => effective(g, &req.sens)?.0,
        None => positive(req.sens.main, "La sensibilidad")?,
    };
    if req.counts == 0 {
        return Err("No se registró movimiento horizontal del mouse".into());
    }
    let measured_yaw = yaw_from_counts(req.turns, req.counts, sens_x);
    let counts_per_360 = req.counts.unsigned_abs() as f64 / req.turns;
    Ok(CalibrateResult {
        measured_yaw,
        counts_per_360,
        cm_per_360: counts_per_360 / req.dpi * CM_PER_INCH,
        expected_yaw: game.map(|g| g.yaw),
        expected_counts: game.map(|g| 360.0 * req.turns / (g.yaw * sens_x)),
        error_percent: game.map(|g| (measured_yaw - g.yaw) / g.yaw * 100.0),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::games;

    fn game(id: &str) -> &'static Game {
        games::find(id).unwrap()
    }

    fn single(v: f64) -> SensValues {
        SensValues { main: v, x: None, y: None }
    }

    fn req(from: &str, to: &str, sens: SensValues, method: Method) -> ConvertRequest {
        ConvertRequest {
            from_game: from.into(),
            to_game: to.into(),
            sens,
            dpi: 800.0,
            method,
            from_fov: None,
            to_fov: None,
            aspect: [16.0, 9.0],
        }
    }

    fn run(from: &str, to: &str, sens: SensValues, method: Method) -> ConvertResult {
        convert(game(from), game(to), &req(from, to, sens, method)).unwrap()
    }

    // --- Sens efectiva por layout ---

    #[test]
    fn doom_effective_uses_axis_over_40() {
        let d = game("doom-eternal");
        let v = SensValues { main: 3.2, x: Some(40.0), y: Some(20.0) };
        assert_eq!(effective(d, &v).unwrap(), (3.2, 1.6));
    }

    #[test]
    fn fortnite_effective_has_independent_y() {
        let f = game("fortnite");
        let v = SensValues { main: 6.0, x: None, y: Some(4.5) };
        assert_eq!(effective(f, &v).unwrap(), (6.0, 4.5));
    }

    // --- Conversiones ---

    #[test]
    fn valorant_to_doom_360() {
        let r = run("valorant", "doom-eternal", single(0.4), Method::Distance360);
        // 0.4 × 0.07 / 0.022 = 1.2727 de sens efectiva
        assert!((r.exact_x - 1.272_727).abs() < 1e-5);
        // Límite de precisión del menú (1 decimal × entero): < 0.5 %.
        assert!(r.error_percent.abs() < 0.5, "{r:?}");
        // Sin sens vertical distinta en el origen, ambos ejes quedan iguales.
        assert_eq!(r.values.x, r.values.y);
        assert!((r.cm_per_360_from - r.cm_per_360_to).abs() < 0.1);
    }

    #[test]
    fn doom_prefers_axes_at_40_when_exact() {
        // CS2 3.2 → DOOM: Escalado 3.2 con ejes en 40 es exacto.
        let r = run("cs2", "doom-eternal", single(3.2), Method::Distance360);
        assert_eq!(r.values, SensValues { main: 3.2, x: Some(40.0), y: Some(40.0) });
    }

    #[test]
    fn doom_to_cs2_reads_user_settings() {
        // Ajustes de la captura del usuario: 3.2 / 40 / 40 → CS2 3.2
        let v = SensValues { main: 3.2, x: Some(40.0), y: Some(40.0) };
        let r = run("doom-eternal", "cs2", v, Method::Distance360);
        assert_eq!(r.values.main, 3.2);
        assert!(r.warnings.iter().any(|w| w.contains("no está verificada")));
    }

    #[test]
    fn xy_ratio_is_kept_between_xy_games() {
        // Fortnite X 6.0 / Y 4.5 (75%) → DOOM: vertical ≈ 75% de la horizontal
        let v = SensValues { main: 6.0, x: None, y: Some(4.5) };
        let r = run("fortnite", "doom-eternal", v, Method::Distance360);
        let ratio = r.achieved_y / r.achieved_x;
        assert!((ratio - 0.75).abs() < 0.02, "{r:?}");
    }

    #[test]
    fn warns_when_destination_has_no_y() {
        let v = SensValues { main: 6.0, x: None, y: Some(4.5) };
        let r = run("fortnite", "valorant", v, Method::Distance360);
        assert!(r.warnings.iter().any(|w| w.contains("vertical separada")));
    }

    #[test]
    fn valorant_to_overwatch_360() {
        // 0.4 × 0.07 / 0.0066 = 4.2424...
        assert_eq!(run("valorant", "overwatch-2", single(0.4), Method::Distance360).values.main, 4.24);
    }

    #[test]
    fn valorant_to_fortnite_360() {
        let r = run("valorant", "fortnite", single(0.4), Method::Distance360);
        // 0.4 × 0.07 / 0.005555 = 5.0405 % → el menú acepta 1 decimal
        assert!((r.exact_x - 5.0405).abs() < 1e-3);
        assert_eq!(r.values, SensValues { main: 5.0, x: None, y: Some(5.0) });
    }

    #[test]
    fn mdm_same_fov_equals_360() {
        let mut a = req("valorant", "overwatch-2", single(0.4), Method::MonitorDistance { percent: 0.0 });
        a.to_fov = Some(103.0);
        let a = convert(game("valorant"), game("overwatch-2"), &a).unwrap();
        let b = run("valorant", "overwatch-2", single(0.4), Method::Distance360);
        assert!((a.exact_x - b.exact_x).abs() < 1e-9);
    }

    #[test]
    fn mdm_lower_fov_lowers_sens() {
        let mdm = run("valorant", "doom-eternal", single(0.4), Method::MonitorDistance { percent: 0.0 });
        let d360 = run("valorant", "doom-eternal", single(0.4), Method::Distance360);
        // DOOM a 90° < VALORANT 103° → menos sens para igualar el movimiento en pantalla.
        assert!(mdm.exact_x < d360.exact_x);
    }

    #[test]
    fn rejects_invalid_input() {
        let r = req("valorant", "doom-eternal", single(0.0), Method::Distance360);
        assert!(convert(game("valorant"), game("doom-eternal"), &r).is_err());
    }

    // --- FOV ---

    #[test]
    fn cm360_known_value() {
        // VALORANT 0.4 @ 800 DPI ≈ 40.82 cm/360
        assert!((cm_per_360(0.07, 0.4, 800.0) - 40.816).abs() < 0.01);
    }

    #[test]
    fn horizontal_fov_same_aspect_is_identity() {
        assert!((horizontal_fov(&game("valorant").fov, 103.0, [16.0, 9.0]) - 103.0).abs() < 1e-9);
    }

    #[test]
    fn cs2_fov_is_hor_plus() {
        // 90° en 4:3 → 106.26° en 16:9
        assert!((horizontal_fov(&game("cs2").fov, 90.0, [16.0, 9.0]) - 106.26).abs() < 0.01);
    }

    #[test]
    fn overwatch_fov_is_fixed_horizontal() {
        assert_eq!(horizontal_fov(&game("overwatch-2").fov, 103.0, [64.0, 27.0]), 103.0);
    }

    // --- Calibración ---

    #[test]
    fn calibrate_recovers_yaw() {
        // VALORANT 0.4: 360 / (0.07 × 0.4) = 12857.14 counts por vuelta; 3 vueltas.
        let r = calibrate(
            Some(game("valorant")),
            &CalibrateRequest { game_id: None, sens: single(0.4), dpi: 800.0, turns: 3.0, counts: -38571 },
        )
        .unwrap();
        assert!((r.measured_yaw - 0.07).abs() < 1e-5);
        assert!(r.error_percent.unwrap().abs() < 0.01);
        assert!((r.cm_per_360 - 40.82).abs() < 0.01);
    }

    #[test]
    fn calibrate_doom_uses_effective_sens() {
        // DOOM 3.2 con horizontal 20 → sens efectiva 1.6; 360/(0.022×1.6) = 10227.27 counts
        let v = SensValues { main: 3.2, x: Some(20.0), y: Some(40.0) };
        let r = calibrate(
            Some(game("doom-eternal")),
            &CalibrateRequest { game_id: None, sens: v, dpi: 800.0, turns: 1.0, counts: 10227 },
        )
        .unwrap();
        assert!(r.error_percent.unwrap().abs() < 0.01, "{r:?}");
    }

    #[test]
    fn calibrate_rejects_zero_counts() {
        let r = CalibrateRequest { game_id: None, sens: single(1.0), dpi: 800.0, turns: 1.0, counts: 0 };
        assert!(calibrate(None, &r).is_err());
    }
}
