import React from "react";
import {
  AbsoluteFill,
  Img,
  staticFile,
  useCurrentFrame,
  useVideoConfig,
  interpolate,
  Easing,
} from "remotion";

const FONT =
  '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif';

// Annotated callouts pointing to parts of the status bar
const CALLOUTS = [
  { label: "5h Window", desc: "3h44m left, 14% used", x: 15, y: -90, delay: 0.8 },
  { label: "7d Window", desc: "27% used, 4d06h reset", x: 34, y: -90, delay: 2.5 },
  { label: "Tone", desc: "advisor / eric / chinese", x: 62, y: -90, delay: 4.5 },
  { label: "Auto-Scheduler", desc: "ON/OFF toggle", x: 78, y: -90, delay: 6.0 },
];

const KEYBINDINGS = [
  { key: "i", action: "type message" },
  { key: "x", action: "quick task" },
  { key: "t", action: "change tone" },
  { key: "a", action: "auto-scheduler" },
  { key: "Shift+Tab", action: "log viewer" },
];

export const StatusBarScene: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps, durationInFrames } = useVideoConfig();

  const headerOpacity = interpolate(frame, [0, fps * 0.5], [0, 1], {
    extrapolateRight: "clamp",
  });

  // Status bar image fade in
  const imgOpacity = interpolate(frame, [fps * 0.3, fps * 0.8], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  // Keybindings section appears later
  const keybindingsStart = fps * 10;
  const kbOpacity = interpolate(
    frame,
    [keybindingsStart, keybindingsStart + fps * 0.5],
    [0, 1],
    { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
  );

  return (
    <AbsoluteFill
      style={{
        background: "linear-gradient(180deg, #0f0f23 0%, #1a1a2e 100%)",
        padding: "50px 80px",
        display: "flex",
        flexDirection: "column",
      }}
    >
      {/* Header */}
      <div style={{ opacity: headerOpacity, marginBottom: 30 }}>
        <h2
          style={{
            fontFamily: FONT,
            fontSize: 48,
            fontWeight: 700,
            color: "#ffffff",
            margin: 0,
          }}
        >
          Status Bar
        </h2>
        <div
          style={{
            width: 80,
            height: 4,
            background: "#FFD700",
            marginTop: 12,
            borderRadius: 2,
          }}
        />
      </div>

      {/* Status bar screenshot — enlarged */}
      <div
        style={{
          opacity: imgOpacity,
          position: "relative",
          display: "flex",
          justifyContent: "center",
          marginBottom: 30,
        }}
      >
        <div style={{ position: "relative", width: "100%" }}>
          <Img
            src={staticFile("images/status-bar.png")}
            style={{
              width: "100%",
              borderRadius: 8,
              border: "1px solid rgba(255,255,255,0.1)",
              boxShadow: "0 10px 40px rgba(0,0,0,0.4)",
            }}
          />

          {/* Animated callout annotations */}
          {CALLOUTS.map((c, i) => {
            const calloutDelay = fps * c.delay;
            const calloutOpacity = interpolate(
              frame,
              [calloutDelay, calloutDelay + fps * 0.4],
              [0, 1],
              { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
            );
            const calloutY = interpolate(
              frame,
              [calloutDelay, calloutDelay + fps * 0.4],
              [10, 0],
              {
                extrapolateLeft: "clamp",
                extrapolateRight: "clamp",
                easing: Easing.out(Easing.cubic),
              }
            );

            return (
              <div
                key={c.label}
                style={{
                  position: "absolute",
                  left: `${c.x}%`,
                  top: `${c.y}%`,
                  opacity: calloutOpacity,
                  transform: `translateY(${calloutY}px) translateX(-50%)`,
                  textAlign: "center",
                }}
              >
                <div
                  style={{
                    background: "rgba(255, 215, 0, 0.15)",
                    border: "1px solid rgba(255, 215, 0, 0.4)",
                    borderRadius: 10,
                    padding: "8px 16px",
                    whiteSpace: "nowrap",
                  }}
                >
                  <div
                    style={{
                      fontFamily: FONT,
                      fontSize: 20,
                      fontWeight: 700,
                      color: "#FFD700",
                    }}
                  >
                    {c.label}
                  </div>
                  <div
                    style={{
                      fontFamily: FONT,
                      fontSize: 16,
                      color: "rgba(255,255,255,0.6)",
                      marginTop: 2,
                    }}
                  >
                    {c.desc}
                  </div>
                </div>
                {/* Arrow pointing down */}
                <div
                  style={{
                    width: 0,
                    height: 0,
                    borderLeft: "8px solid transparent",
                    borderRight: "8px solid transparent",
                    borderTop: "8px solid rgba(255, 215, 0, 0.4)",
                    margin: "0 auto",
                  }}
                />
              </div>
            );
          })}
        </div>
      </div>

      {/* Keybindings section */}
      <div style={{ opacity: kbOpacity, flex: 1 }}>
        <h3
          style={{
            fontFamily: FONT,
            fontSize: 32,
            fontWeight: 600,
            color: "#ffffff",
            margin: "0 0 20px",
          }}
        >
          Key Bindings
        </h3>
        <div style={{ display: "flex", gap: 20, flexWrap: "wrap" }}>
          {KEYBINDINGS.map((kb, i) => {
            const kbDelay = keybindingsStart + fps * (0.3 + i * 0.3);
            const itemOpacity = interpolate(
              frame,
              [kbDelay, kbDelay + fps * 0.3],
              [0, 1],
              { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
            );
            return (
              <div
                key={kb.key}
                style={{
                  opacity: itemOpacity,
                  display: "flex",
                  alignItems: "center",
                  gap: 10,
                  background: "rgba(255,255,255,0.04)",
                  border: "1px solid rgba(255,255,255,0.08)",
                  borderRadius: 10,
                  padding: "10px 18px",
                }}
              >
                <kbd
                  style={{
                    fontFamily: '"SF Mono", "Fira Code", monospace',
                    fontSize: 22,
                    fontWeight: 700,
                    color: "#FFD700",
                    background: "rgba(255,215,0,0.1)",
                    padding: "4px 10px",
                    borderRadius: 6,
                    border: "1px solid rgba(255,215,0,0.2)",
                  }}
                >
                  {kb.key}
                </kbd>
                <span
                  style={{
                    fontFamily: FONT,
                    fontSize: 20,
                    color: "rgba(255,255,255,0.6)",
                  }}
                >
                  {kb.action}
                </span>
              </div>
            );
          })}
        </div>
      </div>
    </AbsoluteFill>
  );
};
