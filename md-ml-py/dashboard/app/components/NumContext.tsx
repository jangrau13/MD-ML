"use client";

import { createContext, useContext } from "react";

const HexModeContext = createContext(false);

export const HexModeProvider = HexModeContext.Provider;

export function useHexMode() {
  return useContext(HexModeContext);
}

/**
 * Renders a number/string value, automatically switching between
 * decimal and hexadecimal based on the global hex/dec toggle.
 *
 * - Floats are always shown as decimal (hex doesn't make sense).
 * - Integer strings (from BigInt) are converted to hex when active.
 * - Plain numbers that look like integers are converted too.
 */
export function Num({
  value,
  className,
}: {
  value: string | number;
  className?: string;
}) {
  const hex = useHexMode();
  return <span className={className}>{formatNum(value, hex)}</span>;
}

export function formatNum(value: string | number, hex: boolean): string {
  if (!hex) return String(value);

  // Float number — keep as decimal
  if (typeof value === "number") {
    if (!Number.isInteger(value)) return String(value);
    return "0x" + value.toString(16);
  }

  // String value — try to parse as BigInt for hex
  try {
    return "0x" + BigInt(value).toString(16);
  } catch {
    return value;
  }
}
