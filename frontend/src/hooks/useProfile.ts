import { useState, useCallback, useEffect } from 'react';
import type { Profile, UpdateProfile } from '../types';
import { api } from '../api/client';

export function useProfile() {
  const [profile, setProfile] = useState<Profile | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let mounted = true;
    api.getProfile()
      .then(p => { if (mounted) setProfile(p); })
      .catch(err => { if (mounted) setError(err.message); })
      .finally(() => { if (mounted) setLoading(false); });
    return () => { mounted = false; };
  }, []);

  const updateProfile = useCallback(async (data: UpdateProfile) => {
    setLoading(true);
    try {
      const updated = await api.updateProfile(data);
      setProfile(updated);
      setError(null);
    } catch (err: any) {
      setError(err.message);
      throw err;
    } finally {
      setLoading(false);
    }
  }, []);

  return { profile, loading, error, updateProfile };
}