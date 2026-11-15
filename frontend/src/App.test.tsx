import {
  act,
  fireEvent,
  render,
  screen,
  cleanup,
} from "@testing-library/react";
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

test("keeps focus and prevents duplicate checks while one is pending", async () => {
  let complete!: () => void;
  invoke.mockReturnValueOnce(
    new Promise<void>((resolve) => {
      complete = resolve;
    }),
  );
  render(<App />);
  const button = screen.getByRole("button", { name: "Check disks" });
  button.focus();
  fireEvent.click(button);
  expect(document.activeElement).toBe(button);
  expect(button.getAttribute("aria-disabled")).toBe("true");
  expect(screen.getByRole("status").textContent).toContain(
    "Checking disk information",
  );
  fireEvent.click(button);
  expect(invoke).toHaveBeenCalledTimes(1);
  await act(async () => complete());
  expect(button.getAttribute("aria-disabled")).toBe("false");
  expect(screen.getByRole("status").textContent).toContain(
    "Disk information was read",
  );
});
