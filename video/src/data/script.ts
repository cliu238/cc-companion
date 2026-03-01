// Narration script for cc-companion video
// Each scene has text and approximate duration target

export interface SceneScript {
  id: string;
  title: string;
  narration: string;
  startMs: number; // actual start time from audio transcription
}

// Scene timestamps derived from actual Whisper transcription of the generated audio.
// These ensure visuals are perfectly synced with narration.
export const scenes: SceneScript[] = [
  {
    id: "intro",
    title: "Introduction",
    narration:
      "Meet cc-companion — a research platform for exploring new human-AI interaction paradigms, built around Claude Code.",
    startMs: 0,
  },
  {
    id: "vision",
    title: "Vision",
    narration:
      "Today's AI-assisted development is reactive. The human asks, the AI answers. " +
      "cc-companion explores a different approach: a proactive AI companion that has context, " +
      "knows when resources are available, and can take initiative. " +
      "It's built on three pillars. " +
      "Context — understanding what you're working on by indexing session logs and project structures. " +
      "Resources — monitoring API usage windows to identify when compute is available. " +
      "And Agency — moving beyond commands to schedule tasks, generate insights, and surface recommendations.",
    startMs: 9880,
  },
  {
    id: "demo-welcome",
    title: "Project Selector",
    narration:
      "When you launch cc-companion, you're greeted with a project selector. " +
      "It discovers all your Claude Code projects, sorted by recent activity. " +
      "Select a project, and the companion sets the working directory and generates an overview automatically.",
    startMs: 46920,
  },
  {
    id: "demo-chat",
    title: "Advisor Chat",
    narration:
      "The Advisor Chat is an interactive mode powered by Claude. " +
      "But the Advisor doesn't execute code — it's designed as a strategist. " +
      "It helps you think through decisions, draft instructions, and spot blind spots. " +
      "You bridge the Advisor and your execution agent, maintaining control while benefiting from dual perspectives.",
    startMs: 62740,
  },
  {
    id: "demo-logs",
    title: "Session Browser",
    narration:
      "The session browser lets you explore all past Claude Code conversations. " +
      "View metadata like dates, message counts, and git branches. " +
      "Search across session content with ripgrep integration, and review tool-call annotations inline.",
    startMs: 82270,
  },
  {
    id: "workflow",
    title: "Workflow",
    narration:
      "A typical workflow goes like this: launch, select a project, consult the Advisor, " +
      "execute with Claude Code, monitor your API usage, and review past sessions. " +
      "The resource monitor tracks your five-hour and seven-day usage windows in real time, " +
      "and the auto-scheduler dispatches tasks when surplus quota is available.",
    startMs: 97020,
  },
];

// Full narration text for TTS generation
export const fullNarration = scenes.map((s) => s.narration).join("\n\n");

// Audio ends at 125120ms (last word "collaboration")
export const audioDurationMs = 127120; // +2s padding

export const FPS = 30;
