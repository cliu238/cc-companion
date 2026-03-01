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

const PILLARS = [
  {
    icon: "🔍",
    title: "Context",
    desc: "Session logs, project structures, CLAUDE.md",
    details: ["Index all Claude Code sessions", "Parse project structures", "Build persistent awareness"],
    color: "#4FC3F7",
  },
  {
    icon: "⚡",
    title: "Resources",
    desc: "API usage windows, reset schedules",
    details: ["Track 5h & 7-day windows", "Monitor reset countdowns", "Identify available capacity"],
    color: "#FFD740",
  },
  {
    icon: "🤖",
    title: "Agency",
    desc: "Task scheduling, insights, recommendations",
    details: ["Auto-dispatch queued tasks", "Generate insights proactively", "Surface recommendations"],
    color: "#CE93D8",
  },
];

// ── Phase 1: Reactive vs Proactive comparison ──
const ComparisonPhase: React.FC<{ frame: number; fps: number }> = ({
  frame,
  fps,
}) => {
  const phaseEnd = fps * 8;
  if (frame > phaseEnd + fps * 1) return null;

  const fadeOut = interpolate(frame, [phaseEnd - fps * 0.5, phaseEnd + fps * 0.5], [1, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  // "Reactive" side
  const reactiveIn = interpolate(frame, [0, fps * 0.6], [0, 1], {
    extrapolateRight: "clamp",
  });
  const reactiveX = interpolate(frame, [0, fps * 0.6], [-80, 0], {
    extrapolateRight: "clamp",
    easing: Easing.out(Easing.cubic),
  });

  // "Proactive" side
  const proactiveIn = interpolate(frame, [fps * 1.5, fps * 2.2], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  const proactiveX = interpolate(frame, [fps * 1.5, fps * 2.2], [80, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
    easing: Easing.out(Easing.cubic),
  });

  // Arrow animation
  const arrowIn = interpolate(frame, [fps * 3, fps * 3.8], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  // Highlight proactive side
  const proactiveGlow = interpolate(frame, [fps * 4, fps * 5], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill
      style={{
        opacity: fadeOut,
        justifyContent: "center",
        alignItems: "center",
        padding: "0 120px",
      }}
    >
      <div style={{ display: "flex", alignItems: "center", gap: 60, width: "100%" }}>
        {/* Reactive */}
        <div
          style={{
            flex: 1,
            opacity: reactiveIn,
            transform: `translateX(${reactiveX}px)`,
            background: "rgba(255,255,255,0.03)",
            border: "1px solid rgba(255,255,255,0.08)",
            borderRadius: 20,
            padding: "50px 40px",
            textAlign: "center",
          }}
        >
          <div style={{ fontSize: 52, marginBottom: 16 }}>💬</div>
          <h3 style={{ fontFamily: FONT, fontSize: 42, fontWeight: 700, color: "rgba(255,255,255,0.5)", margin: "0 0 16px" }}>
            Reactive
          </h3>
          <p style={{ fontFamily: FONT, fontSize: 24, color: "rgba(255,255,255,0.3)", margin: 0, lineHeight: 1.6 }}>
            Human asks → AI answers
          </p>
        </div>

        {/* Arrow */}
        <div
          style={{
            opacity: arrowIn,
            fontSize: 48,
            color: "#FFD700",
            display: "flex",
            flexDirection: "column",
            alignItems: "center",
            gap: 8,
          }}
        >
          <span>→</span>
        </div>

        {/* Proactive */}
        <div
          style={{
            flex: 1,
            opacity: proactiveIn,
            transform: `translateX(${proactiveX}px)`,
            background: interpolate(proactiveGlow, [0, 1], [0, 0.08])
              ? `rgba(255, 215, 0, ${interpolate(proactiveGlow, [0, 1], [0, 0.08])})`
              : "rgba(255,255,255,0.03)",
            border: `1px solid rgba(255, 215, 0, ${interpolate(proactiveGlow, [0, 1], [0.08, 0.4])})`,
            borderRadius: 20,
            padding: "50px 40px",
            textAlign: "center",
            boxShadow: `0 0 ${interpolate(proactiveGlow, [0, 1], [0, 30])}px rgba(255,215,0,${interpolate(proactiveGlow, [0, 1], [0, 0.15])})`,
          }}
        >
          <div style={{ fontSize: 52, marginBottom: 16 }}>🧠</div>
          <h3 style={{ fontFamily: FONT, fontSize: 42, fontWeight: 700, color: "#ffffff", margin: "0 0 16px" }}>
            Proactive
          </h3>
          <p style={{ fontFamily: FONT, fontSize: 24, color: "rgba(255,255,255,0.6)", margin: 0, lineHeight: 1.6 }}>
            Context + Resources → Initiative
          </p>
        </div>
      </div>
    </AbsoluteFill>
  );
};

// ── Phase 2: Three Pillars with detailed reveal ──
const PillarsPhase: React.FC<{ frame: number; fps: number }> = ({
  frame,
  fps,
}) => {
  const phaseStart = fps * 8;
  const localFrame = frame - phaseStart;
  if (localFrame < -fps * 0.5) return null;

  const fadeIn = interpolate(localFrame, [0, fps * 0.8], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  // Header
  const headerY = interpolate(localFrame, [0, fps * 0.6], [30, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
    easing: Easing.out(Easing.cubic),
  });

  return (
    <AbsoluteFill
      style={{
        opacity: fadeIn,
        background: "linear-gradient(180deg, #0f0f23 0%, #1a1a2e 100%)",
        padding: "60px 100px",
        display: "flex",
        flexDirection: "column",
      }}
    >
      {/* Header */}
      <div style={{ transform: `translateY(${headerY}px)`, marginBottom: 40 }}>
        <h2 style={{ fontFamily: FONT, fontSize: 52, fontWeight: 700, color: "#ffffff", margin: 0 }}>
          Three Pillars
        </h2>
        <div style={{ width: 80, height: 4, background: "#FFD700", marginTop: 14, borderRadius: 2 }} />
      </div>

      {/* Pillars */}
      <div style={{ display: "flex", gap: 40, flex: 1 }}>
        {PILLARS.map((pillar, i) => {
          // Stagger each pillar's entrance
          const pillarDelay = fps * (1.5 + i * 2.5);
          const pillarFrame = localFrame - pillarDelay;

          const cardOpacity = interpolate(pillarFrame, [0, fps * 0.5], [0, 1], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
          });
          const cardY = interpolate(pillarFrame, [0, fps * 0.5], [50, 0], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
            easing: Easing.out(Easing.cubic),
          });

          // Card is "active" (highlighted) from its entrance until the next pillar appears
          const isActive = pillarFrame >= 0 && pillarFrame < fps * 7;

          // Detail items stagger
          const detailsStart = fps * 0.8;

          return (
            <div
              key={pillar.title}
              style={{
                flex: 1,
                opacity: cardOpacity,
                transform: `translateY(${cardY}px)`,
                background: isActive
                  ? `rgba(${pillar.color === "#4FC3F7" ? "79,195,247" : pillar.color === "#FFD740" ? "255,215,64" : "206,147,216"},0.06)`
                  : "rgba(255,255,255,0.02)",
                border: `1px solid ${isActive ? pillar.color + "44" : "rgba(255,255,255,0.06)"}`,
                borderRadius: 20,
                padding: "40px 32px",
                display: "flex",
                flexDirection: "column",
                alignItems: "center",
                textAlign: "center",
              }}
            >
              {/* Icon with scale animation */}
              <div
                style={{
                  fontSize: 56,
                  marginBottom: 20,
                  transform: `scale(${interpolate(pillarFrame, [0, fps * 0.3], [0.5, 1], {
                    extrapolateLeft: "clamp",
                    extrapolateRight: "clamp",
                    easing: Easing.out(Easing.back(2)),
                  })})`,
                }}
              >
                {pillar.icon}
              </div>

              {/* Title */}
              <h3 style={{ fontFamily: FONT, fontSize: 38, fontWeight: 700, color: isActive ? pillar.color : "#ffffff", margin: "0 0 12px" }}>
                {pillar.title}
              </h3>

              {/* Short description */}
              <p style={{ fontFamily: FONT, fontSize: 22, color: "rgba(255,255,255,0.45)", margin: "0 0 20px", lineHeight: 1.4 }}>
                {pillar.desc}
              </p>

              {/* Detail items — staggered reveal */}
              <div style={{ width: "100%", display: "flex", flexDirection: "column", gap: 10 }}>
                {pillar.details.map((detail, j) => {
                  const detailDelay = detailsStart + j * fps * 0.4;
                  const detailOpacity = interpolate(pillarFrame, [detailDelay, detailDelay + fps * 0.3], [0, 1], {
                    extrapolateLeft: "clamp",
                    extrapolateRight: "clamp",
                  });
                  const detailX = interpolate(pillarFrame, [detailDelay, detailDelay + fps * 0.3], [20, 0], {
                    extrapolateLeft: "clamp",
                    extrapolateRight: "clamp",
                    easing: Easing.out(Easing.cubic),
                  });

                  return (
                    <div
                      key={j}
                      style={{
                        opacity: detailOpacity,
                        transform: `translateX(${detailX}px)`,
                        display: "flex",
                        alignItems: "center",
                        gap: 10,
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
                          background: pillar.color,
                          flexShrink: 0,
                        }}
                      />
                      <span style={{ fontFamily: FONT, fontSize: 19, color: "rgba(255,255,255,0.6)" }}>
                        {detail}
                      </span>
                    </div>
                  );
                })}
              </div>
            </div>
          );
        })}
      </div>

      {/* Connection line at bottom — animated */}
      <ConnectionFlow localFrame={localFrame} fps={fps} />
    </AbsoluteFill>
  );
};

// ── Animated connection flow between pillars ──
const ConnectionFlow: React.FC<{ localFrame: number; fps: number }> = ({
  localFrame,
  fps,
}) => {
  const flowStart = fps * 20; // appear near end of pillar reveals
  const flowFrame = localFrame - flowStart;
  if (flowFrame < 0) return null;

  const opacity = interpolate(flowFrame, [0, fps * 0.6], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  // Animated progress line
  const progress = interpolate(flowFrame, [fps * 0.3, fps * 2], [0, 100], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
    easing: Easing.inOut(Easing.ease),
  });

  return (
    <div
      style={{
        opacity,
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        padding: "20px 60px 0",
        gap: 16,
      }}
    >
      {PILLARS.map((p, i) => (
        <React.Fragment key={p.title}>
          <div
            style={{
              fontFamily: FONT,
              fontSize: 18,
              fontWeight: 600,
              color: p.color,
              padding: "6px 16px",
              background: `${p.color}15`,
              borderRadius: 20,
              border: `1px solid ${p.color}33`,
            }}
          >
            {p.title}
          </div>
          {i < PILLARS.length - 1 && (
            <div style={{ flex: 1, height: 2, background: "rgba(255,255,255,0.08)", borderRadius: 1, overflow: "hidden" }}>
              <div
                style={{
                  width: `${Math.min(100, Math.max(0, progress - i * 30))}%`,
                  height: "100%",
                  background: `linear-gradient(90deg, ${PILLARS[i].color}, ${PILLARS[i + 1].color})`,
                  borderRadius: 1,
                }}
              />
            </div>
          )}
        </React.Fragment>
      ))}
    </div>
  );
};

// ── Main VisionScene ──
export const VisionScene: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();

  return (
    <AbsoluteFill
      style={{
        background: "linear-gradient(180deg, #0f0f23 0%, #1a1a2e 100%)",
      }}
    >
      <ComparisonPhase frame={frame} fps={fps} />
      <PillarsPhase frame={frame} fps={fps} />
    </AbsoluteFill>
  );
};
