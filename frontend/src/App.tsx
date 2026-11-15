import { invoke } from "@tauri-apps/api/core";
import { useState } from "react";

export function App() {
  const [state, setState] = useState<"idle" | "checking" | "done" | "error">(
    "idle",
  );
  const [error, setError] = useState("");

  async function checkDisks() {
    setState("checking");
    try {
      await invoke("check_disk_discovery");
      setState("done");
    } catch (reason: unknown) {
      setError(
        typeof reason === "string"
          ? reason
          : "The disk check failed. Please try again.",
      );
      setState("error");
    }
  }

  return (
    <main>
      <header>
        <span className="brand">mufus</span>
        <span className="badge">Experimental</span>
      </header>
      <section aria-labelledby="title">
        <h1 id="title">Prepare boot media</h1>
        <p>
          mufus is in early development. Device selection and image writing are
          not available yet.
        </p>
        <div className="status">
          <h2>Read-only disk check</h2>
          <p>
            Check whether mufus can read disk information on this computer. No
            device is selected or modified.
          </p>
          <button
            type="button"
            onClick={checkDisks}
            disabled={state === "checking"}
          >
            {state === "checking" ? "Checking…" : "Check disks"}
          </button>
          <div role="status" aria-live="polite">
            {state === "done" && (
              <p>
                Disk information was read. Device selection is not available
                yet.
              </p>
            )}
            {state === "error" && <p>{error}</p>}
          </div>
        </div>
        <p className="note">This version does not write to disks.</p>
      </section>
    </main>
  );
}
