import React from "react";
import { createRoot } from "react-dom/client";
import "katex/dist/katex.min.css";
import "./styles.css";
import App from "./App";

const root = document.getElementById("root");
if (!root) throw new Error("#root not found");

createRoot(root).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
