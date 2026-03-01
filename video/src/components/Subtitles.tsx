import React from "react";
import { useCurrentFrame, useVideoConfig, AbsoluteFill } from "remotion";
import type { TikTokPage } from "@remotion/captions";

interface SubtitlesProps {
  pages: TikTokPage[];
}

export const Subtitles: React.FC<SubtitlesProps> = ({ pages }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const currentTimeMs = (frame / fps) * 1000;

  const currentPage = pages.find(
    (p) => currentTimeMs >= p.startMs && currentTimeMs < p.startMs + p.durationMs
  );

  if (!currentPage) return null;

  return (
    <AbsoluteFill
      style={{
        justifyContent: "flex-end",
        alignItems: "center",
        padding: "0 80px 80px 80px",
      }}
    >
      <div
        style={{
          textAlign: "center",
          background: "rgba(0, 0, 0, 0.6)",
          borderRadius: 12,
          padding: "16px 28px",
          maxWidth: 1400,
        }}
      >
        {currentPage.tokens.map((token, i) => {
          const isActive = currentTimeMs >= token.fromMs;
          return (
            <span
              key={i}
              style={{
                whiteSpace: "pre-wrap",
                color: isActive ? "#FFD700" : "rgba(255,255,255,0.7)",
                fontSize: 44,
                fontFamily:
                  '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
                fontWeight: isActive ? 700 : 500,
                textShadow: "1px 2px 4px rgba(0,0,0,0.5)",
              }}
            >
              {token.text}
            </span>
          );
        })}
      </div>
    </AbsoluteFill>
  );
};
