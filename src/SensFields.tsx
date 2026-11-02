import type { Game, SensValues } from "./types";

/** Valores del formulario como texto (para permitir escribir "0." etc.). */
export interface SensForm {
  main: string;
  x: string;
  y: string;
}

const step = (decimals: number) => (decimals === 0 ? "1" : String(10 ** -decimals));

/** Formulario con los valores por defecto del juego (o `main` si se indica). */
export function defaultForm(game: Game | undefined, main?: number): SensForm {
  const m = String(main ?? game?.sens.default ?? 1);
  if (game?.layout.kind === "scaledXY") {
    const axis = String(game.layout.axis.default);
    return { main: m, x: axis, y: axis };
  }
  return { main: m, x: "", y: m };
}

export function formToValues(game: Game | undefined, form: SensForm): SensValues {
  const num = (s: string) => (s.trim() === "" ? null : parseFloat(s));
  const main = parseFloat(form.main);
  switch (game?.layout.kind) {
    case "separateXY":
      return { main, y: num(form.y) };
    case "scaledXY":
      return { main, x: num(form.x), y: num(form.y) };
    default:
      return { main };
  }
}

export function valuesToForm(values: SensValues): SensForm {
  return {
    main: String(values.main),
    x: values.x == null ? "" : String(values.x),
    y: values.y == null ? String(values.main) : String(values.y),
  };
}

interface Props {
  game: Game | undefined;
  form: SensForm;
  onChange: (form: SensForm) => void;
}

/** Campos de sensibilidad según cómo los organiza el menú del juego. */
export default function SensFields({ game, form, onChange }: Props) {
  const set = (k: keyof SensForm) => (e: React.ChangeEvent<HTMLInputElement>) =>
    onChange({ ...form, [k]: e.target.value });
  if (!game) {
    // Juego sin datos (calibración): un único valor.
    return (
      <label>
        Sensibilidad
        <input type="number" step="any" value={form.main} onChange={set("main")} />
      </label>
    );
  }
  const unit = game.unit ? ` (${game.unit})` : "";
  const mainStep = step(game.sens.decimals);

  switch (game.layout.kind) {
    case "separateXY":
      return (
        <div className="axes">
          <label>
            {game.sens.label ?? "Sensibilidad X"}{unit}
            <input type="number" step={mainStep} value={form.main} onChange={set("main")} />
          </label>
          <label>
            Sensibilidad Y{unit}
            <input type="number" step={mainStep} value={form.y} onChange={set("y")} />
          </label>
        </div>
      );
    case "scaledXY": {
      const { axis, reference } = game.layout;
      const axisStep = step(axis.decimals);
      return (
        <>
          <label>
            {game.sens.label ?? "Sensibilidad"}{unit}
            <input type="number" step={mainStep} value={form.main} onChange={set("main")} />
          </label>
          <div className="axes">
            <label>
              Horizontal
              <input type="number" step={axisStep} value={form.x} onChange={set("x")} />
            </label>
            <label>
              Vertical
              <input type="number" step={axisStep} value={form.y} onChange={set("y")} />
            </label>
          </div>
          <p className="note">Horizontal y vertical multiplican la base: {reference} = ×1.</p>
        </>
      );
    }
    default:
      return (
        <label>
          {game.sens.label ?? "Sensibilidad"}{unit}
          <input type="number" step={mainStep} value={form.main} onChange={set("main")} />
        </label>
      );
  }
}

/** Muestra los valores a escribir en el menú del juego destino. */
export function SensOutput({ game, values }: { game: Game; values: SensValues }) {
  const unit = game.unit ?? "";
  switch (game.layout.kind) {
    case "separateXY":
      return (
        <div className="outputs">
          <div><span>{game.sens.label ?? "Sensibilidad X"}</span><strong>{values.main}{unit}</strong></div>
          <div><span>Sensibilidad Y</span><strong>{values.y}{unit}</strong></div>
        </div>
      );
    case "scaledXY":
      return (
        <div className="outputs">
          <div><span>{game.sens.label ?? "Base"}</span><strong>{values.main}{unit}</strong></div>
          <div><span>Horizontal</span><strong>{values.x}</strong></div>
          <div><span>Vertical</span><strong>{values.y}</strong></div>
        </div>
      );
    default:
      return (
        <div className="outputs">
          <div><span>{game.sens.label ?? "Sensibilidad"}</span><strong>{values.main}{unit}</strong></div>
        </div>
      );
  }
}
