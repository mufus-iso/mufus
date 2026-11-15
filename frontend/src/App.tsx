import { invoke } from "@tauri-apps/api/core";
import { useRef, useState } from "react";

export function App() {
  const [state, setState] = useState<"idle" | "checking" | "done" | "error">(
    "idle",
  );
  const [error, setError] = useState("");
  const checking = useRef(false);

  async function checkDisks() {
    if (checking.current) return;
    checking.current = true;
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
    } finally {
      checking.current = false;
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
            aria-disabled={state === "checking"}
          >
            Check disks
          </button>
          <div role="status">
            {state === "checking" && <p>Checking disk information…</p>}
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
