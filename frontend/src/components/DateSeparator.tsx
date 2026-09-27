import React from "react";

interface DateSeparatorProps {
  date: string; // ISO string
}

function formatDateLabel(isoDate: string): string {
  const date = new Date(isoDate);
  const now = new Date();

  // Normalize to start of day for comparison
  const dateStart = new Date(date.getFullYear(), date.getMonth(), date.getDate());
  const todayStart = new Date(now.getFullYear(), now.getMonth(), now.getDate());

  const diffDays = Math.round((todayStart.getTime() - dateStart.getTime()) / 86400000);

  if (diffDays === 0) return "Hoy";
  if (diffDays === 1) return "Ayer";

  // Format: "25 sept 2026"
  const months = ["ene", "feb", "mar", "abr", "may", "jun", "jul", "ago", "sept", "oct", "nov", "dic"];
  return `${date.getDate()} ${months[date.getMonth()]} ${date.getFullYear()}`;
}

export const DateSeparator: React.FC<DateSeparatorProps> = ({ date }) => {
  const label = formatDateLabel(date);

  return (
    <div
      style={{
        display: "flex",
        alignItems: "center",
        gap: 12,
        margin: "16px 0",
        opacity: 0.4,
        borderBottom: "1px solid rgba(255,255,255,0.2)",
      }}
    >
      <span style={{ fontSize: 11, color: "rgba(255,255,255,0.5)", whiteSpace: "nowrap" }}>
        {label}
      </span>
    </div>
  );
};