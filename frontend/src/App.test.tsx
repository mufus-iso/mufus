import { fireEvent, render, screen, cleanup } from "@testing-library/react";
import { afterEach, expect, test, vi } from "vitest";
import { App } from "./App";

const invoke = vi.hoisted(() => vi.fn());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
afterEach(() => {
  cleanup();
  invoke.mockReset();
});

test("reports a failed disk check and allows retry", async () => {
  invoke
    .mockRejectedValueOnce("The disk check failed.")
    .mockResolvedValueOnce(undefined);
  render(<App />);
  fireEvent.click(screen.getByRole("button", { name: "Check disks" }));
  expect(await screen.findByText("The disk check failed.")).toBeTruthy();
  fireEvent.click(screen.getByRole("button", { name: "Check disks" }));
  expect(await screen.findByText(/Disk information was read/)).toBeTruthy();
  expect(invoke).toHaveBeenCalledWith("check_disk_discovery");
});
