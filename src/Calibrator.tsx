import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { CalibrateRequest, CalibrateResult, CaptureStatus, Game } from "./types";
import SensFields, { defaultForm, formToValues, type SensForm } from "./SensFields";

interface Props {
  games: Game[];
  dpi: string;
  setDpi: (v: string) => void;
}

const NEW_GAME = "__nuevo__";
const TURNS = [1, 3, 5, 10];
/// Por debajo de este error (%) consideramos que el yaw de la base de datos es correcto.
const TOLERANCE = 1.5;

export default function Calibrator({ games, dpi, setDpi }: Props) {
  const [gameId, setGameId] = useState("valorant");
  const [sensForm, setSensForm] = useState<SensForm>({ main: "0.4", x: "", y: "0.4" });
  const [turns, setTurns] = useState(3);
  const [armed, setArmed] = useState(false);
  const [status, setStatus] = useState<CaptureStatus | null>(null);
  const [result, setResult] = useState<CalibrateResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const wasRecording = useRef(false);

  // Desarmar (soltar F8) al salir de la pestaña.
  useEffect(() => () => void invoke("calibration_arm", { armed: false }), []);

  // Mientras está armado, consultar el estado para mostrar los counts en vivo.
  useEffect(() => {
    if (!armed) return;
    const id = setInterval(() => {
      invoke<CaptureStatus>("capture_status").then(setStatus).catch(() => {});
    }, 100);
    const unlisten = listen<CaptureStatus>("capture-toggled", (e) => setStatus(e.payload));
    return () => {
      clearInterval(id);
      unlisten.then((f) => f());
    };
  }, [armed]);

  // Al terminar una grabación, calcular el resultado.
  useEffect(() => {
    if (!status) return;
    if (status.recording) {
      setResult(null);
      setError(null);
    } else if (wasRecording.current) {
      const request: CalibrateRequest = {
        gameId: gameId === NEW_GAME ? null : gameId,
        sens: formToValues(games.find((g) => g.id === gameId), sensForm),
        dpi: parseFloat(dpi),
        turns,
        counts: status.countsX,
      };
      invoke<CalibrateResult>("calibrate", { request })
        .then(setResult)
        .catch((e) => setError(String(e)));
    }
    wasRecording.current = status.recording;
    // Solo reaccionar a cambios de estado de la captura.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [status?.recording]);

  const changeGame = (id: string) => {
    setGameId(id);
    setSensForm(defaultForm(games.find((g) => g.id === id)));
    setResult(null);
  };

  const toggleArm = async () => {
    try {
      await invoke("calibration_arm", { armed: !armed });
      setArmed(!armed);
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  };

  const toggleRecording = () =>
    invoke<CaptureStatus>("capture_toggle").then(setStatus).catch((e) => setError(String(e)));

  const game = games.find((g) => g.id === gameId);
  const ok = result?.errorPercent != null && Math.abs(result.errorPercent) <= TOLERANCE;

  return (
    <>
      <section className="card">
        <h2>Prueba de 360°</h2>
        <p className="note">
          Mide cuánto gira un juego por cada movimiento del mouse. Sirve para comprobar que los datos
          de un juego son correctos o para calibrar uno que no está en la lista. Solo lee tu mouse,
          no envía input al juego.
        </p>
        <div className="settings">
          <label>
            Juego
            <select value={gameId} onChange={(e) => changeGame(e.target.value)}>
              {games.map((g) => <option key={g.id} value={g.id}>{g.name}</option>)}
              <option value={NEW_GAME}>Otro juego (sin datos)</option>
            </select>
          </label>
          <div className="span-all">
            <p className="note">La sensibilidad que tienes ahora en ese juego:</p>
            <SensFields game={game} form={sensForm} onChange={setSensForm} />
          </div>
          <label>
            DPI del mouse
            <input type="number" value={dpi} onChange={(e) => setDpi(e.target.value)} />
          </label>
          <label>
            Vueltas a dar
            <select value={turns} onChange={(e) => setTurns(Number(e.target.value))}>
              {TURNS.map((t) => <option key={t} value={t}>{t} {t === 1 ? "vuelta" : "vueltas"}</option>)}
            </select>
          </label>
        </div>
        <ol className="steps">
          <li>Pulsa <b>Activar prueba</b> y entra al juego (campo de práctica, sin enemigos).</li>
          <li>Apunta a un punto fijo y fácil de reconocer, como el borde de una pared.</li>
          <li>Pulsa <kbd>F8</kbd> (pitido agudo) y gira <b>despacio</b> hacia un solo lado {turns} {turns === 1 ? "vuelta completa" : "vueltas completas"}.</li>
          <li>Cuando la mira vuelva exactamente al punto, pulsa <kbd>F8</kbd> otra vez (pitido grave).</li>
        </ol>
        <div className="methods">
          <button className={armed ? "active" : ""} onClick={toggleArm}>
            {armed ? "Desactivar prueba" : "Activar prueba (F8)"}
          </button>
          {armed && (
            <button onClick={toggleRecording}>
              {status?.recording ? "Detener" : "Iniciar"} manualmente
            </button>
          )}
        </div>
      </section>

      {armed && (
        <section className="card">
          <div className="live">
            <span className={status?.recording ? "dot recording" : "dot"} />
            <span>{status?.recording ? "Grabando…" : "En espera (pulsa F8)"}</span>
            <strong>{status?.countsX ?? 0}</strong>
            <small>movimientos en X</small>
          </div>
        </section>
      )}

      {(result || error) && (
        <section className="card result">
          {error && <p className="error">{error}</p>}
          {result && (
            <>
              <div className="big">
                <span>Yaw medido</span>
                <strong>{result.measuredYaw.toFixed(5)}</strong>
                <small>grados por movimiento con sensibilidad 1</small>
              </div>
              <dl>
                <dt>Movimientos por 360°</dt><dd>{result.countsPer360.toFixed(0)}</dd>
                <dt>cm/360</dt><dd>{result.cmPer360.toFixed(2)} cm</dd>
                {result.expectedYaw != null && game && (
                  <>
                    <dt>Yaw en la base de datos</dt><dd>{result.expectedYaw}</dd>
                    <dt>Movimientos esperados</dt><dd>{result.expectedCounts?.toFixed(0)} ({turns}×360°)</dd>
                    <dt>Diferencia</dt><dd>{result.errorPercent!.toFixed(2)} %</dd>
                  </>
                )}
              </dl>
              {result.errorPercent != null &&
                (ok ? (
                  <p className="success">La medición coincide con los datos de {game?.name}.</p>
                ) : (
                  <p className="warning">
                    Diferencia mayor a {TOLERANCE}%. Revisa que la sensibilidad sea la correcta, que el
                    juego no tenga aceleración del mouse y que diste exactamente {turns}{" "}
                    {turns === 1 ? "vuelta" : "vueltas"}. Repite con más vueltas para reducir el error.
                  </p>
                ))}
            </>
          )}
        </section>
      )}
    </>
  );
}
