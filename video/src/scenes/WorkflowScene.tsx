import React from "react";
import {
  AbsoluteFill,
  useCurrentFrame,
  useVideoConfig,
  interpolate,
  Easing,
} from "remotion";

const STEPS = [
  { num: "1", label: "Launch", desc: "Start cc-companion" },
  { num: "2", label: "Select", desc: "Pick a project" },
  { num: "3", label: "Consult", desc: "Chat with Advisor" },
  { num: "4", label: "Execute", desc: "Run with Claude Code" },
  { num: "5", label: "Monitor", desc: "Track API usage" },
  { num: "6", label: "Review", desc: "Browse session logs" },
];

export const WorkflowScene: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();

  const headerOpacity = interpolate(frame, [0, fps * 0.4], [0, 1], {
    extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill
      style={{
        background: "linear-gradient(180deg, #0f0f23 0%, #1a1a2e 100%)",
        padding: "70px 100px",
        display: "flex",
        flexDirection: "column",
      }}
    >
      <div style={{ opacity: headerOpacity, marginBottom: 50 }}>
        <h2
          style={{
            fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
            fontSize: 52,
            fontWeight: 700,
            color: "#ffffff",
            margin: 0,
          }}
        >
          Typical Workflow
        </h2>
        <div
          style={{
            width: 80,
            height: 4,
            background: "#FFD700",
            marginTop: 14,
            borderRadius: 2,
          }}
        />
      </div>

      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          gap: 30,
          flex: 1,
          alignContent: "center",
        }}
      >
        {STEPS.map((step, i) => {
          const delay = fps * (0.5 + i * 0.4);
          const opacity = interpolate(frame, [delay, delay + fps * 0.3], [0, 1], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
          });
          const scale = interpolate(frame, [delay, delay + fps * 0.3], [0.8, 1], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
            easing: Easing.out(Easing.back(1.5)),
          });
          const isActive = frame >= delay && frame < delay + fps * 3;

          return (
            <div
              key={step.num}
              style={{
                width: "calc(33.33% - 20px)",
                opacity,
                transform: `scale(${scale})`,
                background: isActive
                  ? "rgba(255, 215, 0, 0.08)"
                  : "rgba(255,255,255,0.03)",
                border: isActive
                  ? "1px solid rgba(255, 215, 0, 0.3)"
                  : "1px solid rgba(255,255,255,0.06)",
                borderRadius: 16,
                padding: "30px 24px",
                display: "flex",
                alignItems: "center",
                gap: 20,
              }}
            >
              <div
                style={{
                  width: 56,
                  height: 56,
                  borderRadius: 28,
                  background: isActive ? "#FFD700" : "rgba(255,255,255,0.1)",
                  color: isActive ? "#0f0f23" : "#ffffff",
                  display: "flex",
                  justifyContent: "center",
                  alignItems: "center",
                  fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
                  fontSize: 24,
                  fontWeight: 800,
                  flexShrink: 0,
                }}
              >
                {step.num}
              </div>
              <div>
                <div
                  style={{
                    fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
                    fontSize: 30,
                    fontWeight: 700,
                    color: "#ffffff",
                  }}
                >
                  {step.label}
                </div>
                <div
                  style={{
                    fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
                    fontSize: 22,
                    color: "rgba(255,255,255,0.45)",
                    marginTop: 4,
                  }}
                >
                  {step.desc}
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </AbsoluteFill>
  );
};
