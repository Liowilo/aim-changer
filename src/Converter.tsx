import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { ConvertRequest, ConvertResult, Game, Method } from "./types";
import SensFields, { SensOutput, defaultForm, formToValues, valuesToForm, type SensForm } from "./SensFields";

const ASPECTS: Record<string, [number, number]> = {
  "16:9": [16, 9],
  "16:10": [16, 10],
  "21:9": [64, 27],
  "4:3": [4, 3],
};

type MethodKey = "distance360" | "mdm0" | "mdm56" | "mdm100";
const METHODS: Record<MethodKey, { label: string; hint: string; method: Method }> = {
  distance360: {
    label: "Distancia 360°",
    hint: "Mismo movimiento de brazo para dar una vuelta completa. Ideal para conservar la memoria muscular.",
    method: { kind: "distance360" },
  },
  mdm0: {
    label: "Monitor 0%",
    hint: "Igual sensación al mover la mira cerca del centro (tracking). Depende del FOV.",
    method: { kind: "monitorDistance", percent: 0 },
  },
  mdm56: {
    label: "Monitor 56.25%",
    hint: "Compromiso popular entre flicks cortos y tracking. Depende del FOV.",
    method: { kind: "monitorDistance", percent: 56.25 },
  },
  mdm100: {
    label: "Monitor 100%",
    hint: "Igual distancia para llevar la mira al borde de la pantalla.",
    method: { kind: "monitorDistance", percent: 100 },
  },
};

interface Props {
  games: Game[];
  dpi: string;
  setDpi: (v: string) => void;
}

export default function Converter({ games, dpi, setDpi }: Props) {
  const [fromId, setFromId] = useState("valorant");
  const [toId, setToId] = useState("doom-eternal");
  const [sensForm, setSensForm] = useState<SensForm>({ main: "0.4", x: "", y: "0.4" });
  const [methodKey, setMethodKey] = useState<MethodKey>("distance360");
  const [aspectKey, setAspectKey] = useState("16:9");
  const [fromFov, setFromFov] = useState<number | null>(null);
  const [toFov, setToFov] = useState<number | null>(null);
  const [result, setResult] = useState<ConvertResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const from = games.find((g) => g.id === fromId);
  const to = games.find((g) => g.id === toId);

  useEffect(() => {
    if (!from || !to) return;
    const request: ConvertRequest = {
      fromGame: fromId,
      toGame: toId,
      sens: formToValues(from, sensForm),
      dpi: parseFloat(dpi),
      method: METHODS[methodKey].method,
      fromFov,
      toFov,
      aspect: ASPECTS[aspectKey],
    };
    invoke<ConvertResult>("convert", { request })
      .then((r) => {
        setResult(r);
        setError(null);
      })
      .catch((e) => {
        setResult(null);
        setError(String(e));
      });
  }, [from, to, fromId, toId, sensForm, dpi, methodKey, aspectKey, fromFov, toFov]);

  const changeFrom = (id: string) => {
    setFromId(id);
    setFromFov(null);
    setSensForm(defaultForm(games.find((g) => g.id === id)));
  };

  const swap = () => {
    setFromId(toId);
    setToId(fromId);
    setFromFov(toFov);
    setToFov(fromFov);
    if (result) setSensForm(valuesToForm(result.values));
  };

  const fovInput = (game: Game | undefined, value: number | null, set: (v: number | null) => void) =>
    game && (
      <label>
        FOV ({game.fov.type === "horizontal" ? "horizontal" : "vertical"})
        <input
          type="number"
          disabled={!game.fov.editable}
          min={game.fov.min}
          max={game.fov.max}
          value={value ?? game.fov.default}
          onChange={(e) => set(e.target.value === "" ? null : parseFloat(e.target.value))}
        />
      </label>
    );

  return (
    <>
      <section className="games">
        <div className="card">
          <h2>Juego origen</h2>
          <select value={fromId} onChange={(e) => changeFrom(e.target.value)}>
            {games.map((g) => <option key={g.id} value={g.id}>{g.name}</option>)}
          </select>
          <SensFields game={from} form={sensForm} onChange={setSensForm} />
          {fovInput(from, fromFov, setFromFov)}
        </div>

        <button className="swap" onClick={swap} title="Intercambiar juegos">⇄</button>

        <div className="card">
          <h2>Juego destino</h2>
          <select value={toId} onChange={(e) => { setToId(e.target.value); setToFov(null); }}>
            {games.map((g) => <option key={g.id} value={g.id}>{g.name}</option>)}
          </select>
          {fovInput(to, toFov, setToFov)}
          {to && <p className="note">{to.notes}</p>}
        </div>
      </section>

      <section className="card settings">
        <label>
          DPI del mouse
          <input type="number" value={dpi} onChange={(e) => setDpi(e.target.value)} />
        </label>
        <label>
          Aspect ratio
          <select value={aspectKey} onChange={(e) => setAspectKey(e.target.value)}>
            {Object.keys(ASPECTS).map((k) => <option key={k}>{k}</option>)}
          </select>
        </label>
        <div className="methods">
          {(Object.keys(METHODS) as MethodKey[]).map((k) => (
            <button key={k} className={k === methodKey ? "active" : ""} onClick={() => setMethodKey(k)}>
              {METHODS[k].label}
            </button>
          ))}
        </div>
        <p className="note">{METHODS[methodKey].hint}</p>
      </section>

      <section className="card result">
        {error && <p className="error">{error}</p>}
        {result && to && (
          <>
            <h2>Pon esto en {to.name}</h2>
            <SensOutput game={to} values={result.values} />
            <p className="note center">
              {Math.abs(result.errorPercent) < 0.005
                ? "Coincidencia exacta."
                : `Diferencia por precisión del menú: ${result.errorPercent.toFixed(2)} %`}
            </p>
            <dl>
              <dt>cm/360 origen</dt><dd>{result.cmPer360From.toFixed(2)} cm ({result.inchPer360From.toFixed(2)} in)</dd>
              <dt>cm/360 destino</dt><dd>{result.cmPer360To.toFixed(2)} cm</dd>
              <dt>FOV horizontal</dt><dd>{result.hfovFrom.toFixed(1)}° → {result.hfovTo.toFixed(1)}°</dd>
            </dl>
            {result.warnings.map((w) => <p key={w} className="warning">{w}</p>)}
          </>
        )}
      </section>
    </>
  );
}
