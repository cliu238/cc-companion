import React from "react";
import {
  AbsoluteFill,
  useCurrentFrame,
  useVideoConfig,
  interpolate,
  Easing,
} from "remotion";

const ASCII_LOGO = `
 ██████╗ ██████╗
██╔════╝██╔════╝
██║     ██║
██║     ██║
╚██████╗╚██████╗
 ╚═════╝ ╚═════╝
`.trim();

export const IntroScene: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();

  const titleOpacity = interpolate(frame, [0, fps * 0.8], [0, 1], {
    extrapolateRight: "clamp",
  });

  const titleY = interpolate(frame, [0, fps * 0.8], [30, 0], {
    extrapolateRight: "clamp",
    easing: Easing.out(Easing.cubic),
  });

  const subtitleOpacity = interpolate(frame, [fps * 0.6, fps * 1.4], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  const logoOpacity = interpolate(frame, [fps * 0.3, fps * 1.0], [0, 0.15], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill
      style={{
        background: "linear-gradient(135deg, #0f0f23 0%, #1a1a3e 50%, #0f0f23 100%)",
        justifyContent: "center",
        alignItems: "center",
      }}
    >
      {/* Background ASCII art */}
      <pre
        style={{
          position: "absolute",
          fontFamily: "monospace",
          fontSize: 48,
          color: "#7c7cff",
          opacity: logoOpacity,
          whiteSpace: "pre",
        }}
      >
        {ASCII_LOGO}
      </pre>

      {/* Title */}
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          opacity: titleOpacity,
          transform: `translateY(${titleY}px)`,
          zIndex: 1,
        }}
      >
        <h1
          style={{
            fontFamily:
              '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
            fontSize: 96,
            fontWeight: 800,
            color: "#ffffff",
            margin: 0,
            letterSpacing: -2,
          }}
        >
          cc-companion
        </h1>
        <div
          style={{
            opacity: subtitleOpacity,
            fontSize: 36,
            color: "rgba(255,255,255,0.6)",
            fontFamily:
              '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
            marginTop: 16,
            fontWeight: 400,
          }}
        >
          A proactive AI companion for Claude Code
        </div>
      </div>
    </AbsoluteFill>
  );
};
