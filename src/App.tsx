import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Game } from "./types";
import Converter from "./Converter";
import Calibrator from "./Calibrator";
import "./App.css";

type Tab = "convert" | "calibrate";

export default function App() {
  const [games, setGames] = useState<Game[]>([]);
  const [dpi, setDpi] = useState("800");
  const [tab, setTab] = useState<Tab>("convert");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<Game[]>("list_games").then(setGames).catch((e) => setError(String(e)));
  }, []);

  return (
    <main className="app">
      <header>
        <h1>AimChanger</h1>
        <p>Lleva tu sensibilidad de un juego a otro con la misma sensación.</p>
      </header>

      <nav className="tabs">
        <button className={tab === "convert" ? "active" : ""} onClick={() => setTab("convert")}>
          Convertir
        </button>
        <button className={tab === "calibrate" ? "active" : ""} onClick={() => setTab("calibrate")}>
          Prueba 360°
        </button>
      </nav>

      {error && <p className="error">{error}</p>}
      {tab === "convert" ? (
        <Converter games={games} dpi={dpi} setDpi={setDpi} />
      ) : (
        <Calibrator games={games} dpi={dpi} setDpi={setDpi} />
      )}
    </main>
  );
}
