import React from "react";
import {
  AbsoluteFill,
  useCurrentFrame,
  useVideoConfig,
  interpolate,
  Easing,
} from "remotion";

const FONT =
  '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif';

const FEATURES = [
  {
    icon: "📊",
    title: "Real-Time Resource Monitoring",
    desc: "Tracks 5h and 7d API usage windows, refreshing every 60s via OAuth tokens",
    color: "#4FC3F7",
  },
  {
    icon: "⏰",
    title: "Auto-Task Scheduler",
    desc: "Dispatches queued tasks when usage is low and reset windows approach",
    color: "#FFD740",
  },
  {
    icon: "⚙️",
    title: "Background Task Execution",
    desc: "Run shell commands in background threads with status tracking",
    color: "#CE93D8",
  },
  {
    icon: "🌐",
    title: "Gateway Support",
    desc: "Route through LiteLLM proxy for team environments",
    color: "#81C784",
  },
];

export const FeaturesScene: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();

  const headerOpacity = interpolate(frame, [0, fps * 0.5], [0, 1], {
    extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill
      style={{
        background: "linear-gradient(180deg, #0f0f23 0%, #1a1a2e 100%)",
        padding: "60px 100px",
        display: "flex",
        flexDirection: "column",
      }}
    >
      <div style={{ opacity: headerOpacity, marginBottom: 40 }}>
        <h2
          style={{
            fontFamily: FONT,
            fontSize: 48,
            fontWeight: 700,
            color: "#ffffff",
            margin: 0,
          }}
        >
          More Features
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

      <div
        style={{
          display: "flex",
          flexDirection: "column",
          gap: 24,
          flex: 1,
          justifyContent: "center",
        }}
      >
        {FEATURES.map((feat, i) => {
          const delay = fps * (0.8 + i * 1.8);
          const opacity = interpolate(frame, [delay, delay + fps * 0.5], [0, 1], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
          });
          const translateX = interpolate(
            frame,
            [delay, delay + fps * 0.5],
            [-40, 0],
            {
              extrapolateLeft: "clamp",
              extrapolateRight: "clamp",
              easing: Easing.out(Easing.cubic),
            }
          );
          const isActive = frame >= delay && frame < delay + fps * 6;

          return (
            <div
              key={feat.title}
              style={{
                opacity,
                transform: `translateX(${translateX}px)`,
                display: "flex",
                alignItems: "center",
                gap: 24,
                background: isActive
                  ? `${feat.color}0A`
                  : "rgba(255,255,255,0.02)",
                border: `1px solid ${isActive ? feat.color + "33" : "rgba(255,255,255,0.06)"}`,
                borderRadius: 16,
                padding: "24px 32px",
              }}
            >
              <div style={{ fontSize: 44, flexShrink: 0 }}>{feat.icon}</div>
              <div>
                <div
                  style={{
                    fontFamily: FONT,
                    fontSize: 28,
                    fontWeight: 700,
                    color: isActive ? feat.color : "#ffffff",
                    marginBottom: 4,
                  }}
                >
                  {feat.title}
                </div>
                <div
                  style={{
                    fontFamily: FONT,
                    fontSize: 20,
                    color: "rgba(255,255,255,0.45)",
                    lineHeight: 1.4,
                  }}
                >
                  {feat.desc}
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </AbsoluteFill>
  );
};
