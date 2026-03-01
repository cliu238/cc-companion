/**
 * Audio generation pipeline:
 * 1. Read narration script
 * 2. Generate speech with OpenAI TTS
 * 3. Transcribe with OpenAI Whisper API for word-level timestamps
 * 4. Convert to @remotion/captions format and save
 */

import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";
import { config } from "dotenv";
import OpenAI from "openai";
import { createTikTokStyleCaptions } from "@remotion/captions";
import type { Caption } from "@remotion/captions";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(__dirname, "..");
const PROJECT_ROOT = path.resolve(ROOT, "..");

// Load .env from project root
config({ path: path.join(PROJECT_ROOT, ".env") });

if (!process.env.OPENAI_API_KEY) {
  console.error("ERROR: OPENAI_API_KEY not found in .env");
  process.exit(1);
}

const openai = new OpenAI({ apiKey: process.env.OPENAI_API_KEY });

const AUDIO_DIR = path.join(ROOT, "public", "audio");
const NARRATION_MP3 = path.join(AUDIO_DIR, "narration.mp3");
const CAPTIONS_PUBLIC = path.join(ROOT, "public", "captions.json");
const CAPTIONS_SRC = path.join(ROOT, "src", "data", "captions.json");

// Import the script
const { fullNarration } = await import("../src/data/script.ts");

async function generateTTS() {
  console.log("→ Generating narration with OpenAI TTS...");
  console.log(`  Script length: ${fullNarration.length} characters`);

  fs.mkdirSync(AUDIO_DIR, { recursive: true });

  const response = await openai.audio.speech.create({
    model: "gpt-4o-mini-tts",
    voice: "coral",
    input: fullNarration,
    response_format: "mp3",
    instructions:
      "Speak clearly and warmly, like a tech product demo. " +
      "Moderate pace, natural pauses between paragraphs. " +
      "Emphasize key terms like 'context', 'resources', 'agency'.",
  });

  const buffer = Buffer.from(await response.arrayBuffer());
  fs.writeFileSync(NARRATION_MP3, buffer);
  console.log(`  ✓ Saved: ${NARRATION_MP3} (${(buffer.length / 1024).toFixed(0)} KB)`);
}

async function transcribeWithTimestamps(): Promise<Caption[]> {
  console.log("→ Transcribing with OpenAI Whisper for word timestamps...");

  const transcription = await openai.audio.transcriptions.create({
    file: fs.createReadStream(NARRATION_MP3),
    model: "whisper-1",
    response_format: "verbose_json",
    timestamp_granularities: ["word"],
  });

  const words = (transcription as any).words as Array<{
    word: string;
    start: number;
    end: number;
  }>;

  console.log(`  ✓ Got ${words.length} word timestamps`);

  // Convert to @remotion/captions Caption format
  const captions: Caption[] = words.map((w, i) => ({
    text: i === 0 ? w.word : " " + w.word,
    startMs: Math.round(w.start * 1000),
    endMs: Math.round(w.end * 1000),
    timestampMs: Math.round(((w.start + w.end) / 2) * 1000),
    confidence: null,
  }));

  return captions;
}

async function main() {
  console.log("=== cc-companion Video Audio Pipeline ===\n");

  // Step 1: Generate TTS audio
  await generateTTS();

  // Step 2: Transcribe for word timestamps
  const captions = await transcribeWithTimestamps();

  // Step 3: Generate TikTok-style pages
  const { pages } = createTikTokStyleCaptions({
    captions,
    combineTokensWithinMilliseconds: 800,
  });

  // Step 4: Save captions to both public/ and src/data/
  const data = { captions, pages };
  fs.writeFileSync(CAPTIONS_PUBLIC, JSON.stringify(data, null, 2));
  fs.writeFileSync(CAPTIONS_SRC, JSON.stringify(data, null, 2));

  const lastCaption = captions[captions.length - 1];
  const durationSec = (lastCaption.endMs / 1000).toFixed(1);

  console.log(`\n  ✓ Saved captions: ${captions.length} words, ${pages.length} pages`);
  console.log(`  ✓ Audio duration: ~${durationSec}s`);
  console.log(`\n=== Done! Run 'npm run dev' to preview ===`);
}

main().catch((err) => {
  console.error("Pipeline failed:", err);
  process.exit(1);
});
