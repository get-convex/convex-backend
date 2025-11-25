import { useWindowSize } from "react-use";
import { useEffect, useState } from "react";
import FlourishTop from "./images/flourish-top.svg";
import FlourishBottom from "./images/flourish-bottom.svg";
import FlourishBottomRight from "./images/flourish-bottom-right.svg";
import FlourishRight from "./images/flourish-right.svg";
import FlourishLeft from "./images/flourish-left.svg";

export function Flourish() {
  const [mounted, setMounted] = useState(false);
  useEffect(() => {
    setMounted(true);
  }, []);

  const { height } = useWindowSize();

  return mounted && height > 500 ? (
    <div className="hidden md:block dark:hidden">
      <div className="absolute top-0 left-1/2 -translate-x-1/2 translate-y-[-20%]">
        <FlourishTop />
      </div>
      <div className="absolute bottom-0 left-1/2 -translate-x-1/2">
        <FlourishBottom />
      </div>
      <div className="absolute right-0 bottom-[35%]">
        <FlourishRight />
      </div>
      <div className="absolute bottom-[20%] left-0 -translate-y-1/2">
        <FlourishLeft />
      </div>
      <div className="absolute right-[8%] bottom-0">
        <FlourishBottomRight />
      </div>
    </div>
  ) : null;
}
