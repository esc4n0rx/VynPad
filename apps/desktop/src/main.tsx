import React from "react";
import ReactDOM from "react-dom/client";
import "@fontsource-variable/inter";
import "./index.css";
import { App } from "@/app/App";
import { installGlobalErrorLogging } from "@/lib/log";
import { bootWindowChrome } from "@/lib/windowChrome";

installGlobalErrorLogging();

// O material da janela é lido antes do primeiro render para o fundo não piscar.
void bootWindowChrome().then(() => {
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
});
