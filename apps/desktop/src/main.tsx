import React from "react";
import ReactDOM from "react-dom/client";
import "@fontsource-variable/inter";
import "./index.css";
import { App } from "@/app/App";
import { installGlobalErrorLogging } from "@/lib/log";
import { bootLanguage } from "@/i18n/boot";
import { bootWindowChrome } from "@/lib/windowChrome";

installGlobalErrorLogging();

// O material da janela e o idioma são lidos antes do primeiro render para nada piscar.
void Promise.all([bootWindowChrome(), bootLanguage()]).then(() => {
  ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
      <App />
    </React.StrictMode>,
  );
});
