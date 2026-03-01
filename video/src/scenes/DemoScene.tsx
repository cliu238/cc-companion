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
  image: string;
  label: string;
  image2?: string;  // optional second image, cross-fades at midpoint
  label2?: string;  // optional label for second image
}

export const DemoScene: React.FC<DemoSceneProps> = ({
  image,
  label,
  image2,
  label2,
}) => {
  const frame = useCurrentFrame();
  const { fps, durationInFrames } = useVideoConfig();

  const hasTwoImages = !!image2;
  const midpoint = Math.round(durationInFrames * 0.55);

  // --- Image 1 ---
  const img1Opacity = hasTwoImages
    ? interpolate(frame, [0, fps * 0.5, midpoint - fps * 0.5, midpoint], [0, 1, 1, 0], {
        extrapolateLeft: "clamp",
        extrapolateRight: "clamp",
      })
    : interpolate(frame, [0, fps * 0.5], [0, 1], { extrapolateRight: "clamp" });

  const img1Scale = interpolate(
    frame,
    [0, hasTwoImages ? midpoint : durationInFrames],
    [1.0, 1.06],
    { extrapolateRight: "clamp", easing: Easing.inOut(Easing.ease) }
  );

  const img1TranslateX = interpolate(
    frame,
    [0, hasTwoImages ? midpoint : durationInFrames],
    [0, -12],
    { extrapolateRight: "clamp" }
  );

  // --- Image 2 (only if provided) ---
  const img2Opacity = hasTwoImages
    ? interpolate(frame, [midpoint - fps * 0.3, midpoint + fps * 0.3], [0, 1], {
        extrapolateLeft: "clamp",
        extrapolateRight: "clamp",
      })
    : 0;

  const img2Scale = hasTwoImages
    ? interpolate(frame, [midpoint, durationInFrames], [1.0, 1.06], {
        extrapolateLeft: "clamp",
        extrapolateRight: "clamp",
        easing: Easing.inOut(Easing.ease),
      })
    : 1;

  const img2TranslateX = hasTwoImages
    ? interpolate(frame, [midpoint, durationInFrames], [0, 12], {
        extrapolateLeft: "clamp",
        extrapolateRight: "clamp",
      })
    : 0;

  // --- Label ---
  const currentLabel = hasTwoImages && frame >= midpoint ? (label2 || label) : label;
  const labelX = interpolate(frame, [fps * 0.3, fps * 0.8], [-200, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
    easing: Easing.out(Easing.cubic),
  });
  const labelOpacity = interpolate(frame, [fps * 0.3, fps * 0.8], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  // Re-animate label on image switch
  const label2Opacity = hasTwoImages
    ? interpolate(frame, [midpoint, midpoint + fps * 0.4], [0, 1], {
        extrapolateLeft: "clamp",
        extrapolateRight: "clamp",
      })
    : 1;

  const imgStyle: React.CSSProperties = {
    maxWidth: "100%",
    maxHeight: "100%",
    borderRadius: 12,
    boxShadow: "0 20px 60px rgba(0,0,0,0.5)",
  };

  const containerStyle = (
    opacity: number,
    scale: number,
    tx: number
  ): React.CSSProperties => ({
    position: "absolute",
    inset: 0,
    opacity,
    transform: `scale(${scale}) translateX(${tx}px)`,
    display: "flex",
    justifyContent: "center",
    alignItems: "center",
    padding: 40,
  });

  return (
    <AbsoluteFill style={{ background: "#0f0f23" }}>
      {/* Image 1 */}
      <div style={containerStyle(img1Opacity, img1Scale, img1TranslateX)}>
        <Img src={staticFile(`images/${image}`)} style={imgStyle} />
      </div>

      {/* Image 2 */}
      {hasTwoImages && (
        <div style={containerStyle(img2Opacity, img2Scale, img2TranslateX)}>
          <Img src={staticFile(`images/${image2}`)} style={imgStyle} />
        </div>
      )}

      {/* Feature label */}
      <div
        style={{
          position: "absolute",
          top: 40,
          left: 60,
          opacity: hasTwoImages && frame >= midpoint - fps * 0.3
            ? label2Opacity
            : labelOpacity,
          transform: hasTwoImages && frame >= midpoint - fps * 0.3
            ? undefined
            : `translateX(${labelX}px)`,
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
          {currentLabel}
        </div>
      </div>
    </AbsoluteFill>
  );
};
