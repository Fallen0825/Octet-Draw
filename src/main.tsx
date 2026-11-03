import React from "react";
import ReactDOM from "react-dom/client";
// Brand fonts are bundled so the app works offline (OFL-1.1).
import "@fontsource/instrument-sans/400.css";
import "@fontsource/instrument-sans/500.css";
import "@fontsource/instrument-sans/600.css";
import "@fontsource/ibm-plex-mono/400.css";
import "@fontsource/ibm-plex-mono/500.css";
import "../brand/tokens.css"; // before App, which reads drawing tokens at load
import "./i18n/ko.css"; // 비공식 한글판: 한글 글꼴
import App from "./App";
import { watchErrors } from "./crashlog";
import { startKorean } from "./i18n/dom"; // 비공식 한글판

watchErrors();
startKorean();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
