import "@testing-library/jest-dom";
import { vi } from "vitest";

// Mock AWS Amplify
vi.mock("aws-amplify", () => ({
  Amplify: {
    configure: vi.fn(),
  },
}));

// Mock environment variables
Object.defineProperty(window, "location", {
  value: {
    href: "http://localhost:3000",
    origin: "http://localhost:3000",
  },
  writable: true,
});
