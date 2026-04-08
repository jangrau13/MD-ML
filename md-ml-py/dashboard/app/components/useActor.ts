"use client";

import useSWR from "swr";
import { ActorState } from "./types";

const fetcher = (url: string) =>
  fetch(url).then((r) => {
    if (!r.ok) throw new Error(`${r.status}`);
    return r.json();
  });

export function useActor(baseUrl: string) {
  const { data, error, isLoading } = useSWR<ActorState>(
    `${baseUrl}/api/state`,
    fetcher,
    { refreshInterval: 400, revalidateOnFocus: false }
  );

  const triggerStep = async () => {
    await fetch(`${baseUrl}/api/step`, { method: "POST" });
  };

  return { state: data ?? null, error, isLoading, triggerStep };
}
