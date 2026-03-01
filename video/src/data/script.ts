// Narration script for cc-companion video
// startMs values are placeholders — replaced after TTS + Whisper pipeline

export interface SceneScript {
  id: string;
  title: string;
  narration: string;
  startMs: number;
}

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
    startMs: 8480,
  },
  {
    id: "demo-welcome",
    title: "Project Selector",
    narration:
      "When you launch cc-companion, you're greeted with a project selector. " +
      "It discovers all your Claude Code projects from your home directory, sorted by recent activity. " +
      "Select a project, and the companion sets the working directory and automatically generates a project overview.",
    startMs: 43660,
  },
  {
    id: "demo-chat",
    title: "Advisor Chat",
    narration:
      "The Advisor Chat is an interactive mode powered by the Claude CLI. " +
      "The Advisor is designed as a strategist — it doesn't execute code directly. " +
      "Instead, it helps you think through decisions, draft instructions for your execution agent, and spot blind spots. " +
      "You bridge the Advisor and Claude Code, maintaining control while benefiting from dual perspectives.",
    startMs: 60000,
  },
  {
    id: "status-bar",
    title: "Status Bar",
    narration:
      "The status bar gives you real-time visibility into your resources. " +
      "On the left, you see the five-hour usage window — how much time remains and what percentage you've used. " +
      "Next is the seven-day window with its own percentage and reset countdown. " +
      "You can also see the current chat tone, gateway status, and whether the auto-scheduler is active. " +
      "The bottom bar shows all available keybindings for the current mode — " +
      "press i to type, x for quick tasks, t to change tone, and Shift+Tab to switch to the log viewer.",
    startMs: 82120,
  },
  {
    id: "demo-logs",
    title: "Session Browser",
    narration:
      "The session browser lets you explore all past Claude Code conversations. " +
      "View metadata like dates, message counts, and git branches. " +
      "Drill into any session to read the full conversation with inline tool-call annotations. " +
      "You can also search across all session content using built-in ripgrep integration.",
    startMs: 112000,
  },
  {
    id: "features",
    title: "More Features",
    narration:
      "Beyond browsing and advising, cc-companion includes real-time resource monitoring — " +
      "continuously tracking Anthropic API usage across both the five-hour and seven-day windows, " +
      "refreshing every sixty seconds using OAuth tokens from the platform keychain. " +
      "The auto-task scheduler uses this data to dispatch queued tasks when usage is low and reset windows are approaching, " +
      "turning otherwise wasted quota into productive work. " +
      "You can also run background shell commands without leaving the interface, " +
      "and route requests through a LiteLLM proxy for team environments.",
    startMs: 130980,
  },
  {
    id: "in-development",
    title: "In Development",
    narration:
      "What's next? With context and resource awareness already in place, " +
      "the next step is enabling autonomous, long-running tasks that execute without human intervention. " +
      "These tasks — too time-consuming for interactive sessions — are queued and dispatched when the scheduler detects available capacity. " +
      "An end-to-end scientific research pipeline is being built as the first use case: " +
      "automated literature review, experiment design, and result analysis that runs across multiple sessions.",
    startMs: 165080,
  },
  {
    id: "roadmap",
    title: "Roadmap",
    narration:
      "Looking further ahead, cc-companion will move beyond answering questions " +
      "to actively initiating conversations — offering suggestions on Claude Code usage patterns, " +
      "surfacing relevant past sessions, and providing teaching-style guidance. " +
      "And with cross-user knowledge sharing, the companion can analyze session logs across team members " +
      "to identify knowledge gaps and overlaps, enabling collaborative learning without requiring direct communication.",
    startMs: 193540,
  },
];

// Full narration text for TTS generation
export const fullNarration = scenes.map((s) => s.narration).join("\n\n");

// Placeholder — updated after audio generation
export let audioDurationMs = 217540;

export const FPS = 30;
