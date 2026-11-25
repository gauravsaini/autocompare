import React from "react";
import ReactDOM from "react-dom/client";
import V2App from "./v2/V2App";
import "./v2/v2.css";
import "./theme/tokens.css";

const root = document.getElementById("root");
const useV1 =
  typeof window !== "undefined" &&
  new URLSearchParams(window.location.search).get("v1") === "1";

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
