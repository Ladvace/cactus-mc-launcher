import { describe, it, expect } from "vitest";
import { parseServerPort, formatAddress, DEFAULT_PORT } from "./serverAddress";

describe("parseServerPort", () => {
  it("defaults when there is no server-port line", () => {
    expect(parseServerPort("motd=hi\nmax-players=20")).toBe(DEFAULT_PORT);
  });
  it("reads an explicit port", () => {
    expect(parseServerPort("server-port=25580\nmotd=x")).toBe(25580);
  });
  it("ignores commented lines", () => {
    expect(parseServerPort("#server-port=1234\nserver-port=25570")).toBe(25570);
  });
  it("falls back on a non-numeric value", () => {
    expect(parseServerPort("server-port=abc")).toBe(DEFAULT_PORT);
  });
});

describe("formatAddress", () => {
  it("omits the default port", () => {
    expect(formatAddress("localhost", DEFAULT_PORT)).toBe("localhost");
  });
  it("appends a non-default port", () => {
    expect(formatAddress("192.168.1.5", 25570)).toBe("192.168.1.5:25570");
  });
});
