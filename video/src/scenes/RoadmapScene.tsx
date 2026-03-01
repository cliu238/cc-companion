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

interface RoadmapSceneProps {
  variant: "in-development" | "roadmap";
}

const CONTENT = {
  "in-development": {
    header: "In Development",
    items: [
      {
        icon: "🔬",
        title: "Autonomous Task Pipeline",
        points: [
          "Long-running tasks queued and dispatched automatically",
          "Executes when scheduler detects available capacity",
          "No human intervention required",
        ],
      },
      {
        icon: "🧪",
        title: "Scientific Research Pipeline",
        points: [
          "Automated literature review",
          "Experiment design and result analysis",
          "Runs across multiple sessions",
        ],
      },
    ],
  },
  roadmap: {
    header: "Roadmap",
    items: [
      {
        icon: "💡",
        title: "Proactive Heuristic Guidance",
        points: [
          "AI initiates conversations, not just responds",
          "Suggests Claude Code usage patterns",
          "Teaching-style developer coaching",
        ],
      },
      {
        icon: "🤝",
        title: "Cross-User Knowledge Sharing",
        points: [
          "Analyze logs across team members",
          "Identify knowledge gaps and overlaps",
          "Collaborative learning without direct communication",
        ],
      },
    ],
  },
};

export const RoadmapScene: React.FC<RoadmapSceneProps> = ({ variant }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const { header, items } = CONTENT[variant];

  const headerOpacity = interpolate(frame, [0, fps * 0.5], [0, 1], {
    extrapolateRight: "clamp",
  });
  const headerY = interpolate(frame, [0, fps * 0.5], [20, 0], {
    extrapolateRight: "clamp",
    easing: Easing.out(Easing.cubic),
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
      <div
        style={{
          opacity: headerOpacity,
          transform: `translateY(${headerY}px)`,
          marginBottom: 50,
        }}
      >
        <h2
          style={{
            fontFamily: FONT,
            fontSize: 48,
            fontWeight: 700,
            color: "#ffffff",
            margin: 0,
          }}
        >
          {header}
        </h2>
        <div
          style={{
            width: 80,
            height: 4,
            background: variant === "in-development" ? "#FF9800" : "#7C4DFF",
            marginTop: 12,
            borderRadius: 2,
          }}
        />
      </div>

      <div style={{ display: "flex", gap: 50, flex: 1 }}>
        {items.map((item, i) => {
          const delay = fps * (1.0 + i * 3);
          const cardOpacity = interpolate(
            frame,
            [delay, delay + fps * 0.6],
            [0, 1],
            { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
          );
          const cardY = interpolate(
            frame,
            [delay, delay + fps * 0.6],
            [40, 0],
            {
              extrapolateLeft: "clamp",
              extrapolateRight: "clamp",
              easing: Easing.out(Easing.cubic),
            }
          );
          const isActive = frame >= delay && frame < delay + fps * 8;
          const accentColor =
            variant === "in-development" ? "#FF9800" : "#7C4DFF";

          return (
            <div
              key={item.title}
              style={{
                flex: 1,
                opacity: cardOpacity,
                transform: `translateY(${cardY}px)`,
                background: isActive
                  ? `${accentColor}0A`
                  : "rgba(255,255,255,0.02)",
                border: `1px solid ${isActive ? accentColor + "33" : "rgba(255,255,255,0.06)"}`,
                borderRadius: 20,
                padding: "40px 36px",
                display: "flex",
                flexDirection: "column",
              }}
            >
              <div style={{ fontSize: 52, marginBottom: 20 }}>{item.icon}</div>
              <h3
                style={{
                  fontFamily: FONT,
                  fontSize: 32,
                  fontWeight: 700,
                  color: isActive ? accentColor : "#ffffff",
                  margin: "0 0 24px",
                }}
              >
                {item.title}
              </h3>
              <div
                style={{
                  display: "flex",
                  flexDirection: "column",
                  gap: 14,
                }}
              >
                {item.points.map((point, j) => {
                  const pointDelay = delay + fps * (0.8 + j * 0.5);
                  const pointOpacity = interpolate(
                    frame,
                    [pointDelay, pointDelay + fps * 0.3],
                    [0, 1],
                    { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
                  );
                  return (
                    <div
                      key={j}
                      style={{
                        opacity: pointOpacity,
                        display: "flex",
                        alignItems: "center",
                        gap: 12,
                        padding: "8px 12px",
                        background: "rgba(255,255,255,0.03)",
                        borderRadius: 8,
                      }}
                    >
                      <div
                        style={{
                          width: 6,
                          height: 6,
                          borderRadius: 3,
                          background: accentColor,
                          flexShrink: 0,
                        }}
                      />
                      <span
                        style={{
                          fontFamily: FONT,
                          fontSize: 21,
                          color: "rgba(255,255,255,0.6)",
                        }}
                      >
                        {point}
                      </span>
                    </div>
                  );
                })}
              </div>
            </div>
          );
        })}
      </div>
    </AbsoluteFill>
  );
};
