import React from "react";
import ReactDOM from "react-dom/client";
import "@fontsource/inter/400.css";
import "@fontsource/inter/500.css";
import "@fontsource/inter/600.css";
import "@fontsource/inter/700.css";
import "@fontsource/inter/800.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/500.css";
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
