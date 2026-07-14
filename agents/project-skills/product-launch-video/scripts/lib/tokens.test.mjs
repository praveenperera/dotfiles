import assert from "node:assert/strict";
import { test } from "node:test";
import { semanticColors, STATUS_ROLE_KEY } from "./tokens.mjs";

test("semantic status roles are never selected as brand accents", () => {
  const colors = [
    ["ink", "#111827"],
    ["canvas", "#ffffff"],
    ["negative", "#dc2626"],
    ["brand", "#1e40af"],
  ];

  assert.deepEqual(semanticColors(colors), {
    ink: "#111827",
    canvas: "#ffffff",
    accent: "#1e40af",
    accent2: "#1e40af",
  });
});

test("status-role matching covers informational and critical variants", () => {
  for (const key of ["info", "neutral-bg", "alert_text", "caution", "critical-fill"]) {
    assert.match(key, STATUS_ROLE_KEY);
  }
});
