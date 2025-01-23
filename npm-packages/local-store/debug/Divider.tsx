export function Divider({
  style,
  children,
}: {
  style?: string;
  children?: React.ReactNode;
}) {
  return (
    <div className="flex items-center gap-2 w-full">
      <div className={`border h-[1px] flex-grow ${style}`} />
      {children}
    </div>
  );
}
