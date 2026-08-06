// Self-scrolling text for the cover info line.
//
// The bandeau under the cover has less room for text now that the delete/
// extract buttons sit next to the role picker — rather than an ellipsis that
// permanently hides whatever doesn't fit, this scrolls the full line past
// when it overflows, and always exposes it in one shot via the native
// `title` tooltip on hover.

import { useEffect, useRef, useState } from "react";

export interface MarqueeTextProps {
  text: string;
  className?: string;
}

export function MarqueeText({ text, className }: MarqueeTextProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const itemRef = useRef<HTMLSpanElement>(null);
  const [overflowing, setOverflowing] = useState(false);

  useEffect(() => {
    const container = containerRef.current;
    const item = itemRef.current;
    if (!container || !item) return;
    // A ResizeObserver rather than a one-off measurement: the panel itself
    // can resize (window resize), and text that fit before might not anymore.
    const measure = () => setOverflowing(item.scrollWidth > container.clientWidth);
    measure();
    const ro = new ResizeObserver(measure);
    ro.observe(container);
    return () => ro.disconnect();
  }, [text]);

  return (
    <div className={className ? `tag-marquee ${className}` : "tag-marquee"} ref={containerRef} title={text}>
      <div className={overflowing ? "tag-marquee-track" : "tag-marquee-track tag-marquee-static"}>
        <span className="tag-marquee-item" ref={itemRef}>
          {text}
        </span>
        {/* An identical second copy is what makes the loop seamless: the
            track animates by exactly one item's width, so the moment it
            "resets" the two copies are pixel-for-pixel aligned. */}
        {overflowing && (
          <span className="tag-marquee-item" aria-hidden="true">
            {text}
          </span>
        )}
      </div>
    </div>
  );
}
