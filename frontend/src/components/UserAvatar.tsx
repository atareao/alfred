import React, { useState } from "react";
import { UserOutlined } from "@ant-design/icons";

export interface UserAvatarProps {
  src?: string | null;
  size?: number;
}

export const UserAvatar: React.FC<UserAvatarProps> = ({ src, size = 24 }) => {
  const [failedSrc, setFailedSrc] = useState<string | null>(null);

  const url = src?.trim() || null;

  if (url === null || url === failedSrc) {
    return <UserOutlined style={{ color: "#1677ff", fontSize: size }} />;
  }

  return (
    <img
      src={url}
      alt="Usuario"
      width={size}
      height={size}
      loading="lazy"
      referrerPolicy="no-referrer"
      onError={() => setFailedSrc(url)}
      style={{ display: "block", borderRadius: "50%", objectFit: "cover" }}
    />
  );
};
