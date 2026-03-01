import { Composition, staticFile } from "remotion";
import { Video } from "./Video";
import { FPS } from "./data/script";

// We'll calculate actual duration from audio length after generation.
// Default: 2.5 minutes = 4500 frames at 30fps.
// This gets overridden by calculateMetadata when captions.json exists.

export const Root: React.FC = () => {
  return (
    <Composition
      id="CcCompanionVideo"
      component={Video}
      durationInFrames={4500}
      fps={FPS}
      width={1920}
      height={1080}
      calculateMetadata={async () => {
        try {
          const resp = await fetch(staticFile("captions.json"));
          const data = await resp.json();
          const lastCaption = data.captions[data.captions.length - 1];
          // Add 2 seconds of padding after last word
          const durationMs = lastCaption.endMs + 2000;
          const durationInFrames = Math.ceil((durationMs / 1000) * FPS);
          return { durationInFrames };
        } catch {
          // captions.json not yet generated — use default
          return { durationInFrames: 4500 };
        }
      }}
    />
  );
};
