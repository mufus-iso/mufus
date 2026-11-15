import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./style.css";

function App() {
  return (
    <main>
      <header>
        <span className="brand">mufus</span>
        <span className="badge">Experimental</span>
      </header>
      <section aria-labelledby="title">
        <h1 id="title">Prepare boot media</h1>
        <p>
          mufus is in early development. Device discovery and image writing are
          not available yet.
        </p>
        <div className="status">
          <h2>Getting started</h2>
          <p>
            Device selection will appear here once safe disk discovery is
            implemented.
          </p>
        </div>
        <p className="note">
          No disks are accessed or modified by this version.
        </p>
      </section>
    </main>
  );
}

const root = document.getElementById("root");
if (!root) throw new Error("Missing application root");
createRoot(root).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
