import icon from "@/icons/icon.svg?raw";

type LogoIconProps = React.HTMLAttributes<HTMLSpanElement> & {
  width?: number | string;
  height?: number | string;
};

export function LogoIcon({ width = 24, height = 24, style, ...props }: LogoIconProps) {
  const iconMarkup = icon
    .replace(/fill="#[A-Fa-f0-9]{6}"/g, 'fill="currentColor"')
    .replace(/width="\d+"/, 'width="100%"')
    .replace(/height="\d+"/, 'height="100%"');

  return (
    <span
      {...props}
      style={{
        width,
        height,
        display: "block",
        color: "inherit",
        lineHeight: 0,
        ...style,
      }}
    >
      <span
        aria-hidden="true"
        style={{
          display: "block",
          width: "100%",
          height: "100%",
          transform: "scale(1.5)",
          transformOrigin: "center",
        }}
        dangerouslySetInnerHTML={{ __html: iconMarkup }}
      />
    </span>
  );
}
