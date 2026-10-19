// `<style module>` through webpack: `$style` must be the class-name map that
// css-loader generated, whichever way css-loader exports it.
import fs from "node:fs";
import { createRequire } from "node:module";
import path from "node:path";
import { test, type TestContext } from "node:test";
import webpack from "webpack";
import "./test/setup.ts";
import vizeWebpack from "./webpack.ts";
import { packageRoot, prepareOutputDir, resolveFixturePath } from "./test/helpers.ts";

const require = createRequire(import.meta.url);
const CSS_LOADER = createRequire(path.join(packageRoot, "..", "rspack", "package.json")).resolve(
  "css-loader",
);

const CSS_LOADER_OPTIONS: Record<string, Record<string, unknown>> = {
  // css-loader 7 default: named exports, default export is the CSS array.
  "named exports": {},
  // css-loader 6 shape: locals on the default export's `locals`.
  "default export locals": { modules: { namedExport: false } },
  "string export": { exportType: "string" },
};

async function cssModulesOf(name: string, rules: webpack.RuleSetRule[]) {
  const outputPath = prepareOutputDir(`webpack-css-modules-${name.replaceAll(" ", "-")}`);
  const entry = path.join(outputPath, "entry.js");
  fs.writeFileSync(
    entry,
    `export { default } from ${JSON.stringify(resolveFixturePath("css-module", "App.vue"))};\n`,
  );
  const compiler = webpack({
    mode: "development",
    devtool: false,
    target: "node",
    entry,
    externals: { vue: `commonjs ${require.resolve("vue")}` },
    output: { path: outputPath, filename: "bundle.cjs", library: { type: "commonjs2" } },
    infrastructureLogging: { level: "error" },
    module: { rules },
    plugins: [vizeWebpack({ isProduction: true })],
    resolve: { extensions: [".ts", ".js", ".vue"] },
  });
  const stats = await new Promise<webpack.Stats>((resolve, reject) => {
    compiler.run((error, result) => {
      compiler.close(() => (error || !result ? reject(error) : resolve(result)));
    });
  });
  if (stats.hasErrors()) {
    throw new Error(JSON.stringify(stats.toJson({ all: false, errors: true }).errors, null, 2));
  }
  const bundlePath = path.join(outputPath, "bundle.cjs");
  const bundle = fs.readFileSync(bundlePath, "utf8");
  const component = require(bundlePath).default as { __cssModules: Record<string, unknown> };
  return { bundle, modules: component.__cssModules };
}

for (const [name, options] of Object.entries(CSS_LOADER_OPTIONS)) {
  void test(`webpack css modules (${name}): $style is css-loader's class map`, async (t: TestContext) => {
    const { bundle, modules } = await cssModulesOf(name, [
      { test: /\.css$/, use: [{ loader: CSS_LOADER, options }] },
    ]);
    const root = (modules.$style as { root?: unknown } | undefined)?.root;
    t.assert.equal(typeof root, "string");
    // The generated class is the one css-loader wrote into the CSS.
    t.assert.match(bundle, new RegExp(`\\.${String(root)} \\{`));
    t.assert.deepEqual(Object.keys(modules.$style as object), ["root"]);
  });
}

// vue-loader hosts tell CSS modules apart by the `?module` resource query and
// often compile every other style with `modules: "global"` (no local names).
void test("webpack css modules: host rules keyed on resourceQuery /module/ apply", async (t: TestContext) => {
  const { bundle, modules } = await cssModulesOf("resource-query", [
    {
      test: /\.css$/,
      oneOf: [
        {
          resourceQuery: /module/,
          use: [
            {
              loader: CSS_LOADER,
              options: { modules: { localIdentName: "[local]_[hash:base64:8]" } },
            },
          ],
        },
        { use: [{ loader: CSS_LOADER, options: { modules: "global" } }] },
      ],
    },
  ]);
  const root = (modules.$style as { root?: unknown } | undefined)?.root;
  t.assert.equal(typeof root, "string");
  t.assert.match(String(root), /^root_/);
  t.assert.match(bundle, new RegExp(`\\.${String(root)} \\{`));
});
