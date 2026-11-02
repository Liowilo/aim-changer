// Espejo de los tipos serializados por src-tauri/src/{games,conversion}.rs.
// Mantener sincronizado si cambian los structs de Rust.

export interface SensRange {
  min: number;
  max: number;
  decimals: number;
  default: number;
  /** Nombre del ajuste en el menú del juego. */
  label?: string;
}

/** Cómo organiza el menú del juego la sensibilidad (ver SensLayout en games.rs). */
export type SensLayout =
  | { kind: "single" }
  | { kind: "separateXY" }
  | { kind: "scaledXY"; reference: number; axis: SensRange };

/**
 * Valores tal como se escriben en el menú del juego.
 * single: main · separateXY: main = X, y = Y · scaledXY: main = base, x/y = ejes.
 */
export interface SensValues {
  main: number;
  x?: number | null;
  y?: number | null;
}

export interface Game {
  id: string;
  name: string;
  yaw: number;
  sens: SensRange;
  layout: SensLayout;
  fov: {
    type: "horizontal" | "vertical";
    scaling: "horPlus" | "fixedHorizontal";
    baseAspect: [number, number];
    default: number;
    min: number;
    max: number;
    editable: boolean;
  };
  verified: { yaw: boolean; sensRange: boolean; fov: boolean; layout: boolean };
  notes: string;
  /** Unidad de la sensibilidad, p. ej. "%" en Fortnite. */
  unit?: string;
}

export type Method =
  | { kind: "distance360" }
  | { kind: "monitorDistance"; percent: number };

export interface ConvertRequest {
  fromGame: string;
  toGame: string;
  sens: SensValues;
  dpi: number;
  method: Method;
  fromFov: number | null;
  toFov: number | null;
  aspect: [number, number];
}

export interface ConvertResult {
  /** Valores a escribir en el menú del juego destino. */
  values: SensValues;
  exactX: number;
  exactY: number;
  achievedX: number;
  achievedY: number;
  errorPercent: number;
  outOfRange: boolean;
  cmPer360From: number;
  cmPer360To: number;
  inchPer360From: number;
  hfovFrom: number;
  hfovTo: number;
  warnings: string[];
}

export interface CaptureStatus {
  recording: boolean;
  countsX: number;
  countsY: number;
  events: number;
}

export interface CalibrateRequest {
  gameId: string | null;
  sens: SensValues;
  dpi: number;
  turns: number;
  counts: number;
}

export interface CalibrateResult {
  measuredYaw: number;
  countsPer360: number;
  cmPer360: number;
  expectedYaw: number | null;
  expectedCounts: number | null;
  errorPercent: number | null;
}
