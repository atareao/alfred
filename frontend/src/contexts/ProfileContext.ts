import { createContext, useContext } from "react";
import type { Profile, UpdateProfile } from "../types";

export interface ProfileContextValue {
  profile: Profile | null;
  loading: boolean;
  error: string | null;
  updateProfile: (data: UpdateProfile) => Promise<void>;
}

export const ProfileContext = createContext<ProfileContextValue | null>(null);

export function useProfileContext(): ProfileContextValue {
  const context = useContext(ProfileContext);
  if (context === null) {
    throw new Error(
      "useProfileContext debe usarse dentro de un <ProfileProvider>",
    );
  }
  return context;
}
