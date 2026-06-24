const MAW_JS_REF_DIR = process.env.MAW_JS_REF_DIR ?? "/home/agent/github.com/Soul-Brews-Studio/maw-js";

export default async function handle(_ctx: { source: string; args: string[] }) {
  const real = await import(`${MAW_JS_REF_DIR}/src/vendor/mpr-plugins/cross-team-queue/src/index.ts`);
  return { ok: true, output: JSON.stringify(await real.handle()) };
}
