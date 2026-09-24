import { useState, useEffect, useCallback } from 'react';

export interface BrowserContext {
  timestamp: string;
  timezone: string;
  latitude: number | null;
  longitude: number | null;
  location_name: string | null;
}

interface UseBrowserContextReturn {
  context: BrowserContext;
  error: string | null;
  permission: 'prompt' | 'granted' | 'denied' | 'unavailable';
  refresh: () => void;
}

function getTimezone(): string {
  try {
    return Intl.DateTimeFormat().resolvedOptions().timeZone;
  } catch {
    return 'UTC';
  }
}

function getTimestamp(): string {
  return new Date().toISOString();
}

const LOCATION_CACHE_KEY = 'alfred_location_cache';
const CACHE_TTL = 3600000; // 1 hour

interface LocationCache {
  latitude: number;
  longitude: number;
  location_name: string;
  timestamp: number;
}

function getCachedLocation(): LocationCache | null {
  try {
    const raw = localStorage.getItem(LOCATION_CACHE_KEY);
    if (!raw) return null;
    const cached = JSON.parse(raw) as LocationCache;
    if (Date.now() - cached.timestamp > CACHE_TTL) return null;
    return cached;
  } catch {
    return null;
  }
}

function setCachedLocation(lat: number, lng: number, name: string): void {
  try {
    localStorage.setItem(LOCATION_CACHE_KEY, JSON.stringify({
      latitude: lat,
      longitude: lng,
      location_name: name,
      timestamp: Date.now(),
    }));
  } catch {
    // localStorage may be full or unavailable
  }
}

async function reverseGeocode(lat: number, lng: number): Promise<string | null> {
  try {
    const response = await fetch(
      `https://nominatim.openstreetmap.org/reverse?lat=${lat}&lon=${lng}&format=json&addressdetails=1&accept-language=es`,
      { headers: { 'User-Agent': 'AlfredApp/1.0' } }
    );
    if (!response.ok) return null;
    const data = await response.json();
    return data.display_name || null;
  } catch {
    return null;
  }
}

export function useBrowserContext(): UseBrowserContextReturn {
  const [context, setContext] = useState<BrowserContext>(() => ({
    timestamp: getTimestamp(),
    timezone: getTimezone(),
    latitude: null,
    longitude: null,
    location_name: null,
  }));
  const [error, setError] = useState<string | null>(null);
  const [permission, setPermission] = useState<'prompt' | 'granted' | 'denied' | 'unavailable'>('prompt');

  const updateTimestamp = useCallback(() => {
    setContext(prev => ({ ...prev, timestamp: getTimestamp() }));
  }, []);

  const refresh = useCallback(() => {
    updateTimestamp();

    // Check cache first
    const cached = getCachedLocation();
    if (cached) {
      setContext(prev => ({
        ...prev,
        latitude: cached.latitude,
        longitude: cached.longitude,
        location_name: cached.location_name,
      }));
      setPermission('granted');
      return;
    }

    if (!navigator.geolocation) {
      setPermission('unavailable');
      return;
    }

    navigator.geolocation.getCurrentPosition(
      async (position) => {
        const { latitude, longitude } = position.coords;
        const name = await reverseGeocode(latitude, longitude);
        if (name) {
          setCachedLocation(latitude, longitude, name);
        }
        setContext(prev => ({
          ...prev,
          latitude,
          longitude,
          location_name: name,
        }));
        setPermission('granted');
        setError(null);
      },
      (err) => {
        setError(err.message);
        setPermission('denied');
      },
      { timeout: 10000, enableHighAccuracy: false }
    );
  }, [updateTimestamp]);

  useEffect(() => {
    refresh();
    // Update timestamp every minute
    const interval = setInterval(updateTimestamp, 60000);
    return () => clearInterval(interval);
  }, [refresh, updateTimestamp]);

  return { context, error, permission, refresh };
}