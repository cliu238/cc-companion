import React from "react";
import {
  AbsoluteFill,
  Sequence,
  Audio,
  staticFile,
} from "remotion";
import { createTikTokStyleCaptions } from "@remotion/captions";
import { IntroScene } from "./scenes/IntroScene";
import { VisionScene } from "./scenes/VisionScene";
import { DemoScene } from "./scenes/DemoScene";
import { WorkflowScene } from "./scenes/WorkflowScene";
import { Subtitles } from "./components/Subtitles";
import { scenes, audioDurationMs, FPS } from "./data/script";
import captionsData from "./data/captions.json";

// Build timeline from actual audio timestamps (Whisper transcription)
function buildSceneTimeline() {
  const totalFrames = Math.ceil((audioDurationMs / 1000) * FPS);

  return scenes.map((sc, i) => {
    const fromFrame = Math.round((sc.startMs / 1000) * FPS);
    // Duration = next scene start (or total end) minus this scene start
    const nextStartMs = i < scenes.length - 1 ? scenes[i + 1].startMs : audioDurationMs;
    const durationInFrames = Math.round(((nextStartMs - sc.startMs) / 1000) * FPS);
    return { ...sc, from: fromFrame, durationInFrames };
  });
}

export const Video: React.FC = () => {
  const timeline = buildSceneTimeline();

  // Parse captions
  const captions = (captionsData as { captions: any[] }).captions || [];
  const { pages } = createTikTokStyleCaptions({
    captions,
    combineTokensWithinMilliseconds: 800,
  });

  const sceneMap: Record<string, (from: number, dur: number) => React.ReactNode> = {
    intro: (from, dur) => (
      <Sequence key="intro" from={from} durationInFrames={dur} name="Intro">
        <IntroScene />
      </Sequence>
    ),
    vision: (from, dur) => (
      <Sequence key="vision" from={from} durationInFrames={dur} name="Vision">
        <VisionScene />
      </Sequence>
    ),
    "demo-welcome": (from, dur) => (
      <Sequence key="demo-welcome" from={from} durationInFrames={dur} name="Demo: Welcome">
        <DemoScene image="welcome.png" label="Project Selector" />
      </Sequence>
    ),
    "demo-chat": (from, dur) => (
      <Sequence key="demo-chat" from={from} durationInFrames={dur} name="Demo: Chat">
        <DemoScene image="chat-mode.png" label="Advisor Chat" />
      </Sequence>
    ),
    "demo-logs": (from, dur) => (
      <Sequence key="demo-logs" from={from} durationInFrames={dur} name="Demo: Logs">
        <DemoScene image="log-mode.png" label="Session Browser" />
      </Sequence>
    ),
    workflow: (from, dur) => (
      <Sequence key="workflow" from={from} durationInFrames={dur} name="Workflow">
        <WorkflowScene />
      </Sequence>
    ),
  };

  return (
    <AbsoluteFill style={{ background: "#0f0f23" }}>
      {/* Narration audio — spans the entire video */}
      <Audio src={staticFile("audio/narration.mp3")} />

      {/* Scene sequences */}
      {timeline.map((sc) => {
        const renderer = sceneMap[sc.id];
        return renderer ? renderer(sc.from, sc.durationInFrames) : null;
      })}

      {/* Subtitle overlay — always on top */}
      <Subtitles pages={pages} />
    </AbsoluteFill>
  );
};
