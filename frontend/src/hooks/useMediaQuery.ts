import { useState, useEffect, useCallback } from "react";

export function useMediaQuery(query: string): boolean {
  const getMatches = useCallback((): boolean => {
    if (typeof window !== "undefined") {
      return window.matchMedia(query).matches;
    }
    return false;
  }, [query]);

  const [matches, setMatches] = useState(getMatches);

  useEffect(() => {
    const mql = window.matchMedia(query);
    const handler = () => setMatches(getMatches());
    mql.addEventListener("change", handler);
    return () => mql.removeEventListener("change", handler);
  }, [query, getMatches]);

  return matches;
}
