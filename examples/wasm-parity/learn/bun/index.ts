export default async function handle(ctx: { source: string; args: string[] }) {
  const args = ctx.source === "cli" ? ctx.args : [];
  const fast = args.includes("--fast");
  const deep = args.includes("--deep");
  if (fast && deep) return { ok: false, error: "maw learn: --fast and --deep are mutually exclusive" };
  const positional = args.filter(a => !a.startsWith("--"));
  const unknown = args.filter(a => a.startsWith("--") && a !== "--fast" && a !== "--deep");
  if (unknown.length > 0) return { ok: false, error: `maw learn: unknown flag(s) ${unknown.join(", ")} (accepts --fast, --deep)` };
  if (!positional[0]) return { ok: false, error: "usage: maw learn <repo> [--fast|--deep]" };
  const mode = fast ? "fast" : deep ? "deep" : "default";
  const agents = mode === "fast" ? 1 : mode === "deep" ? 5 : 3;
  const message = [
    `learn: ${mode} mode on "${positional[0]}" — not yet implemented in core plugin; use Oracle skill /learn for full behavior.`,
    `  planned: ${agents} parallel agent(s), write docs to ψ/learn/<owner>/<repo>/YYYY-MM-DD/HHMM_*.md`,
    `  track:   https://github.com/Soul-Brews-Studio/maw-js/issues/521`,
  ].join("\n");
  return { ok: true, output: message };
}
