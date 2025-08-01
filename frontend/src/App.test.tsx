import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import App from "./App";

describe("App", () => {
  it("renders the main heading", () => {
    render(<App />);
    expect(screen.getByText("家族用TODOアプリ")).toBeDefined();
  });

  it("displays the current development status", () => {
    render(<App />);
    expect(
      screen.getAllByText("現在の開発状況: 基盤セットアップ中")[0]
    ).toBeDefined();
  });
});
