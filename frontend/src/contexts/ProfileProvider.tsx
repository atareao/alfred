import React from "react";
import { ProfileContext } from "./ProfileContext";
import { useProfile } from "../hooks/useProfile";

export const ProfileProvider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  const value = useProfile();
  return (
    <ProfileContext.Provider value={value}>{children}</ProfileContext.Provider>
  );
};
