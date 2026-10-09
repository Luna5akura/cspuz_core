// E2E test for the balance loop solver in the wasm backend.
const path = require("path");
const ModuleFactory = require("/home/luna/Projects/VSCode/Webstorm/puzzle/pzprjs/dist/wasm/cspuz_solver_backend.js");
const Module = ModuleFactory.default || ModuleFactory;

const WASM_PATH = path.resolve(
	"/home/luna/Projects/VSCode/Webstorm/puzzle/pzprjs/dist/wasm",
	"cspuz_solver_backend.wasm"
);

function solve(mod, url) {
	const buf = mod._prepare_input_buffer(url.length);
	const bytes = Buffer.from(url, "utf8");
	mod.HEAPU8.set(bytes, buf);
	const outptr = mod._solve_problem(buf, url.length);
	const len =
		mod.HEAPU8[outptr] |
		(mod.HEAPU8[outptr + 1] << 8) |
		(mod.HEAPU8[outptr + 2] << 16) |
		(mod.HEAPU8[outptr + 3] << 24);
	const jsonBytes = [];
	for (let i = 0; i < len; i++) {
		jsonBytes.push(mod.HEAPU8[outptr + 4 + i]);
	}
	return JSON.parse(Buffer.from(jsonBytes).toString("utf8"));
}

Module({
	locateFile: function (f) {
		return f.endsWith(".wasm") ? WASM_PATH : f;
	},
}).then((mod) => {

// 3x3 外周ループ
{
	const r = solve(mod, "https://puzz.link/p?balance/3/3/84l8");
	console.log("3x3:", JSON.stringify(r));
	if (r.status !== "ok") {
		console.error("FAIL 3x3");
		process.exit(1);
	}
	const lines = (r.description.data || []).filter(
		(d) => d.color === "green" && (d.item === "line" || d.item.kind === "line")
	).length;
	if (lines !== 8) {
		console.error("FAIL 3x3: expected 8 green lines, got", lines);
		process.exit(1);
	}
}

// 4x4 loop_full ヘビ型
{
	const r = solve(mod, "https://puzz.link/p?balance/f/4/4/9hcn9hc");
	console.log("4x4f:", JSON.stringify(r).slice(0, 300));
	if (r.status !== "ok") {
		console.error("FAIL 4x4f");
		process.exit(1);
	}
	const lines = (r.description.data || []).filter(
		(d) => d.color === "green" && (d.item === "line" || d.item.kind === "line")
	).length;
	if (lines !== 16) {
		console.error("FAIL 4x4f: expected 16 green lines, got", lines);
		process.exit(1);
	}
	// 手がかりマーカー (白丸/黒丸/数字) も返ること
	const circles = (r.description.data || []).filter(
		(d) => d.item === "circle" || d.item === "filledCircle"
	).length;
	if (circles !== 4) {
		console.error("FAIL 4x4f: expected 4 circle items, got", circles);
		process.exit(1);
	}
}

// 2x2: (0,0) 白? のみ → ? マスも必ずループ上、唯一解は 4 マスの小ループ
{
	const r = solve(mod, "https://puzz.link/p?balance/2/2/0i");
	console.log("2x2:", JSON.stringify(r).slice(0, 200));
	if (r.status !== "ok") {
		console.error("FAIL 2x2");
		process.exit(1);
	}
	if (r.description.isUnique !== true) {
		console.error("FAIL 2x2: expected unique");
		process.exit(1);
	}
	const lines = (r.description.data || []).filter(
		(d) => d.color === "green" && (d.item === "line" || d.item.kind === "line")
	).length;
	if (lines !== 4) {
		console.error("FAIL 2x2: expected 4 green lines, got", lines);
		process.exit(1);
	}
}

console.log("E2E OK");
});
