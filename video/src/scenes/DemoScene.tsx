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

interface DemoSceneProps {
  image: string; // filename in public/images/
  label: string;
}

export const DemoScene: React.FC<DemoSceneProps> = ({ image, label }) => {
  const frame = useCurrentFrame();
  const { fps, durationInFrames } = useVideoConfig();

  // Fade in
  const opacity = interpolate(frame, [0, fps * 0.5], [0, 1], {
    extrapolateRight: "clamp",
  });

  // Slow Ken Burns zoom effect
  const scale = interpolate(frame, [0, durationInFrames], [1.0, 1.08], {
    extrapolateRight: "clamp",
    easing: Easing.inOut(Easing.ease),
  });

  // Slight pan
  const translateX = interpolate(frame, [0, durationInFrames], [0, -15], {
    extrapolateRight: "clamp",
  });

  // Label slide in
  const labelX = interpolate(frame, [fps * 0.3, fps * 0.8], [-200, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
    easing: Easing.out(Easing.cubic),
  });

  const labelOpacity = interpolate(frame, [fps * 0.3, fps * 0.8], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill style={{ background: "#0f0f23" }}>
      {/* Screenshot with Ken Burns */}
      <div
        style={{
          opacity,
          transform: `scale(${scale}) translateX(${translateX}px)`,
          width: "100%",
          height: "100%",
          display: "flex",
          justifyContent: "center",
          alignItems: "center",
          padding: 40,
        }}
      >
        <Img
          src={staticFile(`images/${image}`)}
          style={{
            maxWidth: "100%",
            maxHeight: "100%",
            borderRadius: 12,
            boxShadow: "0 20px 60px rgba(0,0,0,0.5)",
          }}
        />
      </div>

      {/* Feature label */}
      <div
        style={{
          position: "absolute",
          top: 40,
          left: 60,
          opacity: labelOpacity,
          transform: `translateX(${labelX}px)`,
        }}
      >
        <div
          style={{
            background: "rgba(255, 215, 0, 0.9)",
            color: "#0f0f23",
            fontFamily:
              '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
            fontSize: 28,
            fontWeight: 700,
            padding: "10px 24px",
            borderRadius: 8,
          }}
        >
          {label}
        </div>
      </div>
    </AbsoluteFill>
  );
};
