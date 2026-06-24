const MAW_JS_REF_DIR = process.env.MAW_JS_REF_DIR ?? "/home/agent/github.com/Soul-Brews-Studio/maw-js";

export default async function handle(ctx: { source: string; args: string[]; writer?: (...args: unknown[]) => void }) {
  const real = await import(`${MAW_JS_REF_DIR}/src/vendor/mpr-plugins/learn/index.ts`);
  return real.default(ctx);
}
