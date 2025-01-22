import { useEffectOnce } from "react-use";

export function MockMonaco({
  onChange,
  defaultValue,
  value,
  beforeMount,
  path,
}: {
  onChange: (v: string) => void;
  defaultValue: string;
  value: string;
  beforeMount?: (monaco: any) => void;
  path: string;
}) {
  useEffectOnce(() => {
    beforeMount &&
      beforeMount({
        MarkerSeverity: { Error: 8, Hint: 1 },
        editor: {
          getModels: () => [{ uri: { path: `/${path}` } }],
          setModelMarkers: jest.fn(),
        },
        languages: {
          typescript: {
            javascriptDefaults: { setDiagnosticsOptions: () => {} },
          },
        },
      });
  });
  return (
    <input
      data-testid="mockMonaco"
      defaultValue={defaultValue}
      value={value}
      onChange={(e) => {
        onChange(e.target.value);
      }}
    />
  );
}
