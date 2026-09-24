import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, waitFor } from '@testing-library/react';
import { useBrowserContext } from '../hooks/useBrowserContext';

// ---------------------------------------------------------------------------
// Mocks for browser APIs that don't exist in jsdom
// ---------------------------------------------------------------------------

const mockGetCurrentPosition = vi.fn();
const mockFetch = vi.fn();

beforeEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
  // Stub navigator.geolocation
  Object.defineProperty(navigator, 'geolocation', {
    value: { getCurrentPosition: mockGetCurrentPosition },
    writable: true,
    configurable: true,
  });
  // Stub global fetch for Nominatim reverse geocode
  vi.stubGlobal('fetch', mockFetch);
});

describe('useBrowserContext', () => {
  // -------------------------------------------------------------------------
  // Scenario: Browser context includes timestamp and timezone
  //   Given the hook mounts
  //   When  the context is read
  //   Then  context.timestamp is a valid ISO string and
  //         context.timezone is not empty
  // -------------------------------------------------------------------------
  it('returns timestamp and timezone when mounted', async () => {
    // This will FAIL in RED phase because useBrowserContext does not exist yet
    const { result } = renderHook(() => useBrowserContext());

    // timestamp should be an ISO 8601 string (e.g., "2026-09-24T...")
    expect(result.current.context.timestamp).toBeTruthy();
    expect(typeof result.current.context.timestamp).toBe('string');
    expect(() => new Date(result.current.context.timestamp)).not.toThrow();

    // timezone should be a non-empty IANA timezone string (e.g., "Europe/Madrid")
    expect(result.current.context.timezone).toBeTruthy();
    expect(typeof result.current.context.timezone).toBe('string');
    expect(result.current.context.timezone.length).toBeGreaterThan(0);
  });

  // -------------------------------------------------------------------------
  // Scenario: Browser context exposes permission state and refresh function
  //   Given the hook mounts
  //   When  the return value is inspected
  //   Then  permission is one of the allowed values and refresh is a function
  // -------------------------------------------------------------------------
  it('exposes permission state and refresh function', async () => {
    const { result } = renderHook(() => useBrowserContext());

    expect(['prompt', 'granted', 'denied', 'unavailable']).toContain(
      result.current.permission,
    );
    expect(typeof result.current.refresh).toBe('function');
  });

  // -------------------------------------------------------------------------
  // Scenario: Geolocation denied
  //   Given the user denies geolocation permission
  //   When  the error callback fires
  //   Then  context.latitude is null, context.longitude is null and
  //         location_name is null
  // -------------------------------------------------------------------------
  it('returns null location when geolocation is denied', async () => {
    mockGetCurrentPosition.mockImplementationOnce(
      (_success: PositionCallback, error: PositionErrorCallback) => {
        const err = new Error('denied') as unknown as GeolocationPositionError & { code: number };
        err.code = 1;
        error(err as unknown as GeolocationPositionError);
      },
    );

    const { result } = renderHook(() => useBrowserContext());

    await waitFor(() => {
      expect(result.current.context.latitude).toBeNull();
      expect(result.current.context.longitude).toBeNull();
      expect(result.current.context.location_name).toBeNull();
    });
  });

  // -------------------------------------------------------------------------
  // Scenario: Geolocation granted
  //   Given the user grants geolocation permission
  //   When  the position is obtained and reverse geocoded
  //   Then  context.latitude and context.longitude are numbers, and
  //         location_name is a non-empty string from Nominatim
  // -------------------------------------------------------------------------
  it('returns coordinates and location name when geolocation succeeds', async () => {
    mockGetCurrentPosition.mockImplementationOnce(
      (success: PositionCallback) => {
        success({
          coords: {
            latitude: 39.36,
            longitude: -0.41,
            accuracy: 10,
            altitude: null,
            altitudeAccuracy: null,
            heading: null,
            speed: null,
          },
          timestamp: Date.now(),
        } as GeolocationPosition);
      },
    );

    // Mock Nominatim reverse geocode response
    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: () =>
        Promise.resolve({
          display_name: 'Silla, Valencia, España',
          address: { city: 'Silla' },
        }),
    });

    const { result } = renderHook(() => useBrowserContext());

    await waitFor(() => {
      expect(result.current.context.latitude).toBe(39.36);
      expect(result.current.context.longitude).toBe(-0.41);
    });

    await waitFor(() => {
      expect(result.current.context.location_name).toContain('Silla');
    });

    // Nominatim should have been called with the right lat/lon
    expect(mockFetch).toHaveBeenCalledWith(
      expect.stringContaining('nominatim.openstreetmap.org/reverse'),
      expect.any(Object),
    );
  });

  // -------------------------------------------------------------------------
  // Scenario: Geolocation unavailable (no API)
  //   Given the browser does not support geolocation
  //   When  the hook tries to access geolocation
  //   Then  permission is 'unavailable' and location fields are null
  // -------------------------------------------------------------------------
  it('handles missing geolocation API gracefully', async () => {
    // Remove geolocation entirely
    Object.defineProperty(navigator, 'geolocation', {
      value: undefined,
      writable: true,
      configurable: true,
    });

    const { result } = renderHook(() => useBrowserContext());

    await waitFor(() => {
      expect(result.current.permission).toBe('unavailable');
      expect(result.current.context.latitude).toBeNull();
      expect(result.current.context.longitude).toBeNull();
      expect(result.current.context.location_name).toBeNull();
    });
  });

  // -------------------------------------------------------------------------
  // Scenario: refresh() re-fetches geolocation
  //   Given the hook is mounted with a denied state
  //   When  refresh() is called
  //   Then  getCurrentPosition is called again
  // -------------------------------------------------------------------------
  it('calls getCurrentPosition again when refresh is invoked', async () => {
    // First call denies
    mockGetCurrentPosition.mockImplementationOnce(
      (_success: PositionCallback, error: PositionErrorCallback) => {
        const err = new Error('denied') as unknown as GeolocationPositionError & { code: number };
        err.code = 1;
        error(err as unknown as GeolocationPositionError);
      },
    );

    const { result } = renderHook(() => useBrowserContext());

    await waitFor(() => {
      expect(result.current.permission).toBe('denied');
    });

    // Reset mock to succeed on next call
    mockGetCurrentPosition.mockImplementationOnce(
      (success: PositionCallback) => {
        success({
          coords: { latitude: 40.41, longitude: -3.70, accuracy: 10 },
          timestamp: Date.now(),
        } as GeolocationPosition);
      },
    );

    mockFetch.mockResolvedValueOnce({
      ok: true,
      json: () =>
        Promise.resolve({
          display_name: 'Madrid, España',
          address: { city: 'Madrid' },
        }),
    });

    // Call refresh
    await waitFor(() => {
      result.current.refresh();
    });

    await waitFor(() => {
      expect(result.current.context.latitude).toBe(40.41);
      expect(result.current.context.longitude).toBe(-3.70);
    });
    expect(mockGetCurrentPosition).toHaveBeenCalledTimes(2);
  });
});