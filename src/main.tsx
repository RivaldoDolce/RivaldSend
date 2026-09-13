import React from "react";
import ReactDOM from "react-dom/client";
// Sous-ensembles latins seuls : les graisses complètes embarquaient
// cyrillique, grec et vietnamien pour rien (~200 Ko économisés).
import "@fontsource/inter/latin-400.css";
import "@fontsource/inter/latin-500.css";
import "@fontsource/inter/latin-600.css";
import "@fontsource/inter/latin-700.css";
import "@fontsource/inter/latin-800.css";
import "@fontsource/jetbrains-mono/latin-400.css";
import "@fontsource/jetbrains-mono/latin-500.css";
import App from "./App";
import "./index.css";

if (navigator.hardwareConcurrency <= 4 || matchMedia("(prefers-reduced-motion: reduce)").matches) {
  document.documentElement.classList.add("perf-lite");
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
