import React from "react";
import ReactDOM from "react-dom/client";
import V2App from "./v2/V2App";
import "./v2/v2.css";
import "./theme/tokens.css";

const root = document.getElementById("root");
const params =
  typeof window !== "undefined" ? new URLSearchParams(window.location.search) : null;
const envVersion = import.meta.env.VITE_UI_VERSION;
const useV1 =
  params?.get("v2") !== "1" && (params?.get("v1") === "1" || envVersion === "v1");

const LazyV1App = React.lazy(() => import("./App"));

if (root !== null) {
  ReactDOM.createRoot(root).render(
    <React.StrictMode>
      {useV1 ? (
        <React.Suspense fallback={null}>
          <LazyV1App />
        </React.Suspense>
      ) : (
        <V2App />
      )}
    </React.StrictMode>,
  );
}
