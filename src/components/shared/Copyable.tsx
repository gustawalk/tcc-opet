import { toast } from "sonner";

type CopyableProps = {
  label: string;
  text?: string;
};

export function Copyable({ label, text }: CopyableProps) {
  const handleClick = () => {
    const content = text ?? label;
    navigator.clipboard.writeText(content).then(() => {
      toast.success("Copiado: " + content);
    });
  };

  return (
    <span
      className="cursor-pointer hover:text-primary transition-colors"
      onClick={handleClick}
      title="Clique para copiar"
    >
      {label || "-"}
    </span>
  );
}
