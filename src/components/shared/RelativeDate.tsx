type RelativeDateProps = {
  value?: string | null;
  fallback?: string;
  now?: Date;
};

const DAY_MS = 24 * 60 * 60 * 1000;

export function RelativeDate({
  value,
  fallback = "—",
  now = new Date(),
}: RelativeDateProps) {
  if (!value) return <span>{fallback}</span>;
  const date = new Date(value);
  const startToday = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const startValue = new Date(
    date.getFullYear(),
    date.getMonth(),
    date.getDate(),
  );
  const dayDiff = Math.round((startToday.getTime() - startValue.getTime()) / DAY_MS);
  const time = date.toLocaleTimeString("pt-BR", { hour12: false });
  let label: string;
  if (dayDiff === 0) label = `Hoje ${time}`;
  else if (dayDiff === 1) label = `Ontem ${time}`;
  else label = date.toLocaleString("pt-BR");
  return <time dateTime={value}>{label}</time>;
}