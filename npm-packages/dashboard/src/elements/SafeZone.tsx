import { useRouter } from "next/router";
import { useMouse } from "react-use";

export function SafeZone({
  anchor,
  submenu,
  setIsInSafeZone,
}: {
  anchor: HTMLElement;
  submenu: HTMLElement;
  setIsInSafeZone: (isInSafeZone: boolean) => void;
}) {
  const router = useRouter();
  const showTriangle = router.query.showSafetyTriangle;
  const { docX: mouseX, docY: mouseY } = useMouse({
    current: anchor,
  });

  const { x: anchorX, y: anchorY } = anchor.getBoundingClientRect();
  const {
    height: submenuHeight,
    x: submenuX,
    y: submenuY,
  } = submenu.getBoundingClientRect();

  const svgWidth = submenuX - mouseX;
  const svgHeight = submenuHeight;

  return (
    <svg
      style={{
        position: "fixed",
        width: svgWidth,
        height: submenuHeight,
        cursor: "pointer",
        pointerEvents: "none",
        top: submenuY - anchorY,
        left: mouseX - anchorX + 1,
      }}
      onMouseEnter={() => setIsInSafeZone(true)}
      onMouseLeave={() => setIsInSafeZone(false)}
    >
      <path
        pointerEvents="auto"
        fill={showTriangle ? "rgba(114,140,89,0.3)" : "transparent"}
        d={`M 0, ${mouseY - submenuY} 
            L ${svgWidth},${svgHeight}  
            L ${svgWidth},0 z`}
      />
    </svg>
  );
}
